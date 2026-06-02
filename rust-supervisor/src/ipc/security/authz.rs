//! Command authorization (C3).
//!
//! Classifies IPC methods into risk categories and enforces fine-grained
//! authorization: Read methods are allowed for any authenticated peer;
//! WriteChild and Destructive methods require the peer uid to be in the
//! configured allowed_uids list.

use crate::config::ipc_security::AuthorizationConfig;
use crate::dashboard::error::DashboardError;

/// IPC actions classified by risk level for authorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcRiskAction {
    /// Read-only: hello, state, subscribe.
    Read,
    /// Write: restart, pause, resume, quarantine, add.
    WriteChild,
    /// Destructive: remove, shutdown_tree.
    Destructive,
}

impl IpcRiskAction {
    /// Classifies an IPC method name into its risk category.
    ///
    /// # Arguments
    ///
    /// - `method`: The wire method name (e.g. "command.restart_child").
    ///
    /// # Returns
    ///
    /// Returns the appropriate [`IpcRiskAction`] variant. Unknown commands
    /// are conservatively classified as `WriteChild`.
    pub fn classify(method: &str) -> Self {
        match method {
            "hello" | "state" | "events.subscribe" | "logs.tail" => Self::Read,
            "command.restart_child"
            | "command.pause_child"
            | "command.resume_child"
            | "command.quarantine_child"
            | "command.add_child" => Self::WriteChild,
            "command.remove_child" | "command.shutdown_tree" => Self::Destructive,
            _ => Self::WriteChild, // Unknown commands: treat as write for safety
        }
    }
}

/// Verifies that a peer is authorized to execute the given IPC method (C3).
///
/// # Arguments
///
/// - `method`: The IPC method name.
/// - `peer_uid`: The peer's user identifier.
/// - `config`: Authorization configuration.
///
/// # Returns
///
/// Returns `Ok(())` when authorized, or `Err(DashboardError)` with
/// `authz_denied` on failure.
pub fn verify_authorization(
    method: &str,
    peer_uid: u32,
    config: &AuthorizationConfig,
) -> Result<(), DashboardError> {
    if !config.enabled {
        return Ok(());
    }

    let risk = IpcRiskAction::classify(method);

    match risk {
        IpcRiskAction::Read => Ok(()),
        IpcRiskAction::WriteChild | IpcRiskAction::Destructive => {
            if config.allowed_uids.is_empty() {
                return Err(DashboardError::authz_not_configured());
            }
            if !config.allowed_uids.contains(&peer_uid) {
                return Err(DashboardError::authz_denied(method));
            }
            Ok(())
        }
    }
}
