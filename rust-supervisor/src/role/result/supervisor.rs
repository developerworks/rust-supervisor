//! Supervisor role result and error values.
//!
//! Supervisor role errors preserve the child identifier and lifecycle phase
//! before the adapter maps them into task failures.

use crate::error::types::{SupervisorError, TaskFailure, TaskFailureKind};
use crate::id::types::ChildId;
use crate::policy::task_role_defaults::TaskRole;
use crate::role::lifecycle::RoleLifecyclePhase;
use thiserror::Error;

/// Result returned by supervisor role lifecycle methods.
pub type SupervisorResult<T = ()> = Result<T, SupervisorRoleError>;

/// Structured error returned by a supervisor role.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("supervisor role failed during {phase:?}: {message}")]
pub struct SupervisorRoleError {
    /// Stable child identifier for the failed supervisor role.
    pub child_id: ChildId,
    /// Lifecycle phase where the supervisor role failed.
    pub phase: RoleLifecyclePhase,
    /// Human-readable diagnostic message.
    pub message: String,
}

impl SupervisorRoleError {
    /// Creates a supervisor role error.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Stable child identifier for the failed supervisor role.
    /// - `phase`: Lifecycle phase where the supervisor role failed.
    /// - `message`: Diagnostic message for operators.
    ///
    /// # Returns
    ///
    /// Returns a structured [`SupervisorRoleError`] value.
    ///
    /// # Examples
    ///
    /// ```
    /// let error = rust_supervisor::role::result::supervisor::SupervisorRoleError::new(
    ///     rust_supervisor::id::types::ChildId::new("nested-supervisor"),
    ///     rust_supervisor::role::lifecycle::RoleLifecyclePhase::BuildTree,
    ///     "nested tree could not be built",
    /// );
    /// assert_eq!(error.phase.as_str(), "build_tree");
    /// ```
    pub fn new(child_id: ChildId, phase: RoleLifecyclePhase, message: impl Into<String>) -> Self {
        Self {
            child_id,
            phase,
            message: message.into(),
        }
    }

    /// Creates a supervisor role error from a runtime supervisor error.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Stable child identifier for the failed supervisor role.
    /// - `phase`: Lifecycle phase that observed the runtime error.
    /// - `error`: Runtime supervisor error returned by the nested supervisor.
    ///
    /// # Returns
    ///
    /// Returns a structured [`SupervisorRoleError`] value with the runtime
    /// error rendered into its diagnostic message.
    pub fn from_supervisor_error(
        child_id: ChildId,
        phase: RoleLifecyclePhase,
        error: SupervisorError,
    ) -> Self {
        Self::new(child_id, phase, error.to_string())
    }

    /// Converts this supervisor role error into a runtime task failure.
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
            format!("{}_{}", TaskRole::Supervisor.as_str(), self.phase.as_str()),
            format!(
                "supervisor role child={} phase={} failed: {}",
                self.child_id,
                self.phase.as_str(),
                self.message
            ),
        )
    }
}
