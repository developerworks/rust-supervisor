//! Structured dashboard IPC errors.
//!
//! The dashboard feature exchanges errors with relay code over JSON. This
//! module keeps those errors typed so tests can assert code, stage, target, and
//! retry behavior without parsing strings.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error returned by target-side dashboard IPC handlers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Error)]
#[error("{code} at {stage}: {message}")]
pub struct DashboardError {
    /// Stable machine-readable error code.
    pub code: String,
    /// Processing stage that produced the error.
    pub stage: String,
    /// Optional target process identifier related to the error.
    pub target_id: Option<String>,
    /// Human-readable diagnostic message.
    pub message: String,
    /// Whether a caller can retry after the reported condition changes.
    pub retryable: bool,
}

impl DashboardError {
    /// Creates a structured dashboard error.
    ///
    /// # Arguments
    ///
    /// - `code`: Stable machine-readable error code.
    /// - `stage`: Processing stage that produced the error.
    /// - `target_id`: Optional target process identifier.
    /// - `message`: Human-readable diagnostic message.
    /// - `retryable`: Whether a retry can later succeed.
    ///
    /// # Returns
    ///
    /// Returns a [`DashboardError`] value ready for JSON serialization.
    pub fn new(
        code: impl Into<String>,
        stage: impl Into<String>,
        target_id: Option<String>,
        message: impl Into<String>,
        retryable: bool,
    ) -> Self {
        Self {
            code: code.into(),
            stage: stage.into(),
            target_id,
            message: message.into(),
            retryable,
        }
    }

    /// Creates an unsupported method error.
    ///
    /// # Arguments
    ///
    /// - `method`: Method name rejected by the parser.
    ///
    /// # Returns
    ///
    /// Returns a non-retryable [`DashboardError`].
    pub fn unsupported_method(method: impl AsRef<str>) -> Self {
        Self::new(
            "unsupported_method",
            "protocol_parse",
            None,
            format!("unsupported dashboard IPC method {}", method.as_ref()),
            false,
        )
    }

    /// Creates a validation error.
    ///
    /// # Arguments
    ///
    /// - `stage`: Validation stage.
    /// - `target_id`: Optional target process identifier.
    /// - `message`: Human-readable validation message.
    ///
    /// # Returns
    ///
    /// Returns a non-retryable [`DashboardError`].
    pub fn validation(
        stage: impl Into<String>,
        target_id: Option<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::new("validation_failed", stage, target_id, message, false)
    }

    /// Creates a target unavailable error.
    ///
    /// # Arguments
    ///
    /// - `stage`: Processing stage that tried to use the target.
    /// - `target_id`: Target process identifier.
    /// - `message`: Human-readable diagnostic message.
    ///
    /// # Returns
    ///
    /// Returns a retryable [`DashboardError`].
    pub fn target_unavailable(
        stage: impl Into<String>,
        target_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::new(
            "target_unavailable",
            stage,
            Some(target_id.into()),
            message,
            true,
        )
    }

    // ------------------------------------------------------------------
    // IPC security error constructors (C1-C9)
    // ------------------------------------------------------------------

    /// Creates a socket owner mismatch error (C1).
    pub fn ipc_socket_owner_mismatch(message: impl Into<String>) -> Self {
        Self::new(
            "ipc_socket_owner_mismatch",
            "ipc_bind",
            None,
            message,
            false,
        )
    }

    /// Creates a peer credential uid mismatch error (C2).
    pub fn peer_cred_uid_mismatch(expected: u32, got: u32) -> Self {
        Self::new(
            "peer_cred_uid_mismatch",
            "peer_credentials",
            None,
            format!("peer uid mismatch: expected {expected}, got {got}"),
            false,
        )
    }

    /// Creates a peer credential gid not allowed error (C2).
    pub fn peer_cred_gid_not_allowed(gid: u32) -> Self {
        Self::new(
            "peer_cred_gid_not_allowed",
            "peer_credentials",
            None,
            format!("peer gid {gid} is not in the allowed gid list"),
            false,
        )
    }

    /// Creates a peer credential pid not allowed error (C2).
    pub fn peer_cred_pid_not_allowed(pid: u32) -> Self {
        Self::new(
            "peer_cred_pid_not_allowed",
            "peer_credentials",
            None,
            format!("peer pid {pid} is not in the allowed pid list"),
            false,
        )
    }

    /// Creates a peer credential unavailable error (C2).
    pub fn peer_cred_unavailable(message: impl Into<String>) -> Self {
        Self::new(
            "peer_cred_unavailable",
            "peer_credentials",
            None,
            message,
            false,
        )
    }

    /// Creates an authorization denied error (C3).
    pub fn authz_denied(method: impl Into<String>) -> Self {
        Self::new(
            "authz_denied",
            "authorization",
            None,
            format!("command {} is not authorized", method.into()),
            false,
        )
    }

    /// Creates an authorization not configured error (C3).
    pub fn authz_not_configured() -> Self {
        Self::new(
            "authz_not_configured",
            "authorization",
            None,
            "command authorization is not configured",
            false,
        )
    }

    /// Creates a replay detected error (C4).
    pub fn replay_detected(request_id: impl Into<String>) -> Self {
        Self::new(
            "replay_detected",
            "replay_protection",
            None,
            format!("replay detected for request_id {}", request_id.into()),
            false,
        )
    }

    /// Creates a request too large error (C5).
    pub fn request_too_large(actual: usize, max_bytes: usize) -> Self {
        Self::new(
            "request_too_large",
            "size_limit",
            None,
            format!("request body {actual} bytes exceeds limit of {max_bytes} bytes"),
            false,
        )
    }

    /// Creates a rate limit exceeded error (C6).
    pub fn rate_limit_exceeded() -> Self {
        Self::new(
            "rate_limit_exceeded",
            "rate_limit",
            None,
            "rate limit exceeded",
            false,
        )
    }

    /// Creates an audit write failed error (C7).
    pub fn audit_write_failed(message: impl Into<String>) -> Self {
        Self::new("audit_write_failed", "audit", None, message, false)
    }

    /// Creates an audit queue full error (C7).
    pub fn audit_queue_full() -> Self {
        Self::new(
            "audit_queue_full",
            "audit",
            None,
            "audit defer queue is full",
            false,
        )
    }

    /// Creates an allowlist denied error (C9).
    pub fn allowlist_denied(path: impl Into<String>) -> Self {
        Self::new(
            "allowlist_denied",
            "allowlist",
            None,
            format!("external command not in allowlist: {}", path.into()),
            false,
        )
    }

    /// Creates an allowlist empty error (C9).
    pub fn allowlist_empty() -> Self {
        Self::new(
            "allowlist_empty",
            "allowlist",
            None,
            "external command allowlist is empty — all external commands are denied",
            false,
        )
    }
}
