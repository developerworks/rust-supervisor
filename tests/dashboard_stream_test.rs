use rust_supervisor::dashboard::events::{event_to_record, log_record_for_event};
use rust_supervisor::event::payload::{SupervisorEvent, What, Where};
use rust_supervisor::event::time::{CorrelationId, EventSequence, EventTime, When};
use rust_supervisor::id::types::{Attempt, Generation, SupervisorPath};
use uuid::Uuid;

fn event(sequence: u64) -> SupervisorEvent {
    SupervisorEvent::new(
        When::new(EventTime::deterministic(
            sequence as u128,
            sequence as u128,
            0,
            Generation::initial(),
            Attempt::first(),
        )),
        Where::new(SupervisorPath::root().join("payment_loop")),
        What::ChildRestarted { restart_count: 1 },
        EventSequence::new(sequence),
        CorrelationId::from_uuid(Uuid::nil()),
        7,
    )
}

#[test]
fn dashboard_event_records_preserve_sequence_and_correlation() {
    let first = event_to_record("payments", "cfg-7", &event(1));
    let second = event_to_record("payments", "cfg-7", &event(2));

    assert!(first.sequence < second.sequence);
    assert_eq!(first.correlation_id, second.correlation_id);
    assert_eq!(second.severity, "warning");
}

#[test]
fn dashboard_log_records_can_correlate_to_events() {
    let event = event_to_record("payments", "cfg-7", &event(3));
    let log = log_record_for_event(&event, "child restarted");

    assert_eq!(log.sequence, Some(event.sequence));
    assert_eq!(
        log.correlation_id.as_deref(),
        Some(event.correlation_id.as_str())
    );
}
