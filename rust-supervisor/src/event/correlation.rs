//! Correlation handle for end-to-end lifecycle tracking.
//!
//! This module owns the `CorrelationHandle` type that links supervisor events
//! sharing the same correlation identifier, and the query errors that callers
//! must handle when exporting event chains.

use crate::event::payload::{SupervisorEvent, What};
use crate::event::time::{CorrelationId, EventSequence};
use crate::id::types::ChildId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Duplicate event sequence error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SequenceAlreadyRegistered {
    /// The sequence that was already present.
    pub sequence: EventSequence,
}

impl std::fmt::Display for SequenceAlreadyRegistered {
    /// Formats the error showing the duplicate sequence number.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "sequence {} already registered", self.sequence.value)
    }
}

impl std::error::Error for SequenceAlreadyRegistered {}

/// Error returned by [`CorrelationHandle::export_chain`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CorrelationQueryError {
    /// No events found for the given correlation ID.
    CorrelationNotFound {
        /// The queried correlation identifier.
        correlation_id: CorrelationId,
    },
    /// Event chain is truncated due to log rotation or journal capacity.
    CorrelationTruncated {
        /// The queried correlation identifier.
        correlation_id: CorrelationId,
        /// Total events found.
        total_events: u64,
        /// Maximum events before truncation.
        max_events: u64,
    },
    /// One or more lifecycle stages are missing from the chain.
    CorrelationGapDetected {
        /// The queried correlation identifier.
        correlation_id: CorrelationId,
        /// Set of lifecycle stages that are missing.
        missing_stages: Vec<String>,
        /// Stages that are present in the chain.
        present_stages: Vec<String>,
    },
    /// Sequence collision detected (possible UUID collision).
    CorrelationConflict {
        /// The queried correlation identifier.
        correlation_id: CorrelationId,
        /// Child identifiers that conflict.
        conflicting_child_ids: Vec<ChildId>,
    },
}

impl std::fmt::Display for CorrelationQueryError {
    /// Formats the query error with correlation id and context.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CorrelationNotFound { correlation_id } => {
                write!(f, "correlation {} not found", correlation_id.value)
            }
            Self::CorrelationTruncated {
                correlation_id,
                total_events,
                max_events,
            } => {
                write!(
                    f,
                    "correlation {} truncated: {} events (max {})",
                    correlation_id.value, total_events, max_events
                )
            }
            Self::CorrelationGapDetected {
                correlation_id,
                missing_stages,
                present_stages,
            } => {
                write!(
                    f,
                    "correlation {} gap detected: missing {:?}, present {:?}",
                    correlation_id.value, missing_stages, present_stages
                )
            }
            Self::CorrelationConflict {
                correlation_id,
                conflicting_child_ids,
            } => {
                write!(
                    f,
                    "correlation {} conflict: child_ids {:?}",
                    correlation_id.value, conflicting_child_ids
                )
            }
        }
    }
}

impl std::error::Error for CorrelationQueryError {}

/// Stage names for the five mandatory lifecycle stages.
const STAGES: &[&str] = &[
    "spawn",
    "ready",
    "failure_decision",
    "restart_attempt",
    "shutdown",
];

/// Maps a `What` variant name to its lifecycle stage.
fn what_to_stage(what: &What) -> Option<&'static str> {
    match what {
        What::ChildStarting { .. } => Some("spawn"),
        What::ChildReady { .. } | What::HealthCheckPassed { .. } => Some("ready"),
        What::ChildFailed { .. } | What::ChildPanicked { .. } | What::BudgetDenied { .. } => {
            Some("failure_decision")
        }
        What::ChildRestarting { .. } | What::BackoffScheduled { .. } => Some("restart_attempt"),
        What::ChildStopped { .. }
        | What::ShutdownRequested { .. }
        | What::ShutdownCompleted { .. } => Some("shutdown"),
        _ => None,
    }
}

