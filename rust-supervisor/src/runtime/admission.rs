//! Admission set that enforces at-most-one active attempt per child.
//!
//! The [`AdmissionSet`] is consulted before the control loop activates a new
//! [`ChildSlot`] attempt. It rejects concurrent requests with a structured
//! [`AdmissionConflict`] error.

use crate::id::types::{ChildId, ChildStartCount, Generation};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt::{Display, Formatter};

// ---------------------------------------------------------------------------
// AdmissionConflict
// ---------------------------------------------------------------------------

/// Structured error returned when a concurrent request conflicts with an
/// already-admitted active attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdmissionConflict {
    /// Child identifier that already has an active attempt.
    pub child_id: ChildId,
    /// Generation of the currently active attempt.
    pub active_generation: Generation,
    /// Attempt number of the currently active attempt.
    pub active_attempt: ChildStartCount,
    /// Human-readable description of the conflicting request.
    pub conflicting_request: String,
}

impl AdmissionConflict {
    /// Creates an admission conflict.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Child that already has an active attempt.
    /// - `active_generation`: Generation of the active attempt.
    /// - `active_attempt`: Attempt number of the active attempt.
    /// - `conflicting_request`: Description of the rejected request.
    ///
    /// # Returns
    ///
    /// Returns an [`AdmissionConflict`].
    pub fn new(
        child_id: ChildId,
        active_generation: Generation,
        active_attempt: ChildStartCount,
        conflicting_request: impl Into<String>,
    ) -> Self {
        Self {
            child_id,
            active_generation,
            active_attempt,
            conflicting_request: conflicting_request.into(),
        }
    }
}

impl Display for AdmissionConflict {
    /// Formats the conflict with child id, generation, attempt, and request.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "child {} already has active attempt gen{}-attempt{}; conflicting request: {}",
            self.child_id,
            self.active_generation.value,
            self.active_attempt.value,
            self.conflicting_request,
        )
    }
}

impl std::error::Error for AdmissionConflict {}

// ---------------------------------------------------------------------------
// AdmissionSet
// ---------------------------------------------------------------------------

/// Tracks which children currently have an active attempt admitted.
///
/// The set enforces the invariant that at most one active attempt exists per
/// [`ChildId`] at any moment. The control loop must acquire admission before
/// activating a [`ChildSlot`] and must release when the attempt finishes.
#[derive(Debug, Default)]
pub struct AdmissionSet {
    /// Set of child identifiers with an active admitted attempt.
    admitted: HashSet<ChildId>,
}

impl AdmissionSet {
    /// Creates an empty admission set.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns an [`AdmissionSet`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Attempts to admit a child for execution.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Child identifier to admit.
    /// - `active_generation`: Generation of the currently active attempt (used
    ///   only for the conflict error when admission fails).
    /// - `active_attempt`: Attempt number of the currently active attempt.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` when admission succeeds, or
    /// `Err(AdmissionConflict)` when the child already has an active attempt.
    pub fn try_admit(
        &mut self,
        child_id: ChildId,
        active_generation: Generation,
        active_attempt: ChildStartCount,
    ) -> Result<(), AdmissionConflict> {
        if self.admitted.contains(&child_id) {
            return Err(AdmissionConflict::new(
                child_id,
                active_generation,
                active_attempt,
                "restart or activate request conflicts with existing active attempt",
            ));
        }
        self.admitted.insert(child_id);
        Ok(())
    }

    /// Attempts to admit a child, returning success when the request is
    /// idempotent (same generation and attempt as the currently active one).
    ///
    /// # Arguments
    ///
    /// - `child_id`: Child identifier to admit.
    /// - `request_generation`: Generation claimed by the incoming request.
    /// - `request_attempt`: Attempt number claimed by the incoming request.
    /// - `active_generation`: Generation of the currently active attempt.
    /// - `active_attempt`: Attempt number of the currently active attempt.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` when admission succeeds or the request is idempotent,
    /// or `Err(AdmissionConflict)` when a different active attempt exists.
    pub fn try_admit_or_idempotent(
        &mut self,
        child_id: ChildId,
        request_generation: Generation,
        request_attempt: ChildStartCount,
        active_generation: Generation,
        active_attempt: ChildStartCount,
    ) -> Result<(), AdmissionConflict> {
        if self.admitted.contains(&child_id) {
            // Idempotent: same generation and attempt → treat as success.
            if request_generation == active_generation && request_attempt == active_attempt {
                return Ok(());
            }
            return Err(AdmissionConflict::new(
                child_id,
                active_generation,
                active_attempt,
                "restart or activate request conflicts with existing active attempt",
            ));
        }
        self.admitted.insert(child_id);
        Ok(())
    }

    /// Releases an admitted child from the set.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Child identifier to release.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    pub fn release(&mut self, child_id: &ChildId) {
        self.admitted.remove(child_id);
    }

    /// Returns whether a child is currently admitted.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Child identifier to check.
    ///
    /// # Returns
    ///
    /// Returns `true` when the child has an active admitted attempt.
    pub fn is_admitted(&self, child_id: &ChildId) -> bool {
        self.admitted.contains(child_id)
    }

    /// Returns the number of currently admitted children.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the count of admitted children.
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.admitted.len()
    }

    /// Returns whether the admission set is empty.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns `true` when no children are admitted.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.admitted.is_empty()
    }
}
