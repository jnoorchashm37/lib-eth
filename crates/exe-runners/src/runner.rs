//! A tokio based CLI runner.
//!
//! Entrypoint for running commands.

use crate::{PanickedTaskError, Runtime, RuntimeBuildError, RuntimeBuilder, RuntimeConfig, TaskExecutor};
use std::{future::Future, pin::pin, sync::mpsc, time::Duration};
use tokio::task::JoinHandle;
use tracing::{debug, error, info};

/// Executes CLI commands.
///
/// Provides utilities for running a CLI command to completion.
#[derive(Debug)]
pub struct CliRunner {
    config: CliRunnerConfig,
    runtime: Runtime,
}

impl CliRunner {
    /// Attempts to create a new [`CliRunner`] using the default [`Runtime`].
    pub fn try_default_runtime() -> Result<Self, RuntimeBuildError> {
        Self::try_with_runtime_config(RuntimeConfig::default())
    }

    /// Creates a new [`CliRunner`] with the given [`RuntimeConfig`].
    pub fn try_with_runtime_config(config: RuntimeConfig) -> Result<Self, RuntimeBuildError> {
        let runtime = RuntimeBuilder::new(config).build()?;
        Ok(Self { config: CliRunnerConfig::default(), runtime })
    }

    /// Sets the [`CliRunnerConfig`] for this runner.
    pub const fn with_config(mut self, config: CliRunnerConfig) -> Self {
        self.config = config;
        self
    }

    /// Returns a clone of the underlying [`Runtime`].
    pub fn runtime(&self) -> Runtime {
        self.runtime.clone()
    }

    /// Executes an async block on the runtime and blocks until completion.
    pub fn block_on<F, T>(&self, fut: F) -> T
    where
        F: Future<Output = T>,
    {
        self.runtime.handle().block_on(fut)
    }

    /// Executes the given async command on the tokio runtime until the command
    /// future resolves or until the process receives a `SIGINT` or `SIGTERM`
    /// signal.
    pub fn run_command_until_exit<F, E>(self, command: impl FnOnce(CliContext) -> F) -> Result<(), E>
    where
        F: Future<Output = Result<(), E>>,
        E: Send + Sync + std::fmt::Display + From<std::io::Error> + From<PanickedTaskError> + 'static,
    {
        let (context, task_manager_handle) = cli_context(&self.runtime);

        let command_res = self
            .runtime
            .handle()
            .block_on(run_to_completion_or_panic(task_manager_handle, run_until_ctrl_c(command(context))));

        if let Err(err) = &command_res {
            error!(target: "exe_runners::cli", %err, "shutting down due to error");
        } else {
            debug!(target: "exe_runners::cli", "shutting down gracefully");
            self.runtime
                .graceful_shutdown_with_timeout(self.config.graceful_shutdown_timeout);
        }

        runtime_shutdown(self.runtime, true);

        command_res
    }

    /// Executes a command in a blocking context with access to `CliContext`.
    pub fn run_blocking_command_until_exit<F, E>(
        self,
        command: impl FnOnce(CliContext) -> F + Send + 'static,
    ) -> Result<(), E>
    where
        F: Future<Output = Result<(), E>> + Send + 'static,
        E: Send + Sync + std::fmt::Display + From<std::io::Error> + From<PanickedTaskError> + 'static,
    {
        let (context, task_manager_handle) = cli_context(&self.runtime);

        let handle = self.runtime.handle().clone();
        let handle2 = handle.clone();
        let command_handle = handle.spawn_blocking(move || handle2.block_on(command(context)));

        let command_res = self.runtime.handle().block_on(run_to_completion_or_panic(
            task_manager_handle,
            run_until_ctrl_c(async move { command_handle.await.expect("Failed to join blocking task") }),
        ));

        if let Err(err) = &command_res {
            error!(target: "exe_runners::cli", %err, "shutting down due to error");
        } else {
            debug!(target: "exe_runners::cli", "shutting down gracefully");
            self.runtime
                .graceful_shutdown_with_timeout(self.config.graceful_shutdown_timeout);
        }

        runtime_shutdown(self.runtime, true);

        command_res
    }

    /// Executes a regular future until completion or until external signal
    /// received.
    pub fn run_until_ctrl_c<F, E>(self, fut: F) -> Result<(), E>
    where
        F: Future<Output = Result<(), E>>,
        E: Send + Sync + From<std::io::Error> + 'static,
    {
        self.runtime.handle().block_on(run_until_ctrl_c(fut))?;
        Ok(())
    }

