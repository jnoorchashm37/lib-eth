//! Additional helpers for executing blocking calls.

use std::{
    any::Any,
    cell::RefCell,
    future::Future,
    panic::{catch_unwind, AssertUnwindSafe},
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc
    },
    task::{ready, Context, Poll},
    thread
};

use tokio::sync::{oneshot, AcquireError, OwnedSemaphorePermit, Semaphore};

/// RPC tracing call guard semaphore.
#[derive(Clone, Debug)]
pub struct BlockingTaskGuard(Arc<Semaphore>);

impl BlockingTaskGuard {
    /// Create a new `BlockingTaskGuard` with the given maximum number of
    /// blocking tasks in parallel.
    pub fn new(max_blocking_tasks: usize) -> Self {
        Self(Arc::new(Semaphore::new(max_blocking_tasks)))
    }

    /// See also [`Semaphore::acquire_owned`]
    pub async fn acquire_owned(self) -> Result<OwnedSemaphorePermit, AcquireError> {
        self.0.acquire_owned().await
    }

    /// See also [`Semaphore::acquire_many_owned`]
    pub async fn acquire_many_owned(self, n: u32) -> Result<OwnedSemaphorePermit, AcquireError> {
        self.0.acquire_many_owned(n).await
    }
}

/// Used to execute blocking tasks on a rayon thread pool from within a tokio
/// runtime.
#[derive(Clone, Debug)]
pub struct BlockingTaskPool {
    pool: Arc<rayon::ThreadPool>
}

impl BlockingTaskPool {
    /// Create a new `BlockingTaskPool` with the given thread pool.
    pub fn new(pool: rayon::ThreadPool) -> Self {
        Self { pool: Arc::new(pool) }
    }

    /// Convenience function to start building a new thread pool.
    pub fn builder() -> rayon::ThreadPoolBuilder {
        rayon::ThreadPoolBuilder::new()
    }

    /// Convenience function to build a new thread pool with the default
    /// configuration.
    pub fn build() -> Result<Self, rayon::ThreadPoolBuildError> {
        Self::builder().build().map(Self::new)
    }

    /// Asynchronous wrapper around Rayon's [`ThreadPool::spawn`].
    pub fn spawn<F, R>(&self, func: F) -> BlockingTaskHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static
    {
        let (tx, rx) = oneshot::channel();

        self.pool.spawn(move || {
            let _result = tx.send(catch_unwind(AssertUnwindSafe(func)));
        });

        BlockingTaskHandle { rx }
    }

    /// Asynchronous wrapper around Rayon's [`ThreadPool::spawn_fifo`].
    pub fn spawn_fifo<F, R>(&self, func: F) -> BlockingTaskHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static
    {
        let (tx, rx) = oneshot::channel();

        self.pool.spawn_fifo(move || {
            let _result = tx.send(catch_unwind(AssertUnwindSafe(func)));
        });

        BlockingTaskHandle { rx }
    }
}

/// Async handle for a blocking task running in a Rayon thread pool.
#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
#[pin_project::pin_project]
pub struct BlockingTaskHandle<T> {
    #[pin]
    pub(crate) rx: oneshot::Receiver<thread::Result<T>>
}

impl<T> Future for BlockingTaskHandle<T> {
    type Output = thread::Result<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match ready!(self.project().rx.poll(cx)) {
            Ok(res) => Poll::Ready(res),
            Err(_) => Poll::Ready(Err(Box::<TokioBlockingTaskError>::default()))
        }
    }
}

/// An error returned when the Tokio channel is dropped while awaiting a result.
#[derive(Debug, Default, thiserror::Error)]
#[error("tokio channel dropped while awaiting result")]
#[non_exhaustive]
pub struct TokioBlockingTaskError;

thread_local! {
    static WORKER: RefCell<Worker> = const { RefCell::new(Worker::new()) };
}

/// A rayon thread pool with per-thread [`Worker`] state.
#[derive(Debug)]
pub struct WorkerPool {
    pool: rayon::ThreadPool
}

impl WorkerPool {
    /// Creates a new `WorkerPool` with the given number of threads.
    pub fn new(num_threads: usize) -> Result<Self, rayon::ThreadPoolBuildError> {
        Self::from_builder(rayon::ThreadPoolBuilder::new().num_threads(num_threads))
    }

