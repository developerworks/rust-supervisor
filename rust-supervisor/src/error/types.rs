//! Typed errors and task failure categories.
//!
//! The module keeps expected failures observable without relying on panic or
//! lossy string-only reporting.

use crate::id::types::ChildId;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// High-level error returned by supervisor operations.
#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SupervisorError {
    /// Configuration could not be loaded or validated.
    #[error("configuration error: {message}")]
    FatalConfig {
        /// Human-readable configuration error message.
        message: String,
    },
    /// Requested child does not exist in the registry.
    #[error("child not found: {child_id}")]
    ChildNotFound {
        /// Missing child identifier.
        child_id: ChildId,
    },
    /// A requested operation is invalid for the current lifecycle state.
    #[error("invalid transition: {message}")]
    InvalidTransition {
        /// Transition failure explanation.
        message: String,
    },
    /// A supervised task reported a typed failure.
    #[error("task failure: {failure:?}")]
    Task {
        /// Failure payload reported by a task.
        failure: TaskFailure,
    },
}

impl SupervisorError {
    /// Creates a fatal configuration error.
    ///
    /// # Arguments
    ///
    /// - `message`: Configuration error explanation.
    ///
    /// # Returns
    ///
    /// Returns a [`SupervisorError::FatalConfig`] value.
    pub fn fatal_config(message: impl Into<String>) -> Self {
        Self::FatalConfig {
            message: message.into(),
        }
    }
}

/// Category used by policy and observability when a task exits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskFailureKind {
    /// The task returned a typed error.
    Error,
    /// The task panicked.
    Panic,
    /// The task exceeded a configured timeout.
    Timeout,
    /// The task became unhealthy according to heartbeat policy.
    Unhealthy,
    /// The task was cancelled as part of a shutdown or explicit command.
    Cancelled,
}

/// Typed failure information produced by a supervised task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskFailure {
    /// Failure category used by restart policy.
    pub kind: TaskFailureKind,
    /// Stable failure category label for low-cardinality metrics.
    pub category: String,
    /// Human-readable failure explanation for diagnostics.
    pub message: String,
}

impl TaskFailure {
    /// Creates a typed task failure.
    ///
    /// # Arguments
    ///
    /// - `kind`: Failure category that should drive policy decisions.
    /// - `category`: Low-cardinality label used by metrics.
    /// - `message`: Diagnostic message for operators.
    ///
    /// # Returns
    ///
    /// Returns a [`TaskFailure`] value.
    ///
    /// # Examples
    ///
    /// ```
    /// let failure = rust_supervisor::error::types::TaskFailure::new(
    ///     rust_supervisor::error::types::TaskFailureKind::Error,
    ///     "io",
    ///     "socket closed",
    /// );
    /// assert_eq!(failure.category, "io");
    /// ```
    pub fn new(
        kind: TaskFailureKind,
        category: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            category: category.into(),
            message: message.into(),
        }
    }
}
