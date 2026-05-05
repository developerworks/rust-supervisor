//! Event journal tests.
//!
//! These tests verify fixed-capacity retention behavior.

use rust_supervisor::event::payload::{SupervisorEvent, What, Where};
use rust_supervisor::event::time::{CorrelationId, EventSequence, EventTime, When};
use rust_supervisor::id::types::{Attempt, Generation, SupervisorPath};
use rust_supervisor::journal::ring::EventJournal;
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
        Where::new(SupervisorPath::root()),
        What::ChildRunning { transition: None },
        EventSequence::new(sequence),
        CorrelationId::from_uuid(Uuid::nil()),
        1,
    )
}

#[test]
fn journal_keeps_recent_events_and_counts_dropped_entries() {
    let mut journal = EventJournal::new(2);

    journal.push(event(1));
    journal.push(event(2));
    journal.push(event(3));

    let recent = journal.recent(2);
    assert_eq!(journal.len(), 2);
    assert_eq!(journal.dropped_count, 1);
    assert_eq!(journal.last_sequence, Some(EventSequence::new(3)));
    assert_eq!(recent[0].sequence.value, 2);
    assert_eq!(recent[1].sequence.value, 3);
}
