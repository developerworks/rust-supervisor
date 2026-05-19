//! Runtime control command contract.
//!
//! This module owns auditable command inputs and command results. Runtime code
//! executes these commands and records state changes.

use crate::control::outcome::{ChildControlResult, ChildRuntimeRecord};
use crate::error::types::SupervisorError;
use crate::id::types::{ChildId, SupervisorPath};
use crate::shutdown::coordinator::ShutdownResult;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Stable identifier for an accepted control command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommandId {
    /// UUID value assigned when a command is created.
    pub value: Uuid,
}

impl CommandId {
    /// Creates a command identifier.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a new [`CommandId`].
    ///
    /// # Examples
    ///
    /// ```
    /// let id = rust_supervisor::control::command::CommandId::new();
    /// assert!(!id.value.is_nil());
    /// ```
    pub fn new() -> Self {
        Self {
            value: Uuid::new_v4(),
        }
    }
}

impl Default for CommandId {
    /// Creates the default command identifier.
    fn default() -> Self {
        Self::new()
    }
}

/// Audit metadata attached to each runtime control command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandMeta {
    /// Command identifier used for audit correlation.
    pub command_id: CommandId,
    /// Caller that requested the command.
    pub requested_by: String,
    /// Human-readable command reason.
    pub reason: String,
}

impl CommandMeta {
    /// Creates command metadata.
    ///
    /// # Arguments
    ///
    /// - `requested_by`: Caller that requested the command.
    /// - `reason`: Human-readable command reason.
    ///
    /// # Returns
    ///
    /// Returns a [`CommandMeta`] value with a generated command identifier.
    pub fn new(requested_by: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            command_id: CommandId::new(),
            requested_by: requested_by.into(),
            reason: reason.into(),
        }
    }

    /// Validates audit metadata before command dispatch.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` when actor and reason fields are non-empty.
    pub(crate) fn validate(&self) -> Result<(), SupervisorError> {
        validate_required_text(&self.requested_by, "requested_by")?;
        validate_required_text(&self.reason, "reason")
    }
}

/// Runtime command sent to the control loop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlCommand {
    /// Adds a child description under a supervisor path.
    AddChild {
        /// Audit metadata for the command.
        meta: CommandMeta,
        /// Target supervisor path.
        target: SupervisorPath,
        /// Child manifest text owned by the caller.
        child_manifest: String,
    },
    /// Removes a child after shutting it down.
    RemoveChild {
        /// Audit metadata for the command.
        meta: CommandMeta,
        /// Target child identifier.
        child_id: ChildId,
    },
    /// Restarts a child explicitly.
    RestartChild {
        /// Audit metadata for the command.
        meta: CommandMeta,
        /// Target child identifier.
        child_id: ChildId,
    },
    /// Pauses automatic governance for a child.
    PauseChild {
        /// Audit metadata for the command.
        meta: CommandMeta,
        /// Target child identifier.
        child_id: ChildId,
    },
    /// Resumes automatic governance for a child.
    ResumeChild {
        /// Audit metadata for the command.
        meta: CommandMeta,
        /// Target child identifier.
        child_id: ChildId,
    },
    /// Quarantines a child and stops automatic restarts.
    QuarantineChild {
        /// Audit metadata for the command.
        meta: CommandMeta,
        /// Target child identifier.
        child_id: ChildId,
    },
    /// Starts shutdown for the whole supervisor tree.
    ShutdownTree {
        /// Audit metadata for the command.
        meta: CommandMeta,
    },
    /// Reads current runtime state.
    CurrentState {
        /// Audit metadata for the command.
        meta: CommandMeta,
    },
}

impl ControlCommand {
    /// Returns audit metadata for this command.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a shared reference to [`CommandMeta`].
    pub fn meta(&self) -> &CommandMeta {
        match self {
            Self::AddChild { meta, .. }
            | Self::RemoveChild { meta, .. }
            | Self::RestartChild { meta, .. }
            | Self::PauseChild { meta, .. }
            | Self::ResumeChild { meta, .. }
            | Self::QuarantineChild { meta, .. }
            | Self::ShutdownTree { meta }
            | Self::CurrentState { meta } => meta,
        }
    }

    /// Validates audit metadata attached to this command.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` when the command carries auditable metadata.
    pub(crate) fn validate_audit_metadata(&self) -> Result<(), SupervisorError> {
        self.meta().validate()
    }
}

/// Validates one required text field.
///
/// # Arguments
///
/// - `value`: Text value supplied by the command caller.
/// - `field`: Field name used in the diagnostic message.
///
/// # Returns
///
/// Returns `Ok(())` when the value is not blank.
fn validate_required_text(value: &str, field: &str) -> Result<(), SupervisorError> {
    if value.trim().is_empty() {
        return Err(SupervisorError::InvalidTransition {
            message: format!("control command {field} must not be empty"),
        });
    }
    Ok(())
}

/// Current runtime state returned by `current_state`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrentState {
    /// Number of children known to the control loop.
    pub child_count: usize,
    /// Whether tree shutdown has completed.
    pub shutdown_completed: bool,
    /// Runtime state records for declared children.
    pub child_runtime_records: Vec<ChildRuntimeRecord>,
}

/// Result returned after a control command is executed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandResult {
    /// Child was accepted by the control loop.
    ChildAdded {
        /// Child manifest stored by the runtime.
        child_manifest: String,
    },
    /// Child control result after a command.
    ChildControl {
        /// Outcome produced by the control command.
        outcome: ChildControlResult,
    },
    /// Current state query result.
    CurrentState {
        /// Runtime current state.
        state: CurrentState,
    },
    /// Shutdown command result.
    Shutdown {
        /// Shutdown phase and cause.
        result: ShutdownResult,
    },
}
