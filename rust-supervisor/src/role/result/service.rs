//! Service role result and error values.
//!
//! Service errors preserve the child identifier and lifecycle phase before the
//! adapter maps them into task failures.

use crate::error::types::{TaskFailure, TaskFailureKind};
use crate::id::types::ChildId;
use crate::policy::task_role_defaults::TaskRole;
use crate::role::lifecycle::RoleLifecyclePhase;
use thiserror::Error;

/// Result returned by service role lifecycle methods.
pub type ServiceResult<T = ()> = Result<T, ServiceError>;

/// Structured error returned by a service role.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("service role failed during {phase:?}: {message}")]
pub struct ServiceError {
    /// Stable child identifier for the failed service.
    pub child_id: ChildId,
    /// Lifecycle phase where the service failed.
    pub phase: RoleLifecyclePhase,
    /// Human-readable diagnostic message.
    pub message: String,
}

impl ServiceError {
    /// Creates a service role error.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Stable child identifier for the failed service.
    /// - `phase`: Lifecycle phase where the service failed.
    /// - `message`: Diagnostic message for operators.
    ///
    /// # Returns
    ///
    /// Returns a structured [`ServiceError`] value.
    ///
    /// # Examples
    ///
    /// ```
    /// let error = rust_supervisor::role::result::service::ServiceError::new(
    ///     rust_supervisor::id::types::ChildId::new("service"),
    ///     rust_supervisor::role::lifecycle::RoleLifecyclePhase::Run,
    ///     "socket closed",
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

    /// Converts this service error into a runtime task failure.
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
            format!("{}_{}", TaskRole::Service.as_str(), self.phase.as_str()),
            format!(
                "service role child={} phase={} failed: {}",
                self.child_id,
                self.phase.as_str(),
                self.message
            ),
        )
    }
}