    /// Creates a new `WorkerPool` from a [`rayon::ThreadPoolBuilder`].
    pub fn from_builder(builder: rayon::ThreadPoolBuilder) -> Result<Self, rayon::ThreadPoolBuildError> {
        Ok(Self { pool: build_pool_with_panic_handler(builder)? })
    }

    /// Returns the total number of threads in the underlying rayon pool.
    pub fn current_num_threads(&self) -> usize {
        self.pool.current_num_threads()
    }

    /// Initializes per-thread [`Worker`] state on every thread in the pool.
    pub fn init<T: 'static>(&self, f: impl Fn(Option<&mut T>) -> T + Sync) {
        self.broadcast(self.pool.current_num_threads(), |worker| {
            worker.init::<T>(&f);
        });
    }

    /// Runs a closure on `num_threads` threads in the pool, giving mutable
    /// access to each thread's [`Worker`].
    pub fn broadcast(&self, num_threads: usize, f: impl Fn(&mut Worker) + Sync) {
        if num_threads >= self.pool.current_num_threads() {
            self.pool.broadcast(|_| {
                WORKER.with_borrow_mut(|worker| f(worker));
            });
        } else {
            let remaining = AtomicUsize::new(num_threads);
            self.pool.broadcast(|_| {
                let mut current = remaining.load(Ordering::Relaxed);
                loop {
                    if current == 0 {
                        return;
                    }
                    match remaining.compare_exchange_weak(current, current - 1, Ordering::Relaxed, Ordering::Relaxed) {
                        Ok(_) => break,
                        Err(actual) => current = actual
                    }
                }
                WORKER.with_borrow_mut(|worker| f(worker));
            });
        }
    }

    /// Clears the state on every thread in the pool.
    pub fn clear(&self) {
        self.pool.broadcast(|_| {
            WORKER.with_borrow_mut(Worker::clear);
        });
    }

    /// Runs a closure on the pool with access to the calling thread's
    /// [`Worker`].
    pub fn install<R: Send>(&self, f: impl FnOnce(&Worker) -> R + Send) -> R {
        self.pool.install(|| WORKER.with_borrow(|worker| f(worker)))
    }

    /// Runs a closure on the pool without worker state access.
    pub fn install_fn<R: Send>(&self, f: impl FnOnce() -> R + Send) -> R {
        self.pool.install(f)
    }

    /// Spawns a closure on the pool.
    pub fn spawn(&self, f: impl FnOnce() + Send + 'static) {
        self.pool.spawn(f);
    }

    /// Executes `f` on this pool using [`rayon::in_place_scope`].
    pub fn in_place_scope<'scope, R>(&self, f: impl FnOnce(&rayon::Scope<'scope>) -> R) -> R {
        self.pool.in_place_scope(f)
    }

    /// Access the current thread's [`Worker`] from within an [`install`]
    /// closure.
    pub fn with_worker<R>(f: impl FnOnce(&Worker) -> R) -> R {
        WORKER.with_borrow(|worker| f(worker))
    }

    /// Mutably access the current thread's [`Worker`] from within a pool
    /// closure.
    pub fn with_worker_mut<R>(f: impl FnOnce(&mut Worker) -> R) -> R {
        WORKER.with_borrow_mut(|worker| f(worker))
    }
}

/// Builds a rayon thread pool with a panic handler that prevents aborting the
/// process.
pub fn build_pool_with_panic_handler(
    builder: rayon::ThreadPoolBuilder
) -> Result<rayon::ThreadPool, rayon::ThreadPoolBuildError> {
    builder.panic_handler(|_| {}).build()
}

/// Per-thread state container for a [`WorkerPool`].
#[derive(Debug, Default)]
pub struct Worker {
    state: Option<Box<dyn Any>>
}

impl Worker {
    /// Creates a new empty `Worker`.
    const fn new() -> Self {
        Self { state: None }
    }

