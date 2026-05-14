//! Runtime loop mailbox messages.
//!
//! This module separates externally requested control commands from internal
//! child-attempt and control-plane messages that share the same runtime loop
//! mailbox.

use crate::child_runner::runner::ChildRunReport;
use crate::control::command::{CommandMeta, CommandResult, ControlCommand};
use crate::error::types::SupervisorError;
use crate::id::types::ChildId;
use crate::runtime::lifecycle::RuntimeExitReport;
use tokio::sync::oneshot;

/// Message consumed by the runtime control loop mailbox.
#[derive(Debug)]
pub enum RuntimeLoopMessage {
    /// Control command sent from [`crate::control::handle::SupervisorHandle`].
    Control {
        /// Command to execute.
        command: ControlCommand,
        /// Reply channel used to return the command result.
        reply_sender: oneshot::Sender<Result<CommandResult, SupervisorError>>,
    },
    /// Message emitted by a child attempt task.
    ChildAttempt(ChildAttemptMessage),
    /// Message that controls the runtime control plane itself.
    ControlPlane(ControlPlaneMessage),
}

/// Message emitted after a child attempt changes runtime state.
#[derive(Debug)]
pub enum ChildAttemptMessage {
    /// Child attempt finished and must be evaluated by runtime policy.
    Exited {
        /// Report returned by the child runner.
        report: Box<ChildRunReport>,
    },
    /// Child attempt could not start.
    StartFailed {
        /// Child identifier whose attempt failed before execution.
        child_id: ChildId,
        /// Diagnostic message for the failed attempt.
        message: String,
    },
}

/// Message that controls the runtime control plane task.
#[derive(Debug)]
pub enum ControlPlaneMessage {
    /// Request to stop the runtime control plane itself.
    Shutdown {
        /// Audit metadata for the shutdown request.
        meta: CommandMeta,
        /// Reply channel used to confirm shutdown acceptance.
        reply_sender: oneshot::Sender<Result<RuntimeExitReport, SupervisorError>>,
    },
}
