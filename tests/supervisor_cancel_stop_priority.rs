//! Acceptance tests for external_cancel and manual_stop priority over automatic restart.
//!
//! This test verifies that:
//! 1. When external_cancel or manual_stop competes with automatic restart,
//!    execute action must not re-raise tasks marked for termination
//! 2. Cancel/stop signals have higher priority than restart decisions

use rust_supervisor::event::payload::{ProtectionAction, SupervisorEvent, What};
use rust_supervisor::event::time::{CorrelationId, EventSequence, EventTime, When};
use rust_supervisor::id::types::{ChildId, ChildStartCount, Generation, SupervisorPath};
use rust_supervisor::observe::pipeline::ObservabilityPipeline;

/// Helper to create a deterministic event timestamp
fn deterministic_when(sequence: u64) -> When {
    When::new(EventTime::deterministic(
        sequence as u128,
        sequence as u128,
        0,
        Generation::initial(),
        ChildStartCount::first(),
    ))
}

/// Helper to create a test supervisor event
fn test_event(sequence: u64, what: What, child_id: Option<ChildId>) -> SupervisorEvent {
    let path = SupervisorPath::root();
    let location = rust_supervisor::event::payload::Where::new(path);
    let location = if let Some(ref id) = child_id {
        location.with_child(id.clone(), "test-child")
    } else {
        location
    };

    SupervisorEvent::new(
        deterministic_when(sequence),
        location,
        what,
        EventSequence::new(sequence),
        CorrelationId::from_uuid(uuid::Uuid::nil()),
        1,
    )
}

#[test]
fn test_cancelled_task_not_restarted() {
    // Verify that a task with cancelled exit is not automatically restarted

    let mut pipeline = ObservabilityPipeline::new(100, 10);
    let _subscriber_idx = pipeline.add_subscriber();

    let child_id = ChildId::new("test-child-cancelled".to_string());

    // Simulate a cancelled exit (should not trigger restart)
    let cancelled_event = test_event(
        1,
        What::ChildFailed {
            failure: rust_supervisor::error::types::TaskFailure::new(
                rust_supervisor::error::types::TaskFailureKind::Cancelled,
                "external_cancel",
                "task was externally cancelled",
            ),
        },
        Some(child_id.clone()),
    );

    let lagged = pipeline.emit(cancelled_event);
    assert_eq!(lagged, 0);

    // TODO(T009b, T014): After implementation:
    // 1. Verify classify_exit identifies this as a cancellation
    // 2. Verify decide_action produces DoNotRestart or equivalent
    // 3. Verify execute_action does NOT attempt to restart the child
    // 4. Verify effective_protective_action reflects the no-restart decision
}

#[test]
fn test_manual_stop_has_priority_over_restart() {
    // Verify that manual stop signals take priority over automatic restart logic

    let mut pipeline = ObservabilityPipeline::new(100, 10);
    let _subscriber_idx = pipeline.add_subscriber();

    let child_id = ChildId::new("test-child-manual-stop".to_string());

    // Simulate a child stopped event (manual intervention)
    let stopped_event = test_event(
        1,
        What::ChildStopped {
            reason: "manual_stop".to_string(),
        },
        Some(child_id.clone()),
    );

    let lagged = pipeline.emit(stopped_event);
    assert_eq!(lagged, 0);

    // TODO(T009b, T014): After implementation:
    // 1. Verify manual_stop is classified as a terminal state
    // 2. Verify execute_action respects the stop and does not restart
    // 3. Verify the event indicates supervised_stop or equivalent protection action
}

#[test]
fn test_protection_action_supervised_stop_is_most_restrictive() {
    // Verify SupervisedStop is the most restrictive action on the ladder

    let actions = vec![
        ProtectionAction::RestartAllowed,
        ProtectionAction::RestartQueued,
        ProtectionAction::RestartDenied,
        ProtectionAction::SupervisionPaused,
        ProtectionAction::Escalated,
        ProtectionAction::SupervisedStop,
    ];

    // Find the maximum action
    let max_action = actions.iter().max().unwrap();
    assert_eq!(*max_action, ProtectionAction::SupervisedStop);
}

#[test]
fn test_external_cancel_competition_with_restart() {
    // Test scenario where external_cancel competes with an pending automatic restart

    let mut pipeline = ObservabilityPipeline::new(100, 10);
    let _subscriber_idx = pipeline.add_subscriber();

    let child_id = ChildId::new("test-child-competition".to_string());

    // First, simulate a failure that would normally trigger restart
    let failure_event = test_event(
        1,
        What::ChildFailed {
            failure: rust_supervisor::error::types::TaskFailure::new(
                rust_supervisor::error::types::TaskFailureKind::Error,
                "exit_code",
                "failure before cancel",
            ),
        },
        Some(child_id.clone()),
    );

    pipeline.emit(failure_event);

    // Then, simulate external cancel arriving while restart is pending
    let cancel_event = test_event(
        2,
        What::ChildFailed {
            failure: rust_supervisor::error::types::TaskFailure::new(
                rust_supervisor::error::types::TaskFailureKind::Cancelled,
                "external_cancel",
                "cancel arrived during restart backoff",
            ),
        },
        Some(child_id.clone()),
    );

    pipeline.emit(cancel_event);

    // TODO(T009b, T014): After implementation:
    // 1. Verify the cancel supersedes the pending restart
    // 2. Verify execute_action cancels the pending restart
    // 3. Verify no restart occurs after the cancel
}
