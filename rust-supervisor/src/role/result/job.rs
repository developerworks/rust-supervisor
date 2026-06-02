//! Job role result and error values.
//!
//! Job errors preserve the child identifier and lifecycle phase before the
//! adapter maps them into task failures.

use crate::error::types::{TaskFailure, TaskFailureKind};
use crate::id::types::ChildId;
use crate::policy::task_role_defaults::TaskRole;
use crate::role::lifecycle::RoleLifecyclePhase;
use thiserror::Error;

/// Result returned by job role lifecycle methods.
pub type JobResult<T = ()> = Result<T, JobError>;

/// Structured error returned by a job role.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("job role failed during {phase:?}: {message}")]
pub struct JobError {
    /// Stable child identifier for the failed job.
    pub child_id: ChildId,
    /// Lifecycle phase where the job failed.
    pub phase: RoleLifecyclePhase,
    /// Human-readable diagnostic message.
    pub message: String,
}

impl JobError {
    /// Creates a job role error.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Stable child identifier for the failed job.
    /// - `phase`: Lifecycle phase where the job failed.
    /// - `message`: Diagnostic message for operators.
    ///
    /// # Returns
    ///
    /// Returns a structured [`JobError`] value.
    ///
    /// # Examples
    ///
    /// ```
    /// let error = rust_supervisor::role::result::job::JobError::new(
    ///     rust_supervisor::id::types::ChildId::new("job"),
    ///     rust_supervisor::role::lifecycle::RoleLifecyclePhase::Run,
    ///     "batch failed",
    /// );
    /// assert_eq!(error.phase.as_str(), "run");
    /// ```
    pub fn new(child_id: ChildId, phase: RoleLifecyclePhase, message: impl Into<String>) -> Self {
        Self {
            child_id,
            phase,
            message: message.into(),
        }
    }

    /// Converts this job error into a runtime task failure.
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
            format!("{}_{}", TaskRole::Job.as_str(), self.phase.as_str()),
            format!(
                "job role child={} phase={} failed: {}",
                self.child_id,
                self.phase.as_str(),
                self.message
            ),
        )
    }
}