    /// Initializes the worker state.
    pub fn init<T: 'static>(&mut self, f: impl FnOnce(Option<&mut T>) -> T) {
        let existing = self
            .state
            .take()
            .and_then(|mut b| b.downcast_mut::<T>().is_some().then_some(b));

        let new_state = match existing {
            Some(mut boxed) => {
                let r = boxed.downcast_mut::<T>().expect("type checked above");
                *r = f(Some(r));
                boxed
            }
            None => Box::new(f(None))
        };

        self.state = Some(new_state);
    }

    /// Returns a reference to the state, downcasted to `T`.
    pub fn get<T: 'static>(&self) -> &T {
        self.state
            .as_ref()
            .expect("worker not initialized")
            .downcast_ref::<T>()
            .expect("worker state type mismatch")
    }

    /// Returns a mutable reference to the state, downcasted to `T`.
    pub fn get_mut<T: 'static>(&mut self) -> &mut T {
        self.state
            .as_mut()
            .expect("worker not initialized")
            .downcast_mut::<T>()
            .expect("worker state type mismatch")
    }

    /// Returns a mutable reference to the state, initializing it with `f` on
    /// first access.
    pub fn get_or_init<T: 'static>(&mut self, f: impl FnOnce() -> T) -> &mut T {
        self.state
            .get_or_insert_with(|| Box::new(f()))
            .downcast_mut::<T>()
            .expect("worker state type mismatch")
    }

    /// Clears the worker state, dropping the contained value.
    pub fn clear(&mut self) {
        self.state = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn blocking_pool() {
        let pool = BlockingTaskPool::build().unwrap();
        let res = pool.spawn(move || 5);
        let res = res.await.unwrap();
        assert_eq!(res, 5);
    }

    #[tokio::test]
    async fn blocking_pool_panic() {
        let pool = BlockingTaskPool::build().unwrap();
        let res = pool.spawn(move || -> i32 {
            panic!();
        });
        let res = res.await;
        assert!(res.is_err());
    }

    #[test]
    fn worker_pool_init_and_access() {
        let pool = WorkerPool::new(2).unwrap();

        pool.broadcast(2, |worker| {
            worker.init::<Vec<u8>>(|_| vec![1, 2, 3]);
        });

        let sum: u8 = pool.install(|worker| {
            let v = worker.get::<Vec<u8>>();
            v.iter().sum()
        });
        assert_eq!(sum, 6);

        pool.clear();
    }

    #[test]
    fn worker_pool_reinit_reuses_resources() {
        let pool = WorkerPool::new(1).unwrap();

        pool.broadcast(1, |worker| {
            worker.init::<Vec<u8>>(|existing| {
                assert!(existing.is_none());
                vec![1, 2, 3]
            });
        });

        pool.broadcast(1, |worker| {
            worker.init::<Vec<u8>>(|existing| {
                let v = existing.expect("should have existing state");
                assert_eq!(v, &mut vec![1, 2, 3]);
                v.push(4);
                std::mem::take(v)
            });
        });

        let len = pool.install(|worker| worker.get::<Vec<u8>>().len());
        assert_eq!(len, 4);

        pool.clear();
    }

    #[test]
    fn worker_pool_clear_and_reinit() {
        let pool = WorkerPool::new(1).unwrap();

        pool.broadcast(1, |worker| {
            worker.init::<u64>(|_| 42);
        });
        let val = pool.install(|worker| *worker.get::<u64>());
        assert_eq!(val, 42);

        pool.clear();

        pool.broadcast(1, |worker| {
            worker.init::<String>(|_| "hello".to_string());
        });
        let val = pool.install(|worker| worker.get::<String>().clone());
        assert_eq!(val, "hello");

        pool.clear();
    }

    #[test]
    fn worker_pool_par_iter_with_worker() {
        use rayon::prelude::*;

        let pool = WorkerPool::new(2).unwrap();

        pool.broadcast(2, |worker| {
            worker.init::<u64>(|_| 10);
        });

        let results: Vec<u64> = pool.install(|_| {
            (0u64..4)
                .into_par_iter()
                .map(|i| WorkerPool::with_worker(|w| i + *w.get::<u64>()))
                .collect()
        });
        assert_eq!(results, vec![10, 11, 12, 13]);

        pool.clear();
    }
}
