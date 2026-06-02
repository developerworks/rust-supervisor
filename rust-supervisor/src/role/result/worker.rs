//! Worker role result and error values.
//!
//! Worker errors preserve the child identifier and lifecycle phase before the
//! adapter maps them into task failures.

use crate::error::types::{TaskFailure, TaskFailureKind};
use crate::id::types::ChildId;
use crate::policy::task_role_defaults::TaskRole;
use crate::role::lifecycle::RoleLifecyclePhase;
use thiserror::Error;

/// Result returned by worker role lifecycle methods.
pub type WorkerResult<T = ()> = Result<T, WorkerError>;

/// Structured error returned by a worker role.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("worker role failed during {phase:?}: {message}")]
pub struct WorkerError {
    /// Stable child identifier for the failed worker.
    pub child_id: ChildId,
    /// Lifecycle phase where the worker failed.
    pub phase: RoleLifecyclePhase,
    /// Human-readable diagnostic message.
    pub message: String,
}

impl WorkerError {
    /// Creates a worker role error.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Stable child identifier for the failed worker.
    /// - `phase`: Lifecycle phase where the worker failed.
    /// - `message`: Diagnostic message for operators.
    ///
    /// # Returns
    ///
    /// Returns a structured [`WorkerError`] value.
    ///
    /// # Examples
    ///
    /// ```
    /// let error = rust_supervisor::role::result::worker::WorkerError::new(
    ///     rust_supervisor::id::types::ChildId::new("worker"),
    ///     rust_supervisor::role::lifecycle::RoleLifecyclePhase::Work,
    ///     "queue closed",
    /// );
    /// assert_eq!(error.phase.as_str(), "work");
    /// ```
    pub fn new(child_id: ChildId, phase: RoleLifecyclePhase, message: impl Into<String>) -> Self {
        Self {
            child_id,
            phase,
            message: message.into(),
        }
    }

    /// Converts this worker error into a runtime task failure.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a [`TaskFailure`] value consumed by the runtime policy pipeline.
    pub fn into_task_failure(self) -> TaskFailure {
        TaskFailure::new(
            TaskFailureKind::Error,
            format!("{}_{}", TaskRole::Worker.as_str(), self.phase.as_str()),
            format!(
                "worker role child={} phase={} failed: {}",
                self.child_id,
                self.phase.as_str(),
                self.message
            ),
        )
    }
}
