//! Run summary construction for diagnostics.
//!
//! The builder derives an operator-facing summary from the event journal and
//! final current state. It does not inspect runtime internals.

use crate::error::types::TaskFailure;
use crate::event::payload::{PolicyDecision, SupervisorEvent, What};
use crate::journal::ring::EventJournal;
use crate::state::supervisor::SupervisorState;
use serde::{Deserialize, Serialize};

/// Diagnostic summary for one supervisor run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunSummary {
    /// Run start time in nanoseconds since the Unix epoch.
    pub started_at_unix_nanos: u128,
    /// Run finish time in nanoseconds since the Unix epoch.
    pub finished_at_unix_nanos: u128,
    /// Shutdown cause when the run ended through shutdown.
    pub shutdown_cause: Option<String>,
    /// Total restart count inferred from recent events.
    pub restart_count: u64,
    /// Total failure count inferred from recent events.
    pub failure_count: u64,
    /// Recent typed failures.
    pub recent_failures: Vec<TaskFailure>,
    /// Recent lifecycle events retained for replay.
    pub recent_events: Vec<SupervisorEvent>,
    /// Final current state.
    pub final_state: SupervisorState,
    /// Final policy decision when one was recorded.
    pub final_decision: Option<PolicyDecision>,
}

/// Builder for [`RunSummary`].
#[derive(Debug, Clone)]
pub struct RunSummaryBuilder {
    /// Maximum number of events copied from the journal.
    pub recent_event_limit: usize,
}

impl RunSummaryBuilder {
    /// Creates a run summary builder.
    ///
    /// # Arguments
    ///
    /// - `recent_event_limit`: Maximum number of recent journal events copied.
    ///
    /// # Returns
    ///
    /// Returns a [`RunSummaryBuilder`].
    ///
    /// # Examples
    ///
    /// ```
    /// let builder = rust_supervisor::summary::builder::RunSummaryBuilder::new(8);
    /// assert_eq!(builder.recent_event_limit, 8);
    /// ```
    pub fn new(recent_event_limit: usize) -> Self {
        Self { recent_event_limit }
    }

    /// Builds a run summary from journal and final state.
    ///
    /// # Arguments
    ///
    /// - `journal`: Event journal that contains recent lifecycle facts.
    /// - `final_state`: Final current state for the run.
    /// - `shutdown_cause`: Optional shutdown cause.
    ///
    /// # Returns
    ///
    /// Returns a [`RunSummary`] derived from the inputs.
    pub fn build(
        &self,
        journal: &EventJournal,
        final_state: SupervisorState,
        shutdown_cause: Option<String>,
    ) -> RunSummary {
        let recent_events = journal.recent(self.recent_event_limit);
        let started_at_unix_nanos = started_at(&recent_events);
        let finished_at_unix_nanos = finished_at(&recent_events);
        let recent_failures = collect_failures(&recent_events);
        RunSummary {
            started_at_unix_nanos,
            finished_at_unix_nanos,
            shutdown_cause,
            restart_count: count_restarts(&recent_events),
            failure_count: recent_failures.len() as u64,
            final_decision: last_decision(&recent_events),
            recent_failures,
            recent_events,
            final_state,
        }
    }
}

impl Default for RunSummaryBuilder {
    /// Creates the default run summary builder.
    fn default() -> Self {
        Self::new(32)
    }
}

/// Reads the first event timestamp.
///
/// # Arguments
///
/// - `events`: Events retained for the summary.
///
/// # Returns
///
/// Returns zero when no events exist.
fn started_at(events: &[SupervisorEvent]) -> u128 {
    events
        .first()
        .map(|event| event.when.time.unix_nanos)
        .unwrap_or(0)
}

/// Reads the last event timestamp.
///
/// # Arguments
///
/// - `events`: Events retained for the summary.
///
/// # Returns
///
/// Returns zero when no events exist.
fn finished_at(events: &[SupervisorEvent]) -> u128 {
    events
        .last()
        .map(|event| event.when.time.unix_nanos)
        .unwrap_or(0)
}

/// Collects typed failures from recent events.
///
/// # Arguments
///
/// - `events`: Events retained for the summary.
///
/// # Returns
///
/// Returns failures in event order.
fn collect_failures(events: &[SupervisorEvent]) -> Vec<TaskFailure> {
    events
        .iter()
        .filter_map(|event| match &event.what {
            What::ChildFailed { failure } => Some(failure.clone()),
            _ => None,
        })
        .collect()
}

/// Counts restart events.
///
/// # Arguments
///
/// - `events`: Events retained for the summary.
///
/// # Returns
///
/// Returns the number of child restart events.
fn count_restarts(events: &[SupervisorEvent]) -> u64 {
    events
        .iter()
        .filter(|event| matches!(event.what, What::ChildRestarted { .. }))
        .count() as u64
}

/// Finds the last policy decision.
///
/// # Arguments
///
/// - `events`: Events retained for the summary.
///
/// # Returns
///
/// Returns the last policy decision when one exists.
fn last_decision(events: &[SupervisorEvent]) -> Option<PolicyDecision> {
    events.iter().rev().find_map(|event| event.policy.clone())
}
