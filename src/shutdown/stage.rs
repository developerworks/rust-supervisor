//! Four-stage shutdown policy and phase model.
//!
//! This module owns shutdown timing, causes, and phase transitions. It does not
//! own task handles or cancellation tokens.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Shutdown timing policy for a supervisor tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShutdownPolicy {
    /// Time allowed for cooperative graceful drain.
    pub graceful_timeout: Duration,
    /// Time allowed after aborting asynchronous stragglers.
    pub abort_wait: Duration,
    /// Whether asynchronous stragglers may be aborted after the timeout.
    pub abort_after_timeout: bool,
}

impl ShutdownPolicy {
    /// Creates a shutdown policy.
    ///
    /// # Arguments
    ///
    /// - `graceful_timeout`: Time allowed for cooperative drain.
    /// - `abort_wait`: Time allowed after abort requests.
    /// - `abort_after_timeout`: Whether async stragglers may be aborted.
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
    /// );
    /// assert!(policy.abort_after_timeout);
    /// ```
    pub fn new(
        graceful_timeout: Duration,
        abort_wait: Duration,
        abort_after_timeout: bool,
    ) -> Self {
        Self {
            graceful_timeout,
            abort_wait,
            abort_after_timeout,
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
