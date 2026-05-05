//! Control handle tests.
//!
//! These tests verify idempotent command behavior through the public runtime.

use rust_supervisor::control::command::{CommandResult, ManagedChildState};
use rust_supervisor::id::types::{ChildId, SupervisorPath};
use rust_supervisor::runtime::supervisor::Supervisor;
use rust_supervisor::spec::supervisor::SupervisorSpec;

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

    assert_eq!(
        first,
        CommandResult::ChildState {
            child_id: child_id.clone(),
            state: ManagedChildState::Paused,
            idempotent: false
        }
    );
    assert_eq!(
        second,
        CommandResult::ChildState {
            child_id,
            state: ManagedChildState::Paused,
            idempotent: true
        }
    );
}

/// Verifies that add and shutdown commands return typed results.
#[tokio::test]
async fn add_child_and_shutdown_tree_return_results() {
    let handle = Supervisor::start(SupervisorSpec::root(Vec::new()))
        .await
        .unwrap();

    let added = handle
        .add_child(SupervisorPath::root(), "worker", "operator", "scale")
        .await
        .unwrap();
    let shutdown = handle.shutdown_tree("operator", "done").await.unwrap();

    assert_eq!(
        added,
        CommandResult::ChildAdded {
            child_manifest: "worker".to_owned()
        }
    );
    assert!(matches!(shutdown, CommandResult::Shutdown { .. }));
}
