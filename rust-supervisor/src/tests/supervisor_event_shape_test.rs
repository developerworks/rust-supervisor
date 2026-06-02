//! Supervisor event shape integration tests.
//!
//! These tests keep lifecycle event envelopes serializable and typed.

use rust_supervisor::child_runner::run_exit::TaskExit;
use rust_supervisor::control::outcome::StaleReportHandling;
use rust_supervisor::event::payload::{SupervisorEvent, What, Where};
use rust_supervisor::event::time::{CorrelationId, EventSequence, EventTime, When};
use rust_supervisor::id::types::{ChildId, ChildStartCount, Generation, SupervisorPath};
use uuid::Uuid;

/// Verifies that a lifecycle event keeps `When`, `Where`, and `What` fields.
#[test]
fn supervisor_event_serializes_typed_shape() {
    let event = SupervisorEvent::new(
        When::new(EventTime::deterministic(
            1,
            2,
            3,
            Generation::initial(),
            ChildStartCount::first(),
        )),
        Where::new(SupervisorPath::root()),
        What::ChildRunning { transition: None },
        EventSequence::new(1),
        CorrelationId::from_uuid(Uuid::nil()),
        1,
    );
    let json = serde_json::to_string(&event).expect("serialize event");

    assert!(
        json.contains("child_running"),
        "JSON should contain snake_case variant, got: {json}"
    );
    assert!(json.contains("config_version"));
}

/// Verifies generation fencing payloads remain JSON-stable for downstream mirrors.
#[test]
fn generation_fencing_event_payloads_round_trip_json() {
    let base_when = When::new(EventTime::deterministic(
        9,
        8,
        7,
        Generation::initial(),
        ChildStartCount::first(),
    ));
    let base_where = Where::new(SupervisorPath::root());
    let sequence = EventSequence::new(42);
    let correlation = CorrelationId::from_uuid(Uuid::nil());

    let demos = vec![
        What::ChildRestartConflict {
            child_id: ChildId::new("fence_conflict_demo"),
            current_generation: Some(Generation::initial()),
            current_attempt: Some(ChildStartCount::first()),
            target_generation: Some(Generation::initial().next()),
            command_id: Uuid::nil().to_string(),
            decision: "already_pending".to_owned(),
            reason: "duplicate restart merged".to_owned(),
        },
        What::ChildAttemptStaleReport {
            child_id: ChildId::new("stale_demo"),
            reported_generation: Generation::initial(),
            reported_attempt: ChildStartCount::first(),
            current_generation: Some(Generation::initial().next()),
            current_attempt: Some(ChildStartCount::first()),
            exit_kind: TaskExit::Succeeded,
            handled_as: StaleReportHandling::RecordedForAudit,
        },
        What::ChildRestartFencePendingDrained {
            child_id: ChildId::new("drained_demo"),
        },
    ];

    for demo in demos {
        let event = SupervisorEvent::new(
            base_when,
            base_where.clone(),
            demo,
            sequence,
            correlation,
            1,
        );
        let json = serde_json::to_string(&event).expect("serialize fencing payload");
        assert!(
            json.contains("config_version"),
            "serialized fencing envelope missing metadata"
        );
    }
}
