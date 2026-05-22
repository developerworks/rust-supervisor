//! Target-side dashboard IPC configuration validation.
//!
//! Public YAML input lives in [`crate::config::configurable`]. This module owns
//! semantic checks that are specific to opening a local IPC endpoint.

use crate::config::configurable::{DashboardIpcBindMode, DashboardIpcConfig};
use crate::config::ipc_security::IpcSecurityConfig;
use crate::dashboard::error::DashboardError;

/// Validated target-side dashboard IPC configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedDashboardIpcConfig {
    /// Target process identifier exposed to the relay.
    pub target_id: String,
    /// Local Unix domain socket path.
    pub path: std::path::PathBuf,
    /// Socket file permission string.
    pub permissions: String,
    /// Bind behavior when the socket path already exists.
    pub bind_mode: DashboardIpcBindMode,
    /// Optional dynamic registration settings.
    pub registration: Option<ValidatedDashboardRegistrationConfig>,
    /// Optional IPC security pipeline configuration.
    pub security_config: Option<IpcSecurityConfig>,
}

/// Validated dynamic registration configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedDashboardRegistrationConfig {
    /// Relay registration socket path.
    pub relay_registration_path: std::path::PathBuf,
    /// Display name sent to relay and UI.
    pub display_name: String,
    /// Registration lease duration in seconds.
    pub lease_seconds: u64,
    /// Registration heartbeat interval in seconds.
    pub registration_heartbeat_interval_seconds: u64,
}

/// Validates optional dashboard IPC configuration.
///
/// # Arguments
///
/// - `config`: Optional public configuration section.
///
/// # Returns
///
/// Returns `Ok(None)` when IPC is absent or disabled, and a validated
/// configuration when IPC is enabled.
pub fn validate_dashboard_ipc_config(
    config: Option<&DashboardIpcConfig>,
) -> Result<Option<ValidatedDashboardIpcConfig>, DashboardError> {
    let Some(config) = config else {
        return Ok(None);
    };
    if !config.enabled {
        return Ok(None);
    }
    let target_id = required_text(config.target_id.as_deref(), "dashboard.target_id")?;
    let path = config.path.clone().ok_or_else(|| {
        DashboardError::validation(
            "config",
            None,
            "dashboard.path is required when dashboard IPC is enabled",
        )
    })?;
    if !path.is_absolute() {
        return Err(DashboardError::validation(
            "config",
            Some(target_id.clone()),
            "dashboard.path must be absolute",
        ));
    }
    let registration = validate_registration(config, &target_id)?;
    // Validate security config numerical bounds when present.
    if let Some(ref security) = config.security_config {
        validate_security_config(security, &target_id)?;
    }
    let security_config = config.security_config.clone();
    Ok(Some(ValidatedDashboardIpcConfig {
        target_id,
        path,
        permissions: config
            .permissions
            .clone()
            .unwrap_or_else(|| "0600".to_owned()),
        bind_mode: config.bind_mode.unwrap_or(DashboardIpcBindMode::CreateNew),
        registration,
        security_config,
    }))
}

