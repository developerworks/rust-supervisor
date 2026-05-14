//! Supervisor event shape integration tests.
//!
//! These tests keep lifecycle event envelopes serializable and typed.

use rust_supervisor::event::payload::{SupervisorEvent, What, Where};
use rust_supervisor::event::time::{CorrelationId, EventSequence, EventTime, When};
use rust_supervisor::id::types::{ChildStartCount, Generation, SupervisorPath};
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

    assert!(json.contains("ChildRunning"));
    assert!(json.contains("config_version"));
}
