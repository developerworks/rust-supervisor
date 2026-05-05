//! Supervisor shutdown integration tests.
//!
//! These tests cover four-stage shutdown visibility through the handle.

use rust_supervisor::control::command::CommandResult;
use rust_supervisor::runtime::supervisor::Supervisor;
use rust_supervisor::shutdown::stage::ShutdownPhase;
use rust_supervisor::spec::supervisor::SupervisorSpec;

/// Verifies that shutdown completes and current state reflects completion.
#[tokio::test]
async fn shutdown_tree_completes_without_orphaned_runtime_state() {
    let handle = Supervisor::start(SupervisorSpec::root(Vec::new()))
        .await
        .expect("start supervisor");
    let shutdown = handle
        .shutdown_tree("operator", "test shutdown")
        .await
        .expect("shutdown tree");

    assert!(matches!(
        shutdown,
        CommandResult::Shutdown {
            result: rust_supervisor::shutdown::coordinator::ShutdownResult {
                phase: ShutdownPhase::RequestStop,
                ..
            }
        }
    ));

    let current = handle.current_state().await.expect("current state");
    assert!(matches!(
        current,
        CommandResult::CurrentState {
            state: rust_supervisor::control::command::CurrentState {
                shutdown_completed: true,
                ..
            }
        }
    ));
}
