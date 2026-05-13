//! Supervisor configuration integration tests.
//!
//! These tests verify that validated configuration can drive supervisor startup.

use rust_supervisor::config::loader::load_config_state;
use rust_supervisor::config::yaml::parse_config_state;
use rust_supervisor::error::types::SupervisorError;
use rust_supervisor::runtime::supervisor::Supervisor;
use rust_supervisor::spec::supervisor::SupervisionStrategy;
use std::fs;
use std::path::Path;

/// Verifies that the example YAML configuration can produce a running handle.
#[tokio::test]
async fn yaml_config_derives_startable_supervisor_spec() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let state =
        load_config_state(root.join("examples/config/supervisor.yaml")).expect("load YAML config");
    let spec = state.to_supervisor_spec().expect("derive supervisor spec");
    assert_eq!(spec.strategy, SupervisionStrategy::OneForAll);
    let handle = Supervisor::start(spec).await.expect("start supervisor");

    let current = handle.current_state().await.expect("current state");
    assert!(matches!(
        current,
        rust_supervisor::control::command::CommandResult::CurrentState { .. }
    ));
}

/// Verifies that validated configuration state can start a supervisor runtime.
#[tokio::test]
async fn supervisor_starts_from_config_state() {
    let state = no_ipc_startup_state();
    let handle = Supervisor::start_from_config_state(state)
        .await
        .expect("start from config state");

    let current = handle.current_state().await.expect("current state");
    assert!(matches!(
        current,
        rust_supervisor::control::command::CommandResult::CurrentState { .. }
    ));
}

/// Verifies that YAML file configuration can start a supervisor runtime.
#[tokio::test]
async fn supervisor_starts_from_config_file() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("target/no-ipc-supervisor-config.yaml");
    fs::create_dir_all(path.parent().expect("target parent")).expect("create target");
    fs::write(&path, no_ipc_startup_yaml()).expect("write config");
    let handle = Supervisor::start_from_config_file(&path)
        .await
        .expect("start from config file");
    let _ = fs::remove_file(&path);

    let current = handle.current_state().await.expect("current state");
    assert!(matches!(
        current,
        rust_supervisor::control::command::CommandResult::CurrentState { .. }
    ));
}

/// Verifies that invalid configuration state is rejected before startup.
#[tokio::test]
async fn invalid_config_state_does_not_return_handle() {
    let mut state = no_ipc_startup_state();
    state.observability.event_journal_capacity = 0;

    let result = Supervisor::start_from_config_state(state).await;

    assert!(matches!(
        result,
        Err(SupervisorError::FatalConfig { message }) if message.contains("channel capacity")
    ));
}

/// Verifies that invalid YAML file configuration is rejected before startup.
#[tokio::test]
async fn invalid_config_file_does_not_return_handle() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("target/invalid-supervisor-config.yaml");
    fs::create_dir_all(path.parent().expect("target parent")).expect("create target");
    fs::write(
        &path,
        r#"
supervisor:
  strategy: OneForAll
policy:
  child_restart_limit: 10
  child_restart_window_ms: 60000
  supervisor_failure_limit: 30
  supervisor_failure_window_ms: 60000
  initial_backoff_ms: 6000
  max_backoff_ms: 5000
  jitter_ratio: 0.10
  heartbeat_interval_ms: 1000
  stale_after_ms: 3000
shutdown:
  graceful_timeout_ms: 5000
  abort_wait_ms: 1000
observability:
  event_journal_capacity: 256
  metrics_enabled: true
  audit_enabled: true
"#,
    )
    .expect("write invalid config");

    let result = Supervisor::start_from_config_file(&path).await;
    let _ = fs::remove_file(&path);

    assert!(matches!(
        result,
        Err(SupervisorError::FatalConfig { message })
            if message.contains("policy.initial_backoff_ms")
    ));
}

/// Builds a valid configuration state without dashboard IPC side effects.
fn no_ipc_startup_state() -> rust_supervisor::config::state::ConfigState {
    parse_config_state(no_ipc_startup_yaml()).expect("parse no IPC config")
}

/// Returns a minimal valid supervisor YAML without dashboard IPC.
fn no_ipc_startup_yaml() -> &'static str {
    r#"
supervisor:
  strategy: OneForAll
policy:
  child_restart_limit: 10
  child_restart_window_ms: 60000
  supervisor_failure_limit: 30
  supervisor_failure_window_ms: 60000
  initial_backoff_ms: 100
  max_backoff_ms: 5000
  jitter_ratio: 0.10
  heartbeat_interval_ms: 1000
  stale_after_ms: 3000
shutdown:
  graceful_timeout_ms: 5000
  abort_wait_ms: 1000
observability:
  event_journal_capacity: 256
  metrics_enabled: true
  audit_enabled: true
"#
}
