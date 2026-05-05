//! Target process dynamic registration payloads.
//!
//! The target process builds this payload after local IPC is ready. The relay
//! validates uniqueness, lease, and authorization policy before exposing it to
//! remote sessions.

use crate::dashboard::config::ValidatedDashboardIpcConfig;
use crate::dashboard::error::DashboardError;
use crate::dashboard::model::TargetProcessRegistration;

/// Builds the registration payload for an enabled IPC target.
///
/// # Arguments
///
/// - `config`: Validated IPC configuration.
///
/// # Returns
///
/// Returns a payload ready to send to the relay registration socket.
pub fn build_registration_payload(
    config: &ValidatedDashboardIpcConfig,
) -> Result<TargetProcessRegistration, DashboardError> {
    let registration = config.registration.as_ref().ok_or_else(|| {
        DashboardError::validation(
            "registration",
            Some(config.target_id.clone()),
            "dynamic registration is not enabled",
        )
    })?;
    Ok(TargetProcessRegistration {
        target_id: config.target_id.clone(),
        display_name: registration.display_name.clone(),
        ipc_path: config.path.to_string_lossy().into_owned(),
        authorization_scope: registration.authorization_scope.clone(),
        lease_seconds: registration.lease_seconds,
    })
}

/// Serializes a registration payload as one JSON line.
///
/// # Arguments
///
/// - `registration`: Registration payload.
///
/// # Returns
///
/// Returns newline-delimited JSON.
pub fn registration_to_line(
    registration: &TargetProcessRegistration,
) -> Result<String, DashboardError> {
    let mut line = serde_json::to_string(registration).map_err(|error| {
        DashboardError::new(
            "serialization_failed",
            "registration_write",
            Some(registration.target_id.clone()),
            format!("failed to serialize registration: {error}"),
            false,
        )
    })?;
    line.push('\n');
    Ok(line)
}
