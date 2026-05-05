//! Supervisor control integration tests.
//!
//! These tests verify idempotent command handling through the public handle.

use rust_supervisor::control::command::{CommandResult, ManagedChildState};
use rust_supervisor::id::types::{ChildId, SupervisorPath};
use rust_supervisor::runtime::supervisor::Supervisor;
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
