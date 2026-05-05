//! Supervisor startup integration tests.
//!
//! These tests verify that the runtime handle can start from a valid spec.

use rust_supervisor::control::command::CommandResult;
use rust_supervisor::runtime::supervisor::Supervisor;
use rust_supervisor::spec::supervisor::SupervisorSpec;

/// Verifies that an empty supervisor starts and answers current state.
#[tokio::test]
async fn supervisor_start_returns_control_handle() {
    let handle = Supervisor::start(SupervisorSpec::root(Vec::new()))
        .await
        .expect("start supervisor");
    let current = handle.current_state().await.expect("current state");

    match current {
        CommandResult::CurrentState { state } => {
            assert_eq!(state.child_count, 0);
            assert!(!state.shutdown_completed);
        }
        other => panic!("unexpected result: {other:?}"),
    }
}
