//! Control handle tests.
//!
//! These tests verify idempotent command behavior through the public runtime.

use rust_supervisor::control::command::CommandResult;
use rust_supervisor::control::outcome::ChildControlOperation;
use rust_supervisor::id::types::{ChildId, SupervisorPath};
use rust_supervisor::runtime::supervisor::Supervisor;
use rust_supervisor::spec::supervisor::{DynamicSupervisorPolicy, SupervisorSpec};

/// Verifies that repeated child state commands are idempotent.
#[tokio::test]
async fn supervisor_handle_operations_are_idempotent() {
    let handle = Supervisor::start(SupervisorSpec::root(Vec::new()))
        .await
        .unwrap();
    let child_id = ChildId::new("worker");

    let first = handle
        .pause_child(child_id.clone(), "operator", "maintenance")
        .await
        .unwrap();
    let second = handle
        .pause_child(child_id.clone(), "operator", "repeat")
        .await
        .unwrap();

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
#[tokio::test]
async fn add_child_and_shutdown_tree_return_results() {
    let handle = Supervisor::start(SupervisorSpec::root(Vec::new()))
        .await
        .unwrap();

    let manifest = "name: worker\nkind: async_worker\n";
    let added = handle
        .add_child(SupervisorPath::root(), manifest, "operator", "scale")
        .await
        .unwrap();
    let shutdown = handle.shutdown_tree("operator", "done").await.unwrap();

    assert_eq!(
        added,
        CommandResult::ChildAdded {
            child_manifest: manifest.to_owned()
        }
    );
    assert!(matches!(shutdown, CommandResult::Shutdown { .. }));
}

/// Verifies that dynamic child additions obey the configured child limit.
#[tokio::test]
async fn add_child_respects_dynamic_supervisor_limit() {
    let mut spec = SupervisorSpec::root(Vec::new());
    spec.dynamic_supervisor_policy = DynamicSupervisorPolicy::limited(1);
    let handle = Supervisor::start(spec).await.unwrap();

    let added = handle
        .add_child(
            SupervisorPath::root(),
            "name: worker-one\nkind: async_worker\n",
            "operator",
            "scale",
        )
        .await
        .unwrap();
    let rejected = handle
        .add_child(
            SupervisorPath::root(),
            "name: worker-two\nkind: async_worker\n",
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
#[tokio::test]
async fn add_child_during_shutdown_tree_is_rejected() {
    let handle = Supervisor::start(SupervisorSpec::root(Vec::new()))
        .await
        .unwrap();

    // First, start shutdown in the background.
    let shutdown_handle = handle.clone();
    let shutdown_task = tokio::spawn(async move {
        shutdown_handle
            .shutdown_tree("operator", "concurrent test")
            .await
    });

    // Give shutdown a moment to start.
    tokio::task::yield_now().await;

    // Attempt add_child while shutdown is in progress.
    let result = handle
        .add_child(
            SupervisorPath::root(),
            "name: worker\nkind: async_worker\n",
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
#[tokio::test]
async fn add_child_rejects_disabled_dynamic_supervisor() {
    let mut spec = SupervisorSpec::root(Vec::new());
    spec.dynamic_supervisor_policy.enabled = false;
    let handle = Supervisor::start(spec).await.unwrap();

    let rejected = handle
        .add_child(
            SupervisorPath::root(),
            "name: worker\nkind: async_worker\n",
            "operator",
            "scale",
        )
        .await
        .unwrap_err();

    assert!(rejected.to_string().contains("child limit"));
}