/// Validates dynamic registration settings for an enabled IPC target.
///
/// # Arguments
///
/// - `config`: Public IPC configuration section.
/// - `target_id`: Validated target process identifier.
///
/// # Returns
///
/// Returns optional validated registration settings.
fn validate_registration(
    config: &DashboardIpcConfig,
    target_id: &str,
) -> Result<Option<ValidatedDashboardRegistrationConfig>, DashboardError> {
    let Some(registration) = config.registration.as_ref() else {
        return Ok(None);
    };
    if !registration.enabled {
        return Ok(None);
    }
    let relay_registration_path =
        registration
            .relay_registration_path
            .clone()
            .ok_or_else(|| {
                DashboardError::validation(
                    "config",
                    Some(target_id.to_owned()),
                    "dashboard.registration.relay_registration_path is required",
                )
            })?;
    if !relay_registration_path.is_absolute() {
        return Err(DashboardError::validation(
            "config",
            Some(target_id.to_owned()),
            "dashboard.registration.relay_registration_path must be absolute",
        ));
    }
    let lease_seconds = registration.lease_seconds.ok_or_else(|| {
        DashboardError::validation(
            "config",
            Some(target_id.to_owned()),
            "dashboard.registration.lease_seconds is required",
        )
    })?;
    if lease_seconds == 0 {
        return Err(DashboardError::validation(
            "config",
            Some(target_id.to_owned()),
            "dashboard.registration.lease_seconds must be greater than zero",
        ));
    }
    let heartbeat_seconds = registration
        .registration_heartbeat_interval_seconds
        .unwrap_or(15);
    if heartbeat_seconds == 0 || heartbeat_seconds >= lease_seconds {
        return Err(DashboardError::validation(
            "config",
            Some(target_id.to_owned()),
            "dashboard.registration.registration_heartbeat_interval_seconds must be positive and less than lease_seconds",
        ));
    }
    Ok(Some(ValidatedDashboardRegistrationConfig {
        relay_registration_path,
        display_name: registration
            .display_name
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| target_id.to_owned()),
        lease_seconds,
        registration_heartbeat_interval_seconds: heartbeat_seconds,
    }))
}

/// Reads a required non-empty text field.
///
/// # Arguments
///
/// - `value`: Optional field value.
/// - `field`: Public configuration field name.
///
/// # Returns
///
/// Returns trimmed text or a validation error.
fn required_text(value: Option<&str>, field: &str) -> Result<String, DashboardError> {
    let text = value.unwrap_or_default().trim();
    if text.is_empty() {
        Err(DashboardError::validation(
            "config",
            None,
            format!("{field} must not be empty"),
        ))
    } else {
        Ok(text.to_owned())
    }
}

/// Validates IPC security configuration numerical bounds.
///
/// Ensures that rate limits, replay windows, idempotency caches, and
/// request size limits have sane non-zero values. Returns a validation
/// error when a bound is outside the acceptable range.
fn validate_security_config(
    security: &crate::config::ipc_security::IpcSecurityConfig,
    target_id: &str,
) -> Result<(), DashboardError> {
    if security.rate_limit.enabled {
        if security.rate_limit.refill_rate <= 0.0 {
            return Err(DashboardError::validation(
                "config",
                Some(target_id.to_owned()),
                "dashboard.security.rate_limit.refill_rate must be positive",
            ));
        }
        if security.rate_limit.burst_capacity == 0 {
            return Err(DashboardError::validation(
                "config",
                Some(target_id.to_owned()),
                "dashboard.security.rate_limit.burst_capacity must be > 0",
            ));
        }
    }
    if security.replay_protection.enabled {
        if security.replay_protection.window_size == 0 {
            return Err(DashboardError::validation(
                "config",
                Some(target_id.to_owned()),
                "dashboard.security.replay_protection.window_size must be > 0",
            ));
        }
        if security.replay_protection.ttl_seconds == 0 {
            return Err(DashboardError::validation(
                "config",
                Some(target_id.to_owned()),
                "dashboard.security.replay_protection.ttl_seconds must be > 0",
            ));
        }
    }
    if security.idempotency.enabled {
        if security.idempotency.max_cached_results == 0 {
            return Err(DashboardError::validation(
                "config",
                Some(target_id.to_owned()),
                "dashboard.security.idempotency.max_cached_results must be > 0",
            ));
        }
        if security.idempotency.result_cache_ttl_seconds == 0 {
            return Err(DashboardError::validation(
                "config",
                Some(target_id.to_owned()),
                "dashboard.security.idempotency.result_cache_ttl_seconds must be > 0",
            ));
        }
    }
    if security.request_size_limit.enabled && security.request_size_limit.max_bytes == 0 {
        return Err(DashboardError::validation(
            "config",
            Some(target_id.to_owned()),
            "dashboard.security.request_size_limit.max_bytes must be > 0",
        ));
    }
    Ok(())
}
