//! Observability pipeline tests.
//!
//! These tests verify metrics, journal, tracing, and subscriber fan-out.

use rust_supervisor::event::payload::{SupervisorEvent, What, Where};
use rust_supervisor::event::time::{CorrelationId, EventSequence, EventTime, When};
use rust_supervisor::id::types::{Attempt, ChildId, Generation, SupervisorPath};
use rust_supervisor::observe::metrics::MetricsFacade;
use rust_supervisor::observe::pipeline::ObservabilityPipeline;
use uuid::Uuid;

/// Builds a deterministic supervisor event for observability tests.
/// Builds one deterministic observability event.
fn event(sequence: u64, what: What) -> SupervisorEvent {
    let child_id = ChildId::new("worker");
    SupervisorEvent::new(
        When::new(EventTime::deterministic(
            sequence as u128,
            sequence as u128,
            0,
            Generation::initial(),
            Attempt::first(),
        )),
        Where::new(SupervisorPath::root().join("worker")).with_child(child_id, "Worker"),
        what,
        EventSequence::new(sequence),
        CorrelationId::from_uuid(Uuid::nil()),
        1,
    )
}

/// Verifies that metric labels reject unbounded keys.
/// Verifies that unbounded metric label keys are rejected.
#[test]
fn metrics_facade_rejects_unbounded_label_keys() {
    let facade = MetricsFacade::new();

    assert!(facade.validate_label("state", "running").is_ok());
    assert!(
        facade
            .validate_label("error_message", "socket closed")
            .is_err()
    );
}

/// Verifies fan-out, subscriber lag, metrics, and journal recording.
/// Verifies that the pipeline fans out signals and accounts for lag.
#[test]
fn pipeline_fans_out_signals_and_accounts_for_lag() {
    let mut pipeline = ObservabilityPipeline::new(8, 1);
    let subscriber = pipeline.add_subscriber();

    pipeline.emit(event(1, What::ChildRestarted { restart_count: 1 }));
    pipeline.emit(event(2, What::ChildRestarted { restart_count: 2 }));

    let queued = pipeline.drain_subscriber(subscriber);
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].sequence.value, 2);
    assert_eq!(pipeline.test_recorder.subscriber_lag, 1);
    assert_eq!(pipeline.test_recorder.events.len(), 2);
    assert_eq!(pipeline.test_recorder.metrics.len(), 2);
    assert_eq!(pipeline.journal.len(), 2);
}
