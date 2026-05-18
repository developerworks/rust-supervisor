//! Supervisor control integration tests.
//!
//! These tests verify idempotent command handling through the public handle.

use rust_supervisor::control::command::CommandResult;
use rust_supervisor::control::outcome::{
    ChildAttemptStatus, ChildControlOperation, ChildStopState,
};
use rust_supervisor::error::types::SupervisorError;
use rust_supervisor::id::types::{ChildId, SupervisorPath};
use rust_supervisor::runtime::supervisor::Supervisor;
use rust_supervisor::shutdown::report::ResourceReconcileStatus;
use rust_supervisor::shutdown::stage::ShutdownPhase;
use rust_supervisor::spec::child::{ChildSpec, TaskKind};
use rust_supervisor::spec::supervisor::SupervisorSpec;
use rust_supervisor::task::context::TaskContext;
use rust_supervisor::task::factory::{TaskFactory, TaskResult, service_fn};
use std::sync::Arc;
use tokio::sync::mpsc;

/// Verifies that control commands mutate runtime state.
#[tokio::test]
async fn control_commands_update_child_state() {
    let handle = Supervisor::start(SupervisorSpec::root(Vec::new()))
        .await
        .expect("start supervisor");
    let child_id = ChildId::new("worker");

    let manifest = "name: worker\nkind: async_worker\n";
    let added = handle
        .add_child(SupervisorPath::root(), manifest, "operator", "test")
        .await
        .expect("add child");
    assert!(matches!(added, CommandResult::ChildAdded { .. }));

    let paused = handle
        .pause_child(child_id.clone(), "operator", "test")
        .await
        .expect("pause child");
    assert!(matches!(
        paused,
        CommandResult::ChildControl {
            outcome
        } if outcome.operation_after == ChildControlOperation::Paused
            && !outcome.idempotent
    ));

    let repeated = handle
        .pause_child(child_id, "operator", "test")
        .await
        .expect("pause child again");
    assert!(matches!(
        repeated,
        CommandResult::ChildControl {
            outcome
        } if outcome.idempotent
    ));
}

/// Verifies that the old child-state command result shape is gone.
#[tokio::test]
async fn child_state_result_variant_is_replaced_by_child_control_test() {
    let handle = Supervisor::start(SupervisorSpec::root(Vec::new()))
        .await
        .expect("start supervisor");

    let result = handle
        .pause_child(ChildId::new("worker"), "operator", "pause")
        .await
        .expect("pause child");
    let value = serde_json::to_value(&result).expect("serialize result");

    assert!(matches!(result, CommandResult::ChildControl { .. }));
    assert!(value.get("ChildControl").is_some(), "{value:?}");
    assert!(value.get("ChildState").is_none(), "{value:?}");
}

/// Verifies that child control results expose runtime state identity.
#[tokio::test]
async fn child_control_result_contains_runtime_state_identity_test() {
    let (started_sender, mut started_receiver) = mpsc::channel(1);
    let child_id = ChildId::new("worker");
    let spec = SupervisorSpec::root(vec![worker_child(
        "worker",
        service_fn(move |ctx: TaskContext| {
            let started_sender = started_sender.clone();
            async move {
                let _ignored = started_sender.send(ctx.child_id.value.clone()).await;
                ctx.cancellation_token().cancelled().await;
                TaskResult::Cancelled
            }
        }),
    )]);
    let handle = Supervisor::start(spec).await.expect("start supervisor");
    started_receiver.recv().await.expect("child should start");

    let result = handle
        .pause_child(child_id.clone(), "operator", "pause")
        .await
        .expect("pause child");

    let outcome = match result {
        CommandResult::ChildControl { outcome } => outcome,
        other => panic!("unexpected pause result: {other:?}"),
    };
    assert_eq!(outcome.child_id, child_id);
    assert_eq!(outcome.generation.expect("generation").value, 0);
    assert_eq!(outcome.attempt.expect("attempt").value, 1);
    assert_eq!(outcome.operation_after, ChildControlOperation::Paused);
    assert_eq!(outcome.status, Some(ChildAttemptStatus::Cancelling));
    assert_eq!(outcome.stop_state, ChildStopState::CancelDelivered);

    let _shutdown = handle
        .shutdown_tree("test", "finish control identity test")
        .await
        .expect("shutdown supervisor");
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

/// Creates a worker child from a task factory.
fn worker_child(name: &'static str, factory: impl TaskFactory) -> ChildSpec {
    ChildSpec::worker(
        ChildId::new(name),
        name,
        TaskKind::AsyncWorker,
        Arc::new(factory),
    )
}
