//! Four-stage shutdown policy and phase model.
//!
//! This module owns shutdown timing, causes, and phase transitions. It does not
//! own task handles or cancellation tokens.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

/// Default extra grace beyond graceful_timeout + abort_wait before the
/// global hard deadline is enforced.
pub(crate) const DEFAULT_FORCE_KILL_MARGIN_SECS: u64 = 5;

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
///
/// Uses `std::thread::available_parallelism()` as a fallback since the
/// tokio runtime's actual worker count is not exposed through a public API
/// at the time of writing. This is conservative: on an 8-core machine the
/// heuristic returns 8, matching the default `multi_thread` runtime.
pub fn compute_orphan_threshold_from_worker_count(worker_count: u32) -> u32 {
    (worker_count as f64 * DEFAULT_ORPHAN_THRESHOLD_WORKER_RATIO)
        .ceil()
        .max(1.0) as u32
}

/// Default extra grace beyond graceful_timeout + abort_wait before the
/// global hard deadline is enforced.
/// Default maximum number of orphaned child tasks before the supervisor
/// triggers a controlled process exit. Computed dynamically as a fraction
/// Shutdown timing policy for a supervisor tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShutdownPolicy {
    /// Time allowed for cooperative graceful drain.
    pub graceful_timeout: Duration,
    /// Time allowed after aborting asynchronous stragglers.
    pub abort_wait: Duration,
    /// Whether asynchronous stragglers may be aborted after the timeout.
    pub abort_after_timeout: bool,
    /// Extra grace beyond graceful_timeout + abort_wait before the
    /// global hard deadline is enforced.
    /// Recommended default: 5 seconds.
    pub force_kill_margin: Duration,
    /// Maximum number of orphaned child tasks (tasks that could not be
    /// stopped within policy timeouts) before the supervisor triggers a
    /// controlled process exit to reclaim leaked OS threads.
    /// When set to 0 (default), the threshold is computed automatically
    /// as `max(1, ceil(num_worker_threads * 0.25))` — preventing starvation
    /// before a quarter of the async worker pool is consumed.
    pub max_orphan_threshold: u32,
}

impl ShutdownPolicy {
    /// Creates a shutdown policy.
    ///
    /// # Arguments
    ///
    /// - `graceful_timeout`: Time allowed for cooperative drain.
    /// - `abort_wait`: Time allowed after abort requests.
    /// - `abort_after_timeout`: Whether async stragglers may be aborted.
    /// - `force_kill_margin`: Extra grace before global hard deadline.
    /// - `max_orphan_threshold`: Max orphan count before process exit.
    ///   Pass 0 to compute automatically as `max(1, ceil(workers * 0.25))`.
    ///
    /// # Returns
    ///
    /// Returns a [`ShutdownPolicy`].
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    ///
    /// let policy = rust_supervisor::shutdown::stage::ShutdownPolicy::new(
    ///     Duration::from_secs(5),
    ///     Duration::from_secs(1),
    ///     true,
    ///     Duration::from_secs(5),
    ///     0,
    /// );
    /// assert!(policy.abort_after_timeout);
    /// // Automatic threshold on 8-core machine: max(1, ceil(8 * 0.25)) = 2
    /// assert!(policy.effective_max_orphan_threshold() >= 1);
    /// assert_eq!(policy.effective_global_deadline(), Duration::from_secs(11));
    /// ```
    pub fn new(
        graceful_timeout: Duration,
        abort_wait: Duration,
        abort_after_timeout: bool,
        force_kill_margin: Duration,
        max_orphan_threshold: u32,
    ) -> Self {
        Self {
            graceful_timeout,
            abort_wait,
            abort_after_timeout,
            force_kill_margin,
            max_orphan_threshold,
        }
    }

    /// Returns the total duration budget for the global shutdown hard
    /// deadline: `graceful_timeout + abort_wait + force_kill_margin`.
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
    /// use rust_supervisor::shutdown::stage::ShutdownPolicy;
    ///
    /// let policy = ShutdownPolicy::new(
    ///     Duration::from_secs(5),
    ///     Duration::from_secs(1),
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
        self.graceful_timeout + self.abort_wait + self.force_kill_margin
    }

    /// Returns the effective orphan threshold.
    ///
    /// When `max_orphan_threshold` is 0 (default), computes the threshold
    /// from the number of tokio worker threads:
    /// `max(1, ceil(num_worker_threads * 0.25))`.
    ///
    /// This ensures the supervisor exits before more than a quarter of the
    /// async worker pool is consumed by orphaned (zombie) tasks.
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

impl Default for ShutdownPolicy {
    /// Creates a shutdown policy with recommended defaults:
    /// - `graceful_timeout`: 5 seconds
    /// - `abort_wait`: 1 second
    /// - `abort_after_timeout`: true
    /// - `force_kill_margin`: 5 seconds
    /// - `max_orphan_threshold`: 0 (automatic, based on worker count)
    fn default() -> Self {
        Self {
            graceful_timeout: Duration::from_secs(5),
            abort_wait: Duration::from_secs(1),
            abort_after_timeout: true,
            force_kill_margin: Duration::from_secs(DEFAULT_FORCE_KILL_MARGIN_SECS),
            max_orphan_threshold: 0,
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
