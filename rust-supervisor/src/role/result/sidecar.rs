//! Sidecar role result and error values.
//!
//! Sidecar errors preserve the sidecar child identifier, primary child
//! identifier, and lifecycle phase before the adapter maps them into task
//! failures.

use crate::error::types::{TaskFailure, TaskFailureKind};
use crate::id::types::ChildId;
use crate::policy::task_role_defaults::TaskRole;
use crate::role::lifecycle::RoleLifecyclePhase;
use thiserror::Error;

/// Result returned by sidecar role lifecycle methods.
pub type SidecarResult<T = ()> = Result<T, SidecarError>;

/// Structured error returned by a sidecar role.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("sidecar role failed during {phase:?}: {message}")]
pub struct SidecarError {
    /// Stable child identifier for the failed sidecar.
    pub child_id: ChildId,
    /// Stable child identifier for the primary child linked to the sidecar.
    pub primary_child_id: ChildId,
    /// Lifecycle phase where the sidecar failed.
    pub phase: RoleLifecyclePhase,
    /// Human-readable diagnostic message.
    pub message: String,
}

impl SidecarError {
    /// Creates a sidecar role error.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Stable child identifier for the failed sidecar.
    /// - `primary_child_id`: Stable child identifier for the primary child.
    /// - `phase`: Lifecycle phase where the sidecar failed.
    /// - `message`: Diagnostic message for operators.
    ///
    /// # Returns
    ///
    /// Returns a structured [`SidecarError`] value.
    ///
    /// # Examples
    ///
    /// ```
    /// let error = rust_supervisor::role::result::sidecar::SidecarError::new(
    ///     rust_supervisor::id::types::ChildId::new("metrics-sidecar"),
    ///     rust_supervisor::id::types::ChildId::new("api-service"),
    ///     rust_supervisor::role::lifecycle::RoleLifecyclePhase::Run,
    ///     "socket closed",
    /// );
    /// assert_eq!(error.phase.as_str(), "run");
    /// ```
    pub fn new(
        child_id: ChildId,
        primary_child_id: ChildId,
        phase: RoleLifecyclePhase,
        message: impl Into<String>,
    ) -> Self {
        Self {
            child_id,
            primary_child_id,
            phase,
            message: message.into(),
        }
    }

    /// Converts this sidecar error into a runtime task failure.
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
            format!("{}_{}", TaskRole::Sidecar.as_str(), self.phase.as_str()),
            format!(
                "sidecar role child={} primary={} phase={} failed: {}",
                self.child_id,
                self.primary_child_id,
                self.phase.as_str(),
                self.message
            ),
        )
    }
}
