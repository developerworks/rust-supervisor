//! Run summary tests.
//!
//! These tests verify diagnostic summary derivation from journal events.

use rust_supervisor::error::types::{TaskFailure, TaskFailureKind};
use rust_supervisor::event::payload::{PolicyDecision, SupervisorEvent, What, Where};
use rust_supervisor::event::time::{CorrelationId, EventSequence, EventTime, When};
use rust_supervisor::id::types::{ChildStartCount, Generation, SupervisorPath};
use rust_supervisor::journal::ring::EventJournal;
use rust_supervisor::state::supervisor::SupervisorState;
use rust_supervisor::summary::builder::RunSummaryBuilder;
use uuid::Uuid;

/// Builds one deterministic summary event.
fn event(sequence: u64, what: What) -> SupervisorEvent {
    SupervisorEvent::new(
        When::new(EventTime::deterministic(
            sequence as u128,
            sequence as u128,
            0,
            Generation::initial(),
            ChildStartCount::first(),
        )),
        Where::new(SupervisorPath::root()),
        what,
        EventSequence::new(sequence),
        CorrelationId::from_uuid(Uuid::nil()),
        1,
    )
}

/// Verifies summary collection of failures, restarts, and final decisions.
#[test]
fn summary_collects_failures_restarts_and_final_decision() {
    let mut journal = EventJournal::new(8);
    let failure = TaskFailure::new(TaskFailureKind::Error, "io", "closed");
    journal.push(event(1, What::ChildFailed { failure }));
    journal.push(
        event(2, What::ChildRestarted { restart_count: 1 }).with_policy(PolicyDecision::new(
            "RestartAfter",
            Some(100),
            None,
        )),
    );
    let final_state = SupervisorState::new(SupervisorPath::root(), EventSequence::new(3), 1);

    let summary =
        RunSummaryBuilder::new(8).build(&journal, final_state, Some("operator".to_owned()));

    assert_eq!(summary.failure_count, 1);
    assert_eq!(summary.restart_count, 1);
    assert_eq!(summary.shutdown_cause.as_deref(), Some("operator"));
    assert_eq!(
        summary
            .final_decision
            .as_ref()
            .map(|decision| decision.decision.as_str()),
        Some("RestartAfter")
    );
}
