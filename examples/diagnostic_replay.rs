//! Demonstrates journal replay, metric derivation, and run summary building.

use rust_supervisor::error::types::{TaskFailure, TaskFailureKind};
use rust_supervisor::event::payload::{
    PolicyDecision, StateTransition, SupervisorEvent, What, Where,
};
use rust_supervisor::event::time::{CorrelationId, EventSequence, EventTime, When};
use rust_supervisor::id::types::{Attempt, ChildId, Generation, SupervisorPath};
use rust_supervisor::journal::ring::EventJournal;
use rust_supervisor::observe::metrics::MetricsFacade;
use rust_supervisor::state::child::{ChildLifecycleState, ChildState};
use rust_supervisor::state::supervisor::{ShutdownState, SupervisorState};
use rust_supervisor::summary::builder::RunSummaryBuilder;
use uuid::Uuid;

fn main() {
    let child_id = ChildId::new("market_feed");
    let child_path = SupervisorPath::root().join("market_feed");
    let failure = TaskFailure::new(
        TaskFailureKind::Timeout,
        "external_dependency",
        "market feed heartbeat timed out",
    );
    let policy = PolicyDecision::new(
        "RestartAfter",
        Some(500),
        Some("heartbeat timeout is restartable".to_owned()),
    );

    let mut journal = EventJournal::new(8);
    journal.push(event(
        1,
        child_id.clone(),
        "Market Feed",
        What::ChildRunning {
            transition: Some(StateTransition::new("Starting", "Running")),
        },
        None,
    ));
    journal.push(event(
        2,
        child_id.clone(),
        "Market Feed",
        What::ChildFailed {
            failure: failure.clone(),
        },
        Some(policy.clone()),
    ));
    journal.push(event(
        3,
        child_id.clone(),
        "Market Feed",
        What::BackoffScheduled { delay_ms: 500 },
        Some(policy.clone()),
    ));
    journal.push(event(
        4,
        child_id.clone(),
        "Market Feed",
        What::ChildRestarted { restart_count: 1 },
        Some(policy.clone()),
    ));

    let child = ChildState::declared(child_path, child_id, "Market Feed")
        .with_lifecycle_state(ChildLifecycleState::Running, EventSequence::new(1))
        .record_failure(failure, EventSequence::new(2))
        .with_policy_decision(policy, 1);
    let final_state = SupervisorState::new(SupervisorPath::root(), EventSequence::new(4), 1)
        .with_child(child)
        .with_shutdown_state(ShutdownState::Completed)
        .with_journal_sequence(EventSequence::new(4));
    let summary = RunSummaryBuilder::new(8).build(
        &journal,
        final_state,
        Some("diagnostic replay complete".to_owned()),
    );
    let metrics = MetricsFacade::new();

    println!(
        "summary restart_count={} failure_count={} recent_events={}",
        summary.restart_count,
        summary.failure_count,
        summary.recent_events.len()
    );
    for event in journal.recent(8) {
        println!(
            "event={} metrics={:?}",
            event.what.name(),
            metrics.samples_for_event(&event)
        );
    }
}

fn event(
    sequence: u64,
    child_id: ChildId,
    child_name: &str,
    what: What,
    policy: Option<PolicyDecision>,
) -> SupervisorEvent {
    let sequence_value = EventSequence::new(sequence);
    let event = SupervisorEvent::new(
        When::new(EventTime::deterministic(
            sequence as u128,
            sequence as u128,
            sequence,
            Generation::initial(),
            Attempt::first(),
        )),
        Where::new(SupervisorPath::root().join(&child_id.value)).with_child(child_id, child_name),
        what,
        sequence_value,
        CorrelationId::from_uuid(Uuid::nil()),
        1,
    );

    match policy {
        Some(policy) => event.with_policy(policy),
        None => event,
    }
}
