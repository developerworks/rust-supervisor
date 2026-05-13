//! Target process dynamic registration payloads.
//!
//! The target process builds this payload after local IPC is ready. The relay
//! validates uniqueness and lease before exposing it to remote sessions.

use crate::dashboard::config::ValidatedDashboardIpcConfig;
use crate::dashboard::error::DashboardError;
use crate::dashboard::model::{SupportedCommand, TargetProcessRegistration};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::time::{Duration, sleep};

/// Registration ack returned by relay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistrationAck {
    /// Whether relay accepted the registration.
    pub ok: bool,
    /// Target identifier acknowledged by relay.
    pub target_id: Option<String>,
    /// Successful status text.
    pub status: Option<String>,
    /// Structured error returned by relay.
    pub error: Option<RegistrationAckError>,
    /// Whether supervisor can retry the registration.
    pub retryable: bool,
}

/// Structured registration ack error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistrationAckError {
    /// Stable error code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

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
        lease_seconds: registration.lease_seconds,
        supported_commands: default_supported_commands(),
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

/// Sends one registration upsert to relay.
///
/// # Arguments
///
/// - `config`: Validated IPC configuration.
///
/// # Returns
///
/// Returns the relay registration ack.
pub async fn send_registration_upsert(
    config: &ValidatedDashboardIpcConfig,
) -> Result<RegistrationAck, DashboardError> {
    let registration = config.registration.as_ref().ok_or_else(|| {
        DashboardError::validation(
            "registration",
            Some(config.target_id.clone()),
            "dynamic registration is not enabled",
        )
    })?;
    let payload = build_registration_payload(config)?;
    let line = registration_to_line(&payload)?;
    let stream = UnixStream::connect(&registration.relay_registration_path)
        .await
        .map_err(|error| {
            DashboardError::new(
                "registration_connect_failed",
                "registration_send",
                Some(config.target_id.clone()),
                format!("failed to connect relay registration socket: {error}"),
                true,
            )
        })?;
    let mut stream = BufReader::new(stream);
    stream
        .get_mut()
        .write_all(line.as_bytes())
        .await
        .map_err(|error| {
            DashboardError::new(
                "registration_write_failed",
                "registration_send",
                Some(config.target_id.clone()),
                format!("failed to write registration upsert: {error}"),
                true,
            )
        })?;

    let mut ack_line = String::new();
    stream.read_line(&mut ack_line).await.map_err(|error| {
        DashboardError::new(
            "registration_ack_read_failed",
            "registration_send",
            Some(config.target_id.clone()),
            format!("failed to read registration ack: {error}"),
            true,
        )
    })?;
    let ack: RegistrationAck = serde_json::from_str(ack_line.trim()).map_err(|error| {
        DashboardError::new(
            "registration_ack_decode_failed",
            "registration_send",
            Some(config.target_id.clone()),
            format!("failed to decode registration ack: {error}"),
            false,
        )
    })?;
    Ok(ack)
}

/// Runs registration heartbeat until a non-retryable ack stops it.
///
/// # Arguments
///
/// - `config`: Validated IPC configuration.
///
/// # Returns
///
/// Returns an error when relay reports a non-retryable registration failure.
pub async fn run_registration_heartbeat(
    config: ValidatedDashboardIpcConfig,
) -> Result<(), DashboardError> {
    loop {
        match send_registration_upsert(&config).await {
            Ok(ack) if ack.ok => {}
            Ok(ack) if !ack.retryable => {
                let detail = ack
                    .error
                    .map(|error| format!("{}: {}", error.code, error.message))
                    .unwrap_or_else(|| "registration failed".to_owned());
                return Err(DashboardError::new(
                    "registration_failed",
                    "registration_heartbeat",
                    Some(config.target_id.clone()),
                    detail,
                    false,
                ));
            }
            Ok(_) | Err(_) => {}
        }
        let interval = config
            .registration
            .as_ref()
            .map(|registration| registration.registration_heartbeat_interval_seconds)
            .unwrap_or(15);
        sleep(Duration::from_secs(interval)).await;
    }
}

/// Returns the first dashboard command set supported by the target IPC server.
fn default_supported_commands() -> Vec<SupportedCommand> {
    [
        "restart_child",
        "pause_child",
        "resume_child",
        "quarantine_child",
        "remove_child",
        "add_child",
        "shutdown_tree",
    ]
    .into_iter()
    .map(|name| SupportedCommand {
        name: name.to_owned(),
        idempotent: false,
        timeout_seconds: 30,
    })
    .collect()
}
