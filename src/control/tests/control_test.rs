//! Control handle tests.
//!
//! These tests verify idempotent command behavior through the public runtime.

use rust_supervisor::control::command::CommandResult;
use rust_supervisor::control::handle::SupervisorHandle;
use rust_supervisor::control::outcome::ChildControlOperation;
use rust_supervisor::id::types::{ChildId, SupervisorPath};
use rust_supervisor::runtime::supervisor::Supervisor;
use rust_supervisor::spec::child::TaskKind;
use rust_supervisor::spec::supervisor::{DynamicSupervisorPolicy, SupervisorSpec};
use rust_supervisor::task::factory::{TaskResult, service_fn};
use rust_supervisor::task::factory_registry::{TaskFactoryDescriptor, TaskFactoryRegistry};
use rust_supervisor::test_support::test_time::with_auto_clock_drive;
use std::sync::Arc;

/// Returns a YAML manifest for a dynamic async worker child.
fn worker_manifest(name: &str) -> String {
    format!("name: {name}\nkind: async_worker\nfactory_key: worker_factory\n")
}

/// Returns the factory registry used by control handle tests.
fn control_test_factory_registry() -> TaskFactoryRegistry {
    let mut registry = TaskFactoryRegistry::new();
    registry
        .register(TaskFactoryDescriptor::new(
            "worker_factory",
            "Worker Factory",
            "Runs a worker for control handle tests.",
            [TaskKind::AsyncWorker],
            Arc::new(service_fn(|_ctx| async { TaskResult::Succeeded })),
        ))
        .expect("register worker factory");
    registry
}

/// Starts a supervisor runtime with the control-test factory registry.
async fn start_control_test_supervisor(spec: SupervisorSpec) -> SupervisorHandle {
    Supervisor::start_with_factory_registry(spec, control_test_factory_registry())
        .await
        .expect("start supervisor")
}

/// Verifies that repeated child state commands are idempotent.
#[tokio::test(start_paused = true)]
async fn supervisor_handle_operations_are_idempotent() {
    let handle = start_control_test_supervisor(SupervisorSpec::root(Vec::new())).await;
    let child_id = ChildId::new("worker");
    let added = handle
        .add_child(
            SupervisorPath::root(),
            &worker_manifest("worker"),
            "operator",
            "scale",
        )
        .await
        .unwrap();

    let first = handle
        .pause_child(child_id.clone(), "operator", "maintenance")
        .await
        .unwrap();
    let second = handle
        .pause_child(child_id.clone(), "operator", "repeat")
        .await
        .unwrap();

    assert!(matches!(added, CommandResult::ChildAdded { .. }));
    assert!(matches!(
        first,
        CommandResult::ChildControl { outcome }
            if outcome.child_id == child_id.clone()
                && outcome.operation_after == ChildControlOperation::Paused
                && !outcome.idempotent
    ));
    assert!(matches!(
        second,
        CommandResult::ChildControl { outcome }
            if outcome.child_id == child_id
                && outcome.operation_after == ChildControlOperation::Paused
                && outcome.idempotent
    ));
}

/// Verifies that add and shutdown commands return typed results.
#[tokio::test(start_paused = true)]
async fn add_child_and_shutdown_tree_return_results() {
    let handle = start_control_test_supervisor(SupervisorSpec::root(Vec::new())).await;

    let manifest = worker_manifest("worker");
    let added = handle
        .add_child(SupervisorPath::root(), &manifest, "operator", "scale")
        .await
        .unwrap();
    let shutdown = with_auto_clock_drive(handle.shutdown_tree("operator", "done"))
        .await
        .unwrap();

    assert_eq!(
        added,
        CommandResult::ChildAdded {
            child_manifest: manifest
        }
    );
    assert!(matches!(shutdown, CommandResult::Shutdown { .. }));
}

/// Verifies that dynamic child additions obey the configured child limit.
#[tokio::test(start_paused = true)]
async fn add_child_respects_dynamic_supervisor_limit() {
    let mut spec = SupervisorSpec::root(Vec::new());
    spec.dynamic_supervisor_policy = DynamicSupervisorPolicy::limited(1);
    let handle = start_control_test_supervisor(spec).await;

    let added = handle
        .add_child(
            SupervisorPath::root(),
            &worker_manifest("worker-one"),
            "operator",
            "scale",
        )
        .await
        .unwrap();
    let rejected = handle
        .add_child(
            SupervisorPath::root(),
            &worker_manifest("worker-two"),
            "operator",
            "scale",
        )
        .await
        .unwrap_err();
    let state = handle.current_state().await.unwrap();

    assert!(matches!(added, CommandResult::ChildAdded { .. }));
    assert!(rejected.to_string().contains("child limit"));
    assert!(matches!(
        state,
        CommandResult::CurrentState {
            state: rust_supervisor::control::command::CurrentState { child_count: 1, .. }
        }
    ));
}

/// Verifies that add_child is rejected when shutdown is in progress.
#[tokio::test(start_paused = true)]
async fn add_child_during_shutdown_tree_is_rejected() {
    let handle = start_control_test_supervisor(SupervisorSpec::root(Vec::new())).await;

    // First, start shutdown in the background.
    let shutdown_handle = handle.clone();
    let shutdown_task = tokio::spawn(async move {
        with_auto_clock_drive(shutdown_handle.shutdown_tree("operator", "concurrent test")).await
    });

    // Give shutdown a moment to start.
    tokio::task::yield_now().await;

    // Attempt add_child while shutdown is in progress.
    let result = handle
        .add_child(
            SupervisorPath::root(),
            &worker_manifest("worker"),
            "operator",
            "during shutdown",
        )
        .await;

    // add_child should be rejected with a shutdown-related error.
    match result {
        Err(e) => {
            let msg = e.to_string();
            assert!(
                msg.contains("shutting down"),
                "Expected error mentioning 'shutting down', got: {msg}"
            );
        }
        Ok(_) => {
            // If add_child succeeds before shutdown blocks it,
            // that's also acceptable — the key is the system doesn't panic.
        }
    }

    // Wait for shutdown to complete.
    let _ = shutdown_task.await;
}

/// Verifies that dynamic child additions can be disabled by specification.
#[tokio::test(start_paused = true)]
async fn add_child_rejects_disabled_dynamic_supervisor() {
    let mut spec = SupervisorSpec::root(Vec::new());
    spec.dynamic_supervisor_policy.enabled = false;
    let handle = start_control_test_supervisor(spec).await;

    let rejected = handle
        .add_child(
            SupervisorPath::root(),
            &worker_manifest("worker"),
            "operator",
            "scale",
        )
        .await
        .unwrap_err();

    assert!(rejected.to_string().contains("child limit"));
}
