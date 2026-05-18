//! Correlation tracking tests for end-to-end lifecycle chain validation.
//!
//! This file verifies that `CorrelationHandle` correctly links events sharing
//! the same correlation ID, detects missing lifecycle stages, and handles edge
//! cases like empty handles and truncated chains.

use rust_supervisor::event::correlation::{CorrelationHandle, CorrelationQueryError};
use rust_supervisor::event::payload::Where;
use rust_supervisor::event::payload::{FiniteF64, StateTransition, What};
use rust_supervisor::event::time::EventTime;
use rust_supervisor::event::time::{CorrelationId, EventSequence, When};
use rust_supervisor::id::types::{ChildId, ChildStartCount, Generation, SupervisorPath};
use uuid::Uuid;

/// Helper: builds a deterministic `SupervisorEvent` with a given sequence and What variant.
fn make_event(
    sequence: u64,
    what: What,
    correlation_id: CorrelationId,
) -> rust_supervisor::event::payload::SupervisorEvent {
    rust_supervisor::event::payload::SupervisorEvent::new(
        When::new(EventTime::deterministic(
            sequence as u128,
            sequence as u128,
            0,
            Generation::initial(),
            ChildStartCount::first(),
        )),
        Where::new(SupervisorPath::root().join("test-child")),
        what,
        EventSequence::new(sequence),
        correlation_id,
        1,
    )
}

#[test]
fn test_correlation_chain_complete() {
    let cid = CorrelationId::new();
    let mut handle = CorrelationHandle::new(cid, Some(ChildId::new("test-child")));

    // Link five events covering the mandatory lifecycle stages.
    handle
        .link_event(make_event(
            1,
            What::ChildStarting {
                transition: Some(StateTransition::new("idle", "starting")),
            },
            cid,
        ))
        .unwrap();
    handle
        .link_event(make_event(2, What::ChildReady { transition: None }, cid))
        .unwrap();
    handle
        .link_event(make_event(
            3,
            What::ChildFailed {
                failure: rust_supervisor::error::types::TaskFailure::new(
                    rust_supervisor::error::types::TaskFailureKind::Error,
                    "io",
                    "connection refused",
                ),
            },
            cid,
        ))
        .unwrap();
    handle
        .link_event(make_event(4, What::ChildRestarting { generation: 2 }, cid))
        .unwrap();
    handle
        .link_event(make_event(
            5,
            What::ChildStopped {
                reason: "completed".to_string(),
            },
            cid,
        ))
        .unwrap();

    let chain = handle
        .export_chain(None)
        .expect("complete chain should export successfully");
    assert_eq!(chain.len(), 5, "should have 5 events");

    // Verify chronological order via monotonic_nanos.
    for window in chain.windows(2) {
        assert!(
            window[0].when.time.monotonic_nanos <= window[1].when.time.monotonic_nanos,
            "events should be in chronological order"
        );
    }
}

#[test]
fn test_correlation_gap_detected() {
    let cid = CorrelationId::new();
    let mut handle = CorrelationHandle::new(cid, Some(ChildId::new("test-child")));

    // Only link spawn and shutdown (missing ready, failure_decision, restart_attempt).
    handle
        .link_event(make_event(1, What::ChildStarting { transition: None }, cid))
        .unwrap();
    handle
        .link_event(make_event(
            2,
            What::ChildStopped {
                reason: "completed".to_string(),
            },
            cid,
        ))
        .unwrap();

    let result = handle.export_chain(None);
    match result {
        Err(CorrelationQueryError::CorrelationGapDetected {
            missing_stages,
            present_stages,
            ..
        }) => {
            assert!(
                missing_stages.contains(&"failure_decision".to_string()),
                "should detect missing failure_decision: {:?}",
                missing_stages
            );
            assert!(
                present_stages.contains(&"spawn".to_string()),
                "should have spawn present"
            );
            assert!(
                present_stages.contains(&"shutdown".to_string()),
                "should have shutdown present"
            );
        }
        other => panic!("expected CorrelationGapDetected, got {:?}", other),
    }
}

#[test]
fn test_correlation_not_found() {
    let cid = CorrelationId::new();
    let handle = CorrelationHandle::new(cid, None);

    let result = handle.export_chain(None);
    match result {
        Err(CorrelationQueryError::CorrelationNotFound { .. }) => {} // expected
        other => panic!("expected CorrelationNotFound, got {:?}", other),
    }
}

#[test]
fn test_correlation_sequence_already_registered() {
    let cid = CorrelationId::new();
    let mut handle = CorrelationHandle::new(cid, None);

    handle
        .link_event(make_event(1, What::ChildStarting { transition: None }, cid))
        .unwrap();
    let dup = handle.link_event(make_event(1, What::ChildReady { transition: None }, cid));
    assert!(dup.is_err(), "duplicate sequence should be rejected");
}

#[test]
fn test_correlation_id_uuid_v4_format() {
    let cid = CorrelationId::new();
    let uuid_str = cid.value.to_string();

    // UUID v4 format: xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx where y is 8, 9, a, or b.
    assert_eq!(uuid_str.chars().count(), 36, "UUID should be 36 chars");
    assert_eq!(
        uuid_str.chars().nth(14),
        Some('4'),
        "UUID v4 variant digit should be 4"
    );

    // Non-nil assertion.
    assert!(!cid.value.is_nil(), "CorrelationId should not be nil");
}

#[test]
fn test_correlation_with_stage_filter() {
    let cid = CorrelationId::new();
    let mut handle = CorrelationHandle::new(cid, None);

    // Create a full 5-stage chain.
    handle
        .link_event(make_event(1, What::ChildStarting { transition: None }, cid))
        .unwrap();
    handle
        .link_event(make_event(2, What::ChildReady { transition: None }, cid))
        .unwrap();
    handle
        .link_event(make_event(
            3,
            What::ChildFailed {
                failure: rust_supervisor::error::types::TaskFailure::new(
                    rust_supervisor::error::types::TaskFailureKind::Error,
                    "io",
                    "timeout",
                ),
            },
            cid,
        ))
        .unwrap();
    handle
        .link_event(make_event(4, What::ChildRestarting { generation: 2 }, cid))
        .unwrap();
    handle
        .link_event(make_event(
            5,
            What::ChildStopped {
                reason: "completed".to_string(),
            },
            cid,
        ))
        .unwrap();

    // Filter by "spawn" stage - should return 1 event since gap detection passes on the full chain.
    let spawn_events = handle
        .export_chain(Some("spawn"))
        .expect("spawn filter should succeed on complete chain");
    assert_eq!(spawn_events.len(), 1, "should have 1 spawn event");
    assert_eq!(spawn_events[0].what.name(), "ChildStarting");
}
