//! Identifier value types for supervised trees.
//!
//! The module owns stable IDs, paths, start_counts, and generations used across the
//! supervisor runtime.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use uuid::Uuid;

/// Stable identifier for a child task.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct ChildId {
    /// Human-readable child identifier from configuration.
    pub value: String,
}

impl ChildId {
    /// Creates a child identifier from a non-empty string.
    ///
    /// # Arguments
    ///
    /// - `value`: Identifier text supplied by configuration or code.
    ///
    /// # Returns
    ///
    /// Returns a [`ChildId`] that preserves the input value.
    ///
    /// # Examples
    ///
    /// ```
    /// let id = rust_supervisor::id::types::ChildId::new("worker");
    /// assert_eq!(id.value, "worker");
    /// ```
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
        }
    }
}

impl Display for ChildId {
    /// Formats the child identifier as its stable string value.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.value)
    }
}

/// Stable identifier for a supervisor node.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SupervisorId {
    /// UUID value generated for the supervisor node.
    pub value: Uuid,
}

impl SupervisorId {
    /// Creates a new random supervisor identifier.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a new [`SupervisorId`].
    ///
    /// # Examples
    ///
    /// ```
    /// let id = rust_supervisor::id::types::SupervisorId::new();
    /// assert!(!id.value.is_nil());
    /// ```
    pub fn new() -> Self {
        Self {
            value: Uuid::new_v4(),
        }
    }
}

impl Default for SupervisorId {
    /// Creates the default supervisor identifier.
    fn default() -> Self {
        Self::new()
    }
}

/// Path of a supervisor or child within a supervisor tree.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct SupervisorPath {
    /// Ordered path segments from root to the current node.
    pub segments: Vec<String>,
}

impl SupervisorPath {
    /// Creates a root path.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the root [`SupervisorPath`].
    ///
    /// # Examples
    ///
    /// ```
    /// let path = rust_supervisor::id::types::SupervisorPath::root();
    /// assert_eq!(path.to_string(), "/");
    /// ```
    pub fn root() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    /// Creates a child path by appending a segment.
    ///
    /// # Arguments
    ///
    /// - `segment`: Child segment that should be appended to the current path.
    ///
    /// # Returns
    ///
    /// Returns a new [`SupervisorPath`] with the segment appended.
    ///
    /// # Examples
    ///
    /// ```
    /// let path = rust_supervisor::id::types::SupervisorPath::root().join("worker");
    /// assert_eq!(path.to_string(), "/worker");
    /// ```
    pub fn join(&self, segment: impl Into<String>) -> Self {
        let mut segments = self.segments.clone();
        segments.push(segment.into());
        Self { segments }
    }

    /// Returns the parent path when this path is not the root.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the parent [`SupervisorPath`] or `None` for the root path.
    pub fn parent(&self) -> Option<Self> {
        let mut segments = self.segments.clone();
        segments.pop()?;
        Some(Self { segments })
    }
}

impl Display for SupervisorPath {
    /// Formats the supervisor path as an absolute slash path.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        if self.segments.is_empty() {
            formatter.write_str("/")
        } else {
            write!(formatter, "/{}", self.segments.join("/"))
        }
    }
}

/// Monotonic child_start_count number for a child run.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
pub struct ChildStartCount {
    /// One-based child_start_count number.
    pub value: u64,
}

impl ChildStartCount {
    /// Creates the first child_start_count value.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns child_start_count number one.
    pub fn first() -> Self {
        Self { value: 1 }
    }

    /// Advances this child_start_count value.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the next child_start_count value.
    pub fn next(self) -> Self {
        Self {
            value: self.value.saturating_add(1),
        }
    }
}

/// Monotonic generation number for a child runtime slot.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
pub struct Generation {
    /// Zero-based generation number.
    pub value: u64,
}

impl Generation {
    /// Creates the initial generation value.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns generation zero.
    pub fn initial() -> Self {
        Self { value: 0 }
    }

    /// Advances this generation value.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the next generation value.
    pub fn next(self) -> Self {
        Self {
            value: self.value.saturating_add(1),
        }
    }
}