/// Handle that correlates supervisor events sharing a common correlation ID.
///
/// Events are stored in insertion order and can be exported as a chronologically
/// sorted chain. The chain is validated against five mandatory lifecycle stages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorrelationHandle {
    /// Correlation identifier for this chain.
    pub correlation_id: CorrelationId,
    /// Optional child identifier for scoped queries.
    pub child_id: Option<ChildId>,
    /// Events in insertion order.
    events: Vec<SupervisorEvent>,
    /// Set of registered sequence numbers for duplicate detection.
    sequences: BTreeSet<u64>,
}

impl CorrelationHandle {
    /// Creates a new correlation handle.
    ///
    /// # Arguments
    ///
    /// - `correlation_id`: UUID v4 that identifies this tracking chain.
    /// - `child_id`: Optional child identifier for scoped queries.
    ///
    /// # Returns
    ///
    /// Returns a new [`CorrelationHandle`].
    pub fn new(correlation_id: CorrelationId, child_id: Option<ChildId>) -> Self {
        Self {
            correlation_id,
            child_id,
            events: Vec::new(),
            sequences: BTreeSet::new(),
        }
    }

    /// Links a supervisor event to this correlation handle.
    ///
    /// The event is stored in chronological order. Duplicate sequence numbers
    /// are rejected.
    ///
    /// # Arguments
    ///
    /// - `event`: The supervisor event to associate.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, `Err(SequenceAlreadyRegistered)` if the
    /// event's sequence was already linked.
    pub fn link_event(&mut self, event: SupervisorEvent) -> Result<(), SequenceAlreadyRegistered> {
        if !self.sequences.insert(event.sequence.value) {
            return Err(SequenceAlreadyRegistered {
                sequence: event.sequence,
            });
        }
        self.events.push(event);
        Ok(())
    }

    /// Exports all linked events in chronological order.
    ///
    /// # Arguments
    ///
    /// - `from_stage`: Optional stage filter (e.g., "spawn", "ready").
    ///
    /// # Returns
    ///
    /// Returns a vector of [`SupervisorEvent`] sorted by `when.when.unix_nanos`,
    /// or a [`CorrelationQueryError`] if gaps are detected.
    pub fn export_chain(
        &self,
        from_stage: Option<&str>,
    ) -> Result<Vec<SupervisorEvent>, CorrelationQueryError> {
        if self.events.is_empty() {
            return Err(CorrelationQueryError::CorrelationNotFound {
                correlation_id: self.correlation_id,
            });
        }

        let mut sorted: Vec<SupervisorEvent> = self.events.clone();
        sorted.sort_by(|a, b| {
            a.when
                .time
                .monotonic_nanos
                .cmp(&b.when.time.monotonic_nanos)
                .then_with(|| a.when.time.unix_nanos.cmp(&b.when.time.unix_nanos))
        });

        // Gap detection runs on ALL events regardless of filter.
        let present_stages_all: Vec<String> = {
            let mut stages: Vec<String> = sorted
                .iter()
                .filter_map(|e| what_to_stage(&e.what))
                .map(|s| s.to_string())
                .collect();
            stages.sort();
            stages.dedup();
            stages
        };

        let present_set: std::collections::HashSet<&str> =
            present_stages_all.iter().map(|s| s.as_str()).collect();

        let missing: Vec<String> = STAGES
            .iter()
            .filter(|s| !present_set.contains(**s))
            .map(|s| s.to_string())
            .collect();

        if !missing.is_empty() {
            return Err(CorrelationQueryError::CorrelationGapDetected {
                correlation_id: self.correlation_id,
                missing_stages: missing,
                present_stages: present_stages_all,
            });
        }

        // Filter by stage only for the returned events.
        let filtered: Vec<SupervisorEvent> = if let Some(stage) = from_stage {
            sorted
                .into_iter()
                .filter(|e| what_to_stage(&e.what) == Some(stage))
                .collect()
        } else {
            sorted
        };

        Ok(filtered)
    }

    /// Returns the number of linked events.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the event count.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Reports whether this handle has no linked events.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns `true` when there are no linked events.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}
