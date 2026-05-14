//! Runtime lifecycle integration tests.
//!
//! These tests verify that `SupervisorHandle` exposes stable control-plane
//! state after startup, failure, and shutdown.

use rust_supervisor::control::command::CommandResult;
use rust_supervisor::error::types::SupervisorError;
use rust_supervisor::runtime::lifecycle::{
    RuntimeControlPlaneState, RuntimeExitReport, RuntimeHealthReport,
};
use rust_supervisor::runtime::supervisor::Supervisor;
use rust_supervisor::spec::supervisor::SupervisorSpec;
use rust_supervisor::test_support::factory::runtime_control_plane_failed_handle;
use tokio::sync::broadcast;
use tokio::time::{Duration, timeout};

/// Verifies that a supervisor reports alive immediately after startup.
#[tokio::test]
async fn supervisor_reports_alive_after_start() {
    let handle = start_empty_supervisor().await;

    assert!(handle.is_alive());
}

/// Verifies that health includes startup time and last observation time.
#[tokio::test]
async fn supervisor_health_reports_alive_timestamps_after_start() {
    let handle = start_empty_supervisor().await;
    let health = handle.health();

    assert_alive_health(&health);
}

/// Verifies that subscribers can receive a control loop started event.
#[tokio::test]
async fn supervisor_emits_runtime_control_loop_started_event() {
    let handle = start_empty_supervisor().await;
    let mut receiver = handle.subscribe_events();

    let event = receive_event(&mut receiver).await;

    assert!(event.contains("runtime_control_loop_started"), "{event}");
}

/// Verifies that health reports a failed reason after abnormal loop exit.
#[tokio::test]
async fn supervisor_health_reports_failed_control_loop() {
    let handle = runtime_control_plane_failed_handle().await;
    let health = handle.health();

    assert!(!health.alive);
    assert_eq!(health.state, RuntimeControlPlaneState::Failed);
    let failure = health.failure.expect("failure reason");
    assert_eq!(failure.phase, "watchdog");
    assert!(failure.reason.contains("panic"), "{}", failure.reason);
    assert!(failure.panic);
    assert!(failure.recoverable);
}

/// Verifies that commands after control loop exit report the known reason.
#[tokio::test]
async fn supervisor_command_after_control_loop_exit_reports_known_reason() {
    let handle = runtime_control_plane_failed_handle().await;

    let result = handle.current_state().await;

    assert_invalid_transition_contains(result, "watchdog");
    let result = handle.current_state().await;
    assert_invalid_transition_contains(result, "panic");
}

/// Verifies that shutdown completes the control plane normally.
#[tokio::test]
async fn supervisor_shutdown_completes_runtime_control_plane() {
    let handle = start_empty_supervisor().await;

    let report = handle
        .shutdown("operator", "test shutdown")
        .await
        .expect("shutdown control plane");

    assert_completed_report(&report);
    assert!(!handle.is_alive());
}

/// Verifies that repeated join calls do not hang and return the same result.
#[tokio::test]
async fn supervisor_join_returns_cached_exit_report_repeatedly() {
    let handle = start_empty_supervisor().await;
    let expected = handle
        .shutdown("operator", "repeat join")
        .await
        .expect("shutdown control plane");

    for _index in 0..10 {
        let report = timeout(Duration::from_secs(1), handle.join())
            .await
            .expect("join should not hang")
            .expect("join control plane");
        assert_eq!(report, expected);
    }
}

/// Verifies that repeated shutdown returns the cached final result.
#[tokio::test]
async fn supervisor_shutdown_after_completion_returns_cached_exit_report() {
    let handle = start_empty_supervisor().await;
    let first = handle
        .shutdown("operator", "first shutdown")
        .await
        .expect("first shutdown");
    let second = handle
        .shutdown("operator", "second shutdown")
        .await
        .expect("second shutdown");

    assert_eq!(second, first);
}

/// Starts an empty supervisor.
async fn start_empty_supervisor() -> rust_supervisor::control::handle::SupervisorHandle {
    Supervisor::start(SupervisorSpec::root(Vec::new()))
        .await
        .expect("start supervisor")
}

/// Receives one text event.
async fn receive_event(receiver: &mut broadcast::Receiver<String>) -> String {
    timeout(Duration::from_secs(1), receiver.recv())
        .await
        .expect("event timeout")
        .expect("event receive")
}

/// Asserts that health reports alive.
fn assert_alive_health(health: &RuntimeHealthReport) {
    assert!(health.alive);
    assert_eq!(health.state, RuntimeControlPlaneState::Alive);
    assert!(health.started_at_unix_nanos > 0);
    assert!(health.last_observed_at_unix_nanos >= health.started_at_unix_nanos);
    assert!(health.failure.is_none());
    assert!(health.exit_report.is_none());
}

/// Asserts that a report is completed.
fn assert_completed_report(report: &RuntimeExitReport) {
    assert_eq!(report.state, RuntimeControlPlaneState::Completed);
    assert_eq!(report.phase, "shutdown");
    assert!(!report.reason.trim().is_empty());
    assert!(!report.recoverable);
    assert!(report.completed_at_unix_nanos > 0);
}

/// Asserts that a command returns invalid transition with expected text.
fn assert_invalid_transition_contains(
    result: Result<CommandResult, SupervisorError>,
    expected: &str,
) {
    match result {
        Err(SupervisorError::InvalidTransition { message }) => {
            assert!(message.contains(expected), "{message}");
        }
        other => panic!("unexpected command result: {other:?}"),
    }
}
