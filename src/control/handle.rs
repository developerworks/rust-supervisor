//! Public runtime control handle.
//!
//! The handle owns the command sender side and exposes asynchronous control
//! methods. It keeps command construction separate from runtime execution.

use crate::control::command::{CommandMeta, CommandResult, ControlCommand};
use crate::error::types::SupervisorError;
use crate::id::types::{ChildId, SupervisorPath};
use crate::runtime::control_loop::RuntimeCommand;
use tokio::sync::{broadcast, mpsc, oneshot};

/// Cloneable handle used to control a running supervisor.
#[derive(Debug, Clone)]
pub struct SupervisorHandle {
    command_sender: mpsc::Sender<RuntimeCommand>,
    event_sender: broadcast::Sender<String>,
}

impl SupervisorHandle {
    /// Creates a runtime handle from channel senders.
    ///
    /// # Arguments
    ///
    /// - `command_sender`: Sender used to submit runtime commands.
    /// - `event_sender`: Sender used to subscribe to lifecycle events.
    ///
    /// # Returns
    ///
    /// Returns a [`SupervisorHandle`].
    pub fn new(
        command_sender: mpsc::Sender<RuntimeCommand>,
        event_sender: broadcast::Sender<String>,
    ) -> Self {
        Self {
            command_sender,
            event_sender,
        }
    }

    /// Adds a child manifest under a supervisor path.
    ///
    /// # Arguments
    ///
    /// - `target`: Supervisor path that should receive the child.
    /// - `child_manifest`: Child manifest text supplied by the caller.
    /// - `requested_by`: Actor that requested the command.
    /// - `reason`: Human-readable command reason.
    ///
    /// # Returns
    ///
    /// Returns a [`CommandResult`] after the runtime accepts the command.
    pub async fn add_child(
        &self,
        target: SupervisorPath,
        child_manifest: impl Into<String>,
        requested_by: impl Into<String>,
        reason: impl Into<String>,
    ) -> Result<CommandResult, SupervisorError> {
        self.send(ControlCommand::AddChild {
            meta: CommandMeta::new(requested_by, reason),
            target,
            child_manifest: child_manifest.into(),
        })
        .await
    }

    /// Removes a child from runtime governance.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Target child identifier.
    /// - `requested_by`: Actor that requested the command.
    /// - `reason`: Human-readable command reason.
    ///
    /// # Returns
    ///
    /// Returns a [`CommandResult`] after removal or idempotent reuse.
    pub async fn remove_child(
        &self,
        child_id: ChildId,
        requested_by: impl Into<String>,
        reason: impl Into<String>,
    ) -> Result<CommandResult, SupervisorError> {
        self.child_command(child_id, requested_by, reason, |meta, child_id| {
            ControlCommand::RemoveChild { meta, child_id }
        })
        .await
    }

    /// Restarts a child explicitly.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Target child identifier.
    /// - `requested_by`: Actor that requested the command.
    /// - `reason`: Human-readable command reason.
    ///
    /// # Returns
    ///
    /// Returns a [`CommandResult`] after restart dispatch.
    pub async fn restart_child(
        &self,
        child_id: ChildId,
        requested_by: impl Into<String>,
        reason: impl Into<String>,
    ) -> Result<CommandResult, SupervisorError> {
        self.child_command(child_id, requested_by, reason, |meta, child_id| {
            ControlCommand::RestartChild { meta, child_id }
        })
        .await
    }

    /// Pauses a child idempotently.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Target child identifier.
    /// - `requested_by`: Actor that requested the command.
    /// - `reason`: Human-readable command reason.
    ///
    /// # Returns
    ///
    /// Returns the current child state after the command.
    pub async fn pause_child(
        &self,
        child_id: ChildId,
        requested_by: impl Into<String>,
        reason: impl Into<String>,
    ) -> Result<CommandResult, SupervisorError> {
        self.child_command(child_id, requested_by, reason, |meta, child_id| {
            ControlCommand::PauseChild { meta, child_id }
        })
        .await
    }

