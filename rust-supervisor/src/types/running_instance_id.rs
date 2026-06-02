//! Running instance identifier for child runtime attempts.
//!
//! A [`RunningInstanceId`] pairs a [`Generation`] with a [`ChildStartCount`] to
//! uniquely identify one active attempt within a [`ChildSlot`]. It is used in
//! structured errors and audit events to pinpoint the exact execution context
//! that was running when a conflict occurred.

use crate::id::types::{ChildStartCount, Generation};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// Unique identifier for a single active child attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunningInstanceId {
    /// Generation number of the slot when the attempt was activated.
    pub generation: Generation,
    /// Monotonic attempt number for this generation.
    pub attempt: ChildStartCount,
}

impl RunningInstanceId {
    /// Creates a running instance identifier.
    ///
    /// # Arguments
    ///
    /// - `generation`: Generation number of the owning slot.
    /// - `attempt`: Monotonic attempt number.
    ///
    /// # Returns
    ///
    /// Returns a [`RunningInstanceId`].
    pub fn new(generation: Generation, attempt: ChildStartCount) -> Self {
        Self {
            generation,
            attempt,
        }
    }
}

impl Display for RunningInstanceId {
    /// Formats the running instance identifier as `gen{generation}-attempt{attempt}`.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "gen{}-attempt{}",
            self.generation.value, self.attempt.value
        )
    }
}
