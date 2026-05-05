//! Minimal runtime control loop.
//!
//! This module executes control-plane commands and preserves idempotent command
//! results for the runtime handle.

use crate::control::command::{CommandResult, ControlCommand, CurrentState, ManagedChildState};
use crate::error::types::SupervisorError;
use crate::id::types::ChildId;
use crate::shutdown::coordinator::ShutdownCoordinator;
use crate::shutdown::stage::{ShutdownCause, ShutdownPolicy};
use std::collections::HashMap;
use tokio::sync::{broadcast, oneshot};

/// Command envelope sent from [`crate::control::handle::SupervisorHandle`].
#[derive(Debug)]
pub struct RuntimeCommand {
    /// Command to execute.
    pub command: ControlCommand,
    /// Reply channel used to return the command result.
    pub reply_sender: oneshot::Sender<Result<CommandResult, SupervisorError>>,
}

/// Mutable state owned by the control loop.
#[derive(Debug)]
pub struct RuntimeControlState {
    shutdown: ShutdownCoordinator,
    children: HashMap<ChildId, ManagedChildState>,
    manifests: Vec<String>,
}

impl RuntimeControlState {
    /// Creates empty control state.
    ///
    /// # Arguments
    ///
    /// - `shutdown_policy`: Policy used by the shutdown coordinator.
    ///
    /// # Returns
    ///
    /// Returns a [`RuntimeControlState`] value.
    pub fn new(shutdown_policy: ShutdownPolicy) -> Self {
        Self {
            shutdown: ShutdownCoordinator::new(shutdown_policy),
            children: HashMap::new(),
            manifests: Vec::new(),
        }
    }

    /// Executes one control command.
    ///
    /// # Arguments
    ///
    /// - `command`: Command received by the runtime.
    ///
    /// # Returns
    ///
    /// Returns a command result.
    pub fn execute(&mut self, command: ControlCommand) -> Result<CommandResult, SupervisorError> {
        match command {
            ControlCommand::AddChild { child_manifest, .. } => {
                self.manifests.push(child_manifest.clone());
                Ok(CommandResult::ChildAdded { child_manifest })
            }
            ControlCommand::RemoveChild { child_id, .. } => {
                Ok(self.set_child_state(child_id, ManagedChildState::Removed))
            }
            ControlCommand::RestartChild { child_id, .. } => {
                Ok(self.set_child_state(child_id, ManagedChildState::Running))
            }
            ControlCommand::PauseChild { child_id, .. } => {
                Ok(self.set_child_state(child_id, ManagedChildState::Paused))
            }
            ControlCommand::ResumeChild { child_id, .. } => {
                Ok(self.set_child_state(child_id, ManagedChildState::Running))
            }
            ControlCommand::QuarantineChild { child_id, .. } => {
                Ok(self.set_child_state(child_id, ManagedChildState::Quarantined))
            }
            ControlCommand::ShutdownTree { meta } => {
                let cause = ShutdownCause::new(meta.requested_by, meta.reason);
                let result = self.shutdown.request_stop(cause);
                self.shutdown.advance();
                self.shutdown.advance();
                self.shutdown.advance();
                self.shutdown.advance();
                self.shutdown.complete();
                Ok(CommandResult::Shutdown { result })
            }
            ControlCommand::CurrentState { .. } => Ok(CommandResult::CurrentState {
                state: CurrentState {
                    child_count: self.children.len(),
                    shutdown_completed: self.shutdown.phase()
                        == crate::shutdown::stage::ShutdownPhase::Completed,
                },
            }),
        }
    }

    /// Sets a child state and reports whether the operation was idempotent.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Target child identifier.
    /// - `next`: Requested managed child state.
    ///
    /// # Returns
    ///
    /// Returns a [`CommandResult::ChildState`] value.
    fn set_child_state(&mut self, child_id: ChildId, next: ManagedChildState) -> CommandResult {
        let previous = self.children.insert(child_id.clone(), next);
        CommandResult::ChildState {
            child_id,
            state: next,
            idempotent: previous == Some(next),
        }
    }
}

/// Runs the control loop until all command senders are dropped.
///
/// # Arguments
///
/// - `receiver`: Runtime command receiver.
/// - `event_sender`: Event channel used for audit text.
/// - `shutdown_policy`: Shutdown policy used by the coordinator.
///
/// # Returns
///
/// This function returns when `receiver` is closed.
pub async fn run_control_loop(
    mut receiver: tokio::sync::mpsc::Receiver<RuntimeCommand>,
    event_sender: broadcast::Sender<String>,
    shutdown_policy: ShutdownPolicy,
) {
    let mut state = RuntimeControlState::new(shutdown_policy);
    while let Some(envelope) = receiver.recv().await {
        let command_name = command_name(&envelope.command);
        let result = state.execute(envelope.command);
        let _ = event_sender.send(format!("control_command:{command_name}"));
        let _ = envelope.reply_sender.send(result);
    }
}

/// Returns a stable command name for audit text.
///
/// # Arguments
///
/// - `command`: Command being executed.
///
/// # Returns
///
/// Returns a static command name.
fn command_name(command: &ControlCommand) -> &'static str {
    match command {
        ControlCommand::AddChild { .. } => "add_child",
        ControlCommand::RemoveChild { .. } => "remove_child",
        ControlCommand::RestartChild { .. } => "restart_child",
        ControlCommand::PauseChild { .. } => "pause_child",
        ControlCommand::ResumeChild { .. } => "resume_child",
        ControlCommand::QuarantineChild { .. } => "quarantine_child",
        ControlCommand::ShutdownTree { .. } => "shutdown_tree",
        ControlCommand::CurrentState { .. } => "current_state",
    }
}
