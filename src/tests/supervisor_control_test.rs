//! Supervisor control integration tests.
//!
//! These tests verify idempotent command handling through the public handle.

use rust_supervisor::control::command::{CommandResult, ManagedChildState};
use rust_supervisor::error::types::SupervisorError;
use rust_supervisor::id::types::{ChildId, SupervisorPath};
use rust_supervisor::runtime::supervisor::Supervisor;
use rust_supervisor::shutdown::report::ResourceReconcileStatus;
use rust_supervisor::shutdown::stage::ShutdownPhase;
use rust_supervisor::spec::supervisor::SupervisorSpec;

/// Verifies that control commands mutate runtime state.
#[tokio::test]
async fn control_commands_update_child_state() {
    let handle = Supervisor::start(SupervisorSpec::root(Vec::new()))
        .await
        .expect("start supervisor");
    let child_id = ChildId::new("worker");

    let added = handle
        .add_child(
            SupervisorPath::root(),
            "worker manifest",
            "operator",
            "test",
        )
        .await
        .expect("add child");
    assert!(matches!(added, CommandResult::ChildAdded { .. }));

    let paused = handle
        .pause_child(child_id.clone(), "operator", "test")
        .await
        .expect("pause child");
    assert!(matches!(
        paused,
        CommandResult::ChildState {
            state: ManagedChildState::Paused,
            ..
        }
    ));

    let repeated = handle
        .pause_child(child_id, "operator", "test")
        .await
        .expect("pause child again");
    assert!(matches!(
        repeated,
        CommandResult::ChildState {
            idempotent: true,
            ..
        }
    ));
}

/// Verifies that control commands require auditable metadata.
#[tokio::test]
async fn control_commands_reject_empty_audit_metadata() {
    let handle = Supervisor::start(SupervisorSpec::root(Vec::new()))
        .await
        .expect("start supervisor");
    let child_id = ChildId::new("worker");

    let missing_actor = handle
        .pause_child(child_id, " ", "maintenance window")
        .await;
    assert_invalid_transition(missing_actor, "requested_by");

    let missing_reason = handle.shutdown_tree("operator", "\t").await;
    assert_invalid_transition(missing_reason, "reason");
}

/// Verifies that shutdown control command returns a completed report.
#[tokio::test]
async fn shutdown_tree_returns_completed_report() {
    let handle = Supervisor::start(SupervisorSpec::root(Vec::new()))
        .await
        .expect("start supervisor");

    let result = handle
        .shutdown_tree("operator", "control regression")
        .await
        .expect("shutdown tree");

    match result {
        CommandResult::Shutdown { result } => {
            assert_eq!(result.phase, ShutdownPhase::Completed);
            let report = result.report.expect("shutdown report should exist");
            assert_eq!(report.phase, ShutdownPhase::Completed);
            assert_eq!(
                report.reconcile.socket_status,
                ResourceReconcileStatus::NotOwned
            );
        }
        other => panic!("unexpected command result: {other:?}"),
    }
}

/// Asserts that a command returned the expected invalid transition field.
fn assert_invalid_transition(result: Result<CommandResult, SupervisorError>, expected_field: &str) {
    match result {
        Err(SupervisorError::InvalidTransition { message }) => {
            assert!(message.contains(expected_field), "{message}");
        }
        other => panic!("unexpected command result: {other:?}"),
    }
}
