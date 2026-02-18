//! A tokio based CLI runner.

//! Entrypoint for running commands.

use std::{future::Future, pin::pin, sync::mpsc, time::Duration};

use reth_tasks::{PanickedTaskError, TaskExecutor, TaskManager};
use tracing::{debug, error, info};

/// Executes CLI commands.
///
/// Provides utilities for running a cli command to completion.
#[derive(Debug)]
pub struct CliRunner {
    config:  CliRunnerConfig,
    runtime: tokio::runtime::Runtime
}

impl CliRunner {
    /// Attempts to create a new [`CliRunner`] using a default tokio runtime.
    pub fn try_default_runtime() -> Result<Self, std::io::Error> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        Ok(Self { config: CliRunnerConfig::default(), runtime })
    }

    /// Sets the [`CliRunnerConfig`] for this runner.
    pub const fn with_config(mut self, config: CliRunnerConfig) -> Self {
        self.config = config;
        self
    }

    /// Returns the underlying tokio runtime.
    pub fn runtime(&self) -> &tokio::runtime::Runtime {
        &self.runtime
    }

    /// Executes an async block on the runtime and blocks until completion.
    pub fn block_on<F, T>(&self, fut: F) -> T
    where
        F: Future<Output = T>
    {
        self.runtime.block_on(fut)
    }

    /// Executes the given _async_ command on the tokio runtime until the
    /// command future resolves or until the process receives a `SIGINT` or
    /// `SIGTERM` signal.
    ///
    /// Tasks spawned by the command via the [`TaskExecutor`] are shut down and
    /// an attempt is made to drive their shutdown to completion after the
    /// command has finished.
    pub fn run_command_until_exit<F, E>(self, command: impl FnOnce(CliContext) -> F) -> Result<(), E>
    where
        F: Future<Output = Result<(), E>>,
        E: Send + Sync + From<std::io::Error> + From<PanickedTaskError> + 'static
    {
        let (context, mut task_manager) = cli_context(self.runtime.handle());

        // Executes the command until it finished or ctrl-c was fired
        let command_res = self
            .runtime
            .block_on(run_to_completion_or_panic(&mut task_manager, run_until_ctrl_c(command(context))));

        if command_res.is_err() {
            error!("shutting down due to error");
        } else {
            debug!("shutting down gracefully");
            // after the command has finished or exit signal was received we shutdown the
            // runtime which fires the shutdown signal to all tasks spawned via the task
            // executor and awaiting on tasks spawned with graceful shutdown
            task_manager.graceful_shutdown_with_timeout(self.config.graceful_shutdown_timeout);
        }

        runtime_shutdown(self.runtime, true);

        command_res
    }

    /// Executes a command in a blocking context with access to `CliContext`.
    ///
    /// See [`Runtime::spawn_blocking`](tokio::runtime::Runtime::spawn_blocking).
    pub fn run_blocking_command_until_exit<F, E>(
        self,
        command: impl FnOnce(CliContext) -> F + Send + 'static
    ) -> Result<(), E>
    where
        F: Future<Output = Result<(), E>> + Send + 'static,
        E: Send + Sync + From<std::io::Error> + From<PanickedTaskError> + 'static
    {
        let (context, mut task_manager) = cli_context(self.runtime.handle());

        // Spawn the command on the blocking thread pool
        let handle = self.runtime.handle().clone();
        let handle2 = handle.clone();
        let command_handle = handle.spawn_blocking(move || handle2.block_on(command(context)));

        // Wait for the command to complete or ctrl-c
        let command_res = self.runtime.block_on(run_to_completion_or_panic(
            &mut task_manager,
            run_until_ctrl_c(async move { command_handle.await.expect("Failed to join blocking task") })
        ));

        if command_res.is_err() {
            error!("shutting down due to error");
        } else {
            debug!("shutting down gracefully");
            task_manager.graceful_shutdown_with_timeout(self.config.graceful_shutdown_timeout);
        }

        runtime_shutdown(self.runtime, true);

        command_res
    }

    /// Executes a regular future until completion or until external signal
    /// received.
    pub fn run_until_ctrl_c<F, E>(self, fut: F) -> Result<(), E>
    where
        F: Future<Output = Result<(), E>>,
        E: Send + Sync + From<std::io::Error> + 'static
    {
        self.runtime.block_on(run_until_ctrl_c(fut))?;
        Ok(())
    }

    /// Executes a regular future as a spawned blocking task until completion or
    /// until external signal received.
    ///
    /// See [`Runtime::spawn_blocking`](tokio::runtime::Runtime::spawn_blocking).
    pub fn run_blocking_until_ctrl_c<F, E>(self, fut: F) -> Result<(), E>
    where
        F: Future<Output = Result<(), E>> + Send + 'static,
        E: Send + Sync + From<std::io::Error> + 'static
    {
        let handle = self.runtime.handle().clone();
        let handle2 = handle.clone();
        let fut = handle.spawn_blocking(move || handle2.block_on(fut));
        self.runtime
            .block_on(run_until_ctrl_c(async move { fut.await.expect("Failed to join task") }))?;

        runtime_shutdown(self.runtime, false);

        Ok(())
    }
}

/// Creates a task manager and the corresponding [`CliContext`] for commands.
fn cli_context(runtime: &tokio::runtime::Handle) -> (CliContext, TaskManager) {
    let task_manager = TaskManager::new(runtime.clone());
    let context = CliContext { task_executor: task_manager.executor() };
    (context, task_manager)
}

/// Additional context provided by the [`CliRunner`] when executing commands
#[derive(Debug)]
pub struct CliContext {
    /// Used to execute/spawn tasks
    pub task_executor: TaskExecutor
}

/// Default timeout for graceful shutdown of tasks.
const DEFAULT_GRACEFUL_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

/// Configuration for [`CliRunner`].
#[derive(Debug, Clone)]
pub struct CliRunnerConfig {
    /// Timeout for graceful shutdown of tasks.
    ///
    /// After the command completes, this is the maximum time to wait for
    /// spawned tasks to finish before forcefully terminating them.
    pub graceful_shutdown_timeout: Duration
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
///
/// Returns the error if a task panicked, or the given future returned an error.
async fn run_to_completion_or_panic<F, E>(task_manager: &mut TaskManager, fut: F) -> Result<(), E>
where
    F: Future<Output = Result<(), E>>,
    E: Send + Sync + From<PanickedTaskError> + 'static
{
    let fut = pin!(fut);
    tokio::select! {
        task_manager_result = task_manager => {
            match task_manager_result {
                Ok(()) => return Ok(()),
                Err(panicked_error) => return Err(panicked_error.into())
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
    E: Send + Sync + 'static + From<std::io::Error>
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
                info!("Received ctrl-c");
            },
            _ = sigterm => {
                info!("Received SIGTERM");
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
                info!("Received ctrl-c");
            },
            res = fut => res?,
        }
    }

    Ok(())
}

/// Default timeout for waiting on the tokio runtime to shut down.
const DEFAULT_RUNTIME_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

/// Shut down the given tokio runtime and wait for it if
/// `wait` is set.
///
/// Dropping the runtime on the current thread could block due to tokio pool
/// teardown. Instead, we drop it on a separate thread and optionally wait for
/// completion.
fn runtime_shutdown(rt: tokio::runtime::Runtime, wait: bool) {
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
                tracing::warn!(%err, "runtime shutdown timed out");
            });
    }
}
