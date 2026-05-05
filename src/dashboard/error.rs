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
}
