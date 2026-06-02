//! Four-stage shutdown phase model and tree policy runtime helpers.
//!
//! This module owns shutdown causes, phase transitions, and runtime helpers for
//! [`TreeShutdownPolicy`](crate::spec::shutdown::TreeShutdownPolicy). It does not
//! own task handles or cancellation tokens.

use crate::spec::shutdown::TreeShutdownPolicy;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

/// Default ratio of tokio worker threads used to compute
/// `max_orphan_threshold` when no explicit value is set.
const DEFAULT_ORPHAN_THRESHOLD_WORKER_RATIO: f64 = 0.25;

/// Global counter used to ensure at most one call to
/// `detect_num_worker_threads` performs the runtime query.
static NUM_WORKER_THREADS: AtomicU32 = AtomicU32::new(0);

/// Queries the tokio runtime for the number of worker threads.
///
/// Falls back to `std::thread::available_parallelism()` when the runtime
/// is not available (e.g. in unit tests or `current_thread` mode).
fn detect_num_worker_threads() -> u32 {
    NUM_WORKER_THREADS
        .load(Ordering::Relaxed)
        .max(std::thread::available_parallelism().map_or(1, |v| v.get() as u32))
}

/// Approximates the tokio worker thread count without blocking.
pub fn compute_orphan_threshold_from_worker_count(worker_count: u32) -> u32 {
    (worker_count as f64 * DEFAULT_ORPHAN_THRESHOLD_WORKER_RATIO)
        .ceil()
        .max(1.0) as u32
}

impl TreeShutdownPolicy {
    /// Returns the total duration budget for the global shutdown hard
    /// deadline: `budget.graceful_timeout + budget.abort_wait + force_kill_margin`.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the total [`Duration`].
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use rust_supervisor::spec::shutdown::{ShutdownBudget, TreeShutdownPolicy};
    ///
    /// let policy = TreeShutdownPolicy::new(
    ///     ShutdownBudget::new(Duration::from_secs(5), Duration::from_secs(1)),
    ///     true,
    ///     Duration::from_secs(5),
    ///     0,
    /// );
    /// assert_eq!(
    ///     policy.effective_global_deadline(),
    ///     Duration::from_secs(11),
    /// );
    /// ```
    pub fn effective_global_deadline(&self) -> Duration {
        self.budget.graceful_timeout + self.budget.abort_wait + self.force_kill_margin
    }

    /// Returns the effective orphan threshold.
    ///
    /// When `max_orphan_threshold` is 0 (default), computes the threshold
    /// from the number of tokio worker threads:
    /// `max(1, ceil(num_worker_threads * 0.25))`.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the effective orphan threshold.
    pub fn effective_max_orphan_threshold(&self) -> u32 {
        if self.max_orphan_threshold > 0 {
            self.max_orphan_threshold
        } else {
            compute_orphan_threshold_from_worker_count(detect_num_worker_threads())
        }
    }
}

/// Observable phase in the four-stage shutdown state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShutdownPhase {
    /// Runtime is not shutting down.
    Idle,
    /// Stop has been requested and cancellation should propagate.
    RequestStop,
    /// Runtime is waiting for cooperative child completion.
    GracefulDrain,
    /// Runtime is aborting asynchronous stragglers when allowed.
    AbortStragglers,
    /// Runtime is reconciling final state after task completion.
    Reconcile,
    /// Shutdown has completed.
    Completed,
}

impl ShutdownPhase {
    /// Returns the next phase in the shutdown state machine.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the next [`ShutdownPhase`] or `None` when already completed.
    pub fn next(self) -> Option<Self> {
        match self {
            Self::Idle => Some(Self::RequestStop),
            Self::RequestStop => Some(Self::GracefulDrain),
            Self::GracefulDrain => Some(Self::AbortStragglers),
            Self::AbortStragglers => Some(Self::Reconcile),
            Self::Reconcile => Some(Self::Completed),
            Self::Completed => None,
        }
    }
}

/// Cause attached to a shutdown request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShutdownCause {
    /// Actor that requested shutdown.
    pub requested_by: String,
    /// Human-readable reason supplied by the caller.
    pub reason: String,
}

impl ShutdownCause {
    /// Creates a shutdown cause.
    ///
    /// # Arguments
    ///
    /// - `requested_by`: Actor that requested shutdown.
    /// - `reason`: Human-readable reason.
    ///
    /// # Returns
    ///
    /// Returns a [`ShutdownCause`].
    pub fn new(requested_by: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            requested_by: requested_by.into(),
            reason: reason.into(),
        }
    }
}