    /// Resumes a child idempotently.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Target child identifier.
    /// - `requested_by`: Actor that requested the command.
    /// - `reason`: Human-readable command reason.
    ///
    /// # Returns
    ///
    /// Returns the current child state after the command.
    pub async fn resume_child(
        &self,
        child_id: ChildId,
        requested_by: impl Into<String>,
        reason: impl Into<String>,
    ) -> Result<CommandResult, SupervisorError> {
        self.child_command(child_id, requested_by, reason, |meta, child_id| {
            ControlCommand::ResumeChild { meta, child_id }
        })
        .await
    }

    /// Quarantines a child idempotently.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Target child identifier.
    /// - `requested_by`: Actor that requested the command.
    /// - `reason`: Human-readable command reason.
    ///
    /// # Returns
    ///
    /// Returns the current child state after the command.
    pub async fn quarantine_child(
        &self,
        child_id: ChildId,
        requested_by: impl Into<String>,
        reason: impl Into<String>,
    ) -> Result<CommandResult, SupervisorError> {
        self.child_command(child_id, requested_by, reason, |meta, child_id| {
            ControlCommand::QuarantineChild { meta, child_id }
        })
        .await
    }

    /// Shuts down the supervisor tree idempotently.
    ///
    /// # Arguments
    ///
    /// - `requested_by`: Actor that requested shutdown.
    /// - `reason`: Human-readable shutdown reason.
    ///
    /// # Returns
    ///
    /// Returns the current shutdown result.
    pub async fn shutdown_tree(
        &self,
        requested_by: impl Into<String>,
        reason: impl Into<String>,
    ) -> Result<CommandResult, SupervisorError> {
        self.send(ControlCommand::ShutdownTree {
            meta: CommandMeta::new(requested_by, reason),
        })
        .await
    }

    /// Queries the current runtime state.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a [`CommandResult::CurrentState`] value.
    pub async fn current_state(&self) -> Result<CommandResult, SupervisorError> {
        self.send(ControlCommand::CurrentState {
            meta: CommandMeta::new("system", "current_state"),
        })
        .await
    }

    /// Subscribes to runtime event text emitted by the control loop.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a broadcast receiver for event text.
    pub fn subscribe_events(&self) -> broadcast::Receiver<String> {
        self.event_sender.subscribe()
    }

    /// Sends one control command and waits for the result.
    ///
    /// # Arguments
    ///
    /// - `command`: Command that should be executed by the runtime loop.
    ///
    /// # Returns
    ///
    /// Returns a command result or a supervisor error when the runtime is gone.
    async fn send(&self, command: ControlCommand) -> Result<CommandResult, SupervisorError> {
        let (reply_sender, reply_receiver) = oneshot::channel();
        self.command_sender
            .send(RuntimeCommand::Control {
                command,
                reply_sender,
            })
            .await
            .map_err(|_| SupervisorError::InvalidTransition {
                message: "runtime control loop is closed".to_owned(),
            })?;
        reply_receiver
            .await
            .map_err(|_| SupervisorError::InvalidTransition {
                message: "runtime control loop dropped command reply".to_owned(),
            })?
    }

    /// Builds and sends a child-targeted command.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Target child identifier.
    /// - `requested_by`: Actor that requested the command.
    /// - `reason`: Human-readable command reason.
    /// - `builder`: Command builder for the child operation.
    ///
    /// # Returns
    ///
    /// Returns a command result from the runtime loop.
    async fn child_command<F>(
        &self,
        child_id: ChildId,
        requested_by: impl Into<String>,
        reason: impl Into<String>,
        builder: F,
    ) -> Result<CommandResult, SupervisorError>
    where
        F: FnOnce(CommandMeta, ChildId) -> ControlCommand,
    {
        let meta = CommandMeta::new(requested_by, reason);
        self.send(builder(meta, child_id)).await
    }
}
