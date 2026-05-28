//! Supervisor shutdown integration tests.
//!
//! These tests cover four-stage shutdown visibility through the handle.
//!
//! Shutdown uses Tokio paused runtime and
//! [`with_auto_clock_drive`](rust_supervisor::test_support::test_time::with_auto_clock_drive)
//! per `SC-010`.

use rust_supervisor::control::command::CommandResult;
use rust_supervisor::runtime::supervisor::Supervisor;
use rust_supervisor::shutdown::stage::ShutdownPhase;
use rust_supervisor::spec::supervisor::SupervisorSpec;
use rust_supervisor::test_support::test_time::with_auto_clock_drive;

/// Verifies that shutdown completes and current state reflects completion.
#[tokio::test(start_paused = true)]
async fn shutdown_tree_completes_without_orphaned_runtime_state() {
    let handle = Supervisor::start(SupervisorSpec::root(Vec::new()))
        .await
        .expect("start supervisor");
    let shutdown = with_auto_clock_drive(handle.shutdown_tree("operator", "test shutdown"))
        .await
        .expect("shutdown tree");

    assert!(matches!(
        shutdown,
        CommandResult::Shutdown {
            result: rust_supervisor::shutdown::coordinator::ShutdownResult {
                phase: ShutdownPhase::Completed,
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
