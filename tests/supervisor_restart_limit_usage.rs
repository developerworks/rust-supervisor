//! Acceptance tests for restart_execution_plan restart_limit usage in evaluate budget stage.
//!
//! This test verifies that:
//! 1. When restart_execution_plan carries restart_limit, the evaluate budget stage reads it
//! 2. The restart_limit affects the final disposition decision
//! 3. Budget exhaustion triggers appropriate protection actions

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
fn test_protection_action_order() {
    // Verify the protection restrictiveness ladder ordering
    // restart_allowed < restart_queued < restart_denied < supervision_paused < escalated < supervised_stop

    assert!(ProtectionAction::RestartAllowed < ProtectionAction::RestartQueued);
    assert!(ProtectionAction::RestartQueued < ProtectionAction::RestartDenied);
    assert!(ProtectionAction::RestartDenied < ProtectionAction::SupervisionPaused);
    assert!(ProtectionAction::SupervisionPaused < ProtectionAction::Escalated);
    assert!(ProtectionAction::Escalated < ProtectionAction::SupervisedStop);
}

#[test]
fn test_restart_limit_affects_budget_evaluation() {
    // This test will initially pass with basic structure but needs full implementation
    // to verify actual budget evaluation logic

    let mut pipeline = ObservabilityPipeline::new(100, 10);
    let _subscriber_idx = pipeline.add_subscriber();

    let child_id = ChildId::new("test-child-limit".to_string());

    // Simulate a failure event
    let failure_event = test_event(
        1,
        What::ChildFailed {
            failure: rust_supervisor::error::types::TaskFailure::new(
                rust_supervisor::error::types::TaskFailureKind::Error,
                "exit_code",
                "restart limit test",
            ),
        },
        Some(child_id.clone()),
    );

    // Emit through pipeline
    let lagged = pipeline.emit(failure_event);
    assert_eq!(lagged, 0);

    // TODO(T009, T012): After evaluate budget implementation:
    // 1. Create a restart_execution_plan with restart_limit = 3
    // 2. Trigger 3 consecutive failures
    // 3. Verify the 4th failure gets restart_denied or higher protection action
    // 4. Verify the event's effective_protective_action field reflects the budget state
}

#[test]
fn test_escalation_policy_in_budget_evaluation() {
    // Test that escalation_policy from restart_execution_plan is consumed by evaluate budget stage

    let mut pipeline = ObservabilityPipeline::new(100, 10);
    let _subscriber_idx = pipeline.add_subscriber();

    let child_id = ChildId::new("test-child-escalation".to_string());

    // Simulate a fatal config error that should trigger escalation
    let fatal_event = test_event(
        1,
        What::ChildFailed {
            failure: rust_supervisor::error::types::TaskFailure::new(
                rust_supervisor::error::types::TaskFailureKind::Error,
                "fatal_config",
                "configuration error requiring escalation",
            ),
        },
        Some(child_id.clone()),
    );

    let lagged = pipeline.emit(fatal_event);
    assert_eq!(lagged, 0);

    // TODO(T009, T012): After implementation:
    // 1. Verify escalation_policy is read from restart_execution_plan
    // 2. Verify fatal errors trigger Escalated or SupervisedStop action
    // 3. Verify the event's effective_protective_action reflects escalation
}

#[test]
fn test_event_contains_protective_action_field() {
    // Verify that SupervisorEvent has the effective_protective_action field

    let child_id = ChildId::new("test-child-action".to_string());
    let event = test_event(1, What::ChildRunning { transition: None }, Some(child_id));

    // Verify the field exists and has default value
    assert_eq!(event.effective_protective_action, None);

    // TODO(T009): After implementation, verify:
    // 1. After pipeline processing, effective_protective_action is populated
    // 2. The action matches the decided protection level
}
