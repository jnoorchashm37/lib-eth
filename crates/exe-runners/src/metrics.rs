//! Task executor metrics helpers.

use core::fmt;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc
};

/// Task executor metrics.
#[derive(Debug, Clone, Default)]
pub struct TaskExecutorMetrics {
    /// Number of spawned critical tasks.
    pub(crate) critical_tasks_total:                  Counter,
    /// Number of finished spawned critical tasks.
    pub(crate) finished_critical_tasks_total:         Counter,
    /// Number of spawned regular tasks.
    pub(crate) regular_tasks_total:                   Counter,
    /// Number of finished spawned regular tasks.
    pub(crate) finished_regular_tasks_total:          Counter,
    /// Number of spawned regular blocking tasks.
    pub(crate) regular_blocking_tasks_total:          Counter,
    /// Number of finished spawned regular blocking tasks.
    pub(crate) finished_regular_blocking_tasks_total: Counter
}

impl TaskExecutorMetrics {
    /// Increments the counter for spawned critical tasks.
    pub(crate) fn inc_critical_tasks(&self) {
        self.critical_tasks_total.increment(1);
    }

    /// Increments the counter for spawned regular tasks.
    pub(crate) fn inc_regular_tasks(&self) {
        self.regular_tasks_total.increment(1);
    }

    /// Increments the counter for spawned regular blocking tasks.
    pub(crate) fn inc_regular_blocking_tasks(&self) {
        self.regular_blocking_tasks_total.increment(1);
    }
}

#[derive(Clone, Default)]
pub(crate) struct Counter(Arc<AtomicU64>);

impl Counter {
    pub(crate) fn increment(&self, value: u64) {
        self.0.fetch_add(value, Ordering::Relaxed);
    }
}

impl fmt::Debug for Counter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Counter").finish()
    }
}

/// Helper type for increasing counters even if a task fails.
pub struct IncCounterOnDrop(Counter);

impl fmt::Debug for IncCounterOnDrop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("IncCounterOnDrop").finish()
    }
}

impl IncCounterOnDrop {
    /// Creates a new instance of `IncCounterOnDrop` with the given counter.
    pub(crate) const fn new(counter: Counter) -> Self {
        Self(counter)
    }
}

impl Drop for IncCounterOnDrop {
    fn drop(&mut self) {
        self.0.increment(1);
    }
}