    /// Executes a regular future as a spawned blocking task until completion or
    /// until external signal received.
    pub fn run_blocking_until_ctrl_c<F, E>(self, fut: F) -> Result<(), E>
    where
        F: Future<Output = Result<(), E>> + Send + 'static,
        E: Send + Sync + From<std::io::Error> + 'static,
    {
        let handle = self.runtime.handle().clone();
        let handle2 = handle.clone();
        let fut = handle.spawn_blocking(move || handle2.block_on(fut));
        self.runtime
            .handle()
            .block_on(run_until_ctrl_c(async move { fut.await.expect("Failed to join task") }))?;

        runtime_shutdown(self.runtime, false);

        Ok(())
    }
}

/// Extracts the task manager handle from the runtime and creates the
/// [`CliContext`].
fn cli_context(runtime: &Runtime) -> (CliContext, JoinHandle<Result<(), PanickedTaskError>>) {
    let handle = runtime
        .take_task_manager_handle()
        .expect("Runtime must contain a TaskManager handle");
    let context = CliContext { task_executor: runtime.clone() };
    (context, handle)
}

/// Additional context provided by the [`CliRunner`] when executing commands.
#[derive(Debug)]
pub struct CliContext {
    /// Used to execute/spawn tasks.
    pub task_executor: TaskExecutor,
}

/// Default timeout for graceful shutdown of tasks.
const DEFAULT_GRACEFUL_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

/// Configuration for [`CliRunner`].
#[derive(Debug, Clone)]
pub struct CliRunnerConfig {
    /// Timeout for graceful shutdown of tasks.
    pub graceful_shutdown_timeout: Duration,
}

impl Default for CliRunnerConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl CliRunnerConfig {
    /// Creates a new config with default values.
    pub const fn new() -> Self {
        Self { graceful_shutdown_timeout: DEFAULT_GRACEFUL_SHUTDOWN_TIMEOUT }
    }

    /// Sets the graceful shutdown timeout.
    pub const fn with_graceful_shutdown_timeout(mut self, timeout: Duration) -> Self {
        self.graceful_shutdown_timeout = timeout;
        self
    }
}

/// Runs the given future to completion or until a critical task panicked.
async fn run_to_completion_or_panic<F, E>(
    task_manager_handle: JoinHandle<Result<(), PanickedTaskError>>,
    fut: F,
) -> Result<(), E>
where
    F: Future<Output = Result<(), E>>,
    E: Send + Sync + From<PanickedTaskError> + 'static,
{
    let fut = pin!(fut);
    tokio::select! {
        task_manager_result = task_manager_handle => {
            if let Ok(Err(panicked_error)) = task_manager_result {
                return Err(panicked_error.into());
            }
        },
        res = fut => res?,
    }
    Ok(())
}

/// Runs the future to completion or until:
/// - `ctrl-c` is received.
/// - `SIGTERM` is received (unix only).
async fn run_until_ctrl_c<F, E>(fut: F) -> Result<(), E>
where
    F: Future<Output = Result<(), E>>,
    E: Send + Sync + 'static + From<std::io::Error>,
{
    let ctrl_c = tokio::signal::ctrl_c();

    #[cfg(unix)]
    {
        let mut stream = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        let sigterm = stream.recv();
        let sigterm = pin!(sigterm);
        let ctrl_c = pin!(ctrl_c);
        let fut = pin!(fut);

        tokio::select! {
            _ = ctrl_c => {
                info!(target: "exe_runners::cli", "Received ctrl-c");
            },
            _ = sigterm => {
                info!(target: "exe_runners::cli", "Received SIGTERM");
            },
            res = fut => res?,
        }
    }

    #[cfg(not(unix))]
    {
        let ctrl_c = pin!(ctrl_c);
        let fut = pin!(fut);

        tokio::select! {
            _ = ctrl_c => {
                info!(target: "exe_runners::cli", "Received ctrl-c");
            },
            res = fut => res?,
        }
    }

    Ok(())
}

/// Default timeout for waiting on the runtime to shut down.
const DEFAULT_RUNTIME_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

/// Shut down the given [`Runtime`], and wait for it if `wait` is set.
fn runtime_shutdown(rt: Runtime, wait: bool) {
    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new()
        .name("rt-shutdown".to_string())
        .spawn(move || {
            drop(rt);
            let _ = tx.send(());
        })
        .unwrap();

    if wait {
        let _ = rx
            .recv_timeout(DEFAULT_RUNTIME_SHUTDOWN_TIMEOUT)
            .inspect_err(|err| {
                tracing::warn!(target: "exe_runners::cli", %err, "runtime shutdown timed out");
            });
    }
}
