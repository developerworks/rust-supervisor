//! Runtime loop mailbox messages.
//!
//! This module separates externally requested control commands from internal
//! child-child_start_count and control-plane messages that share the same runtime loop
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
    /// Message emitted by a child child_start_count task.
    ChildStart(ChildStartMessage),
    /// Message that controls the runtime control plane itself.
    ControlPlane(ControlPlaneMessage),
}

/// Message emitted after a child child_start_count changes runtime state.
#[derive(Debug)]
pub enum ChildStartMessage {
    /// Child child_start_count finished and must be evaluated by runtime policy.
    Exited {
        /// Report returned by the child runner.
        report: Box<ChildRunReport>,
    },
    /// Child child_start_count could not start.
    StartFailed {
        /// Child identifier whose child_start_count failed before execution.
        child_id: ChildId,
        /// Diagnostic message for the failed child_start_count.
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
