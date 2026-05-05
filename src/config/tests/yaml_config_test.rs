//! Tests for YAML configuration loading and validation.

use rust_supervisor::config::yaml::parse_config_state;
use rust_supervisor::spec::supervisor::SupervisionStrategy;

/// Returns a valid YAML configuration document.
/// Returns a valid YAML configuration document for parser tests.
fn valid_yaml() -> &'static str {
    r#"
supervisor:
  strategy: RestForOne
policy:
  child_restart_limit: 10
  child_restart_window_ms: 60000
  supervisor_failure_limit: 30
  supervisor_failure_window_ms: 60000
  initial_backoff_ms: 10
  max_backoff_ms: 1000
  jitter_ratio: 0.0
  heartbeat_interval_ms: 1000
  stale_after_ms: 3000
shutdown:
  graceful_timeout_ms: 1000
  abort_wait_ms: 100
observability:
  event_journal_capacity: 64
  metrics_enabled: true
  audit_enabled: true
"#
}

/// Verifies that all required runtime tunables load from YAML.
/// Verifies that YAML loading fills every required runtime tunable.
#[test]
fn yaml_config_loads_required_runtime_tunables() {
    let state = parse_config_state(valid_yaml()).expect("valid YAML should load");

    assert_eq!(state.supervisor.strategy, SupervisionStrategy::RestForOne);
    assert_eq!(state.policy.child_restart_limit, 10);
    assert_eq!(state.policy.supervisor_failure_limit, 30);
    assert_eq!(state.shutdown.graceful_timeout_ms, 1000);
    assert_eq!(state.observability.event_journal_capacity, 64);
}

/// Verifies that missing runtime tunables are rejected.
/// Verifies that missing required YAML values are rejected.
#[test]
fn yaml_config_rejects_missing_required_tunables() {
    let result = parse_config_state("policy: {}\n");

    assert!(result.is_err());
}

/// Verifies that invalid backoff ranges are rejected.
/// Verifies that the initial backoff cannot exceed the maximum backoff.
#[test]
fn yaml_config_rejects_invalid_backoff_range() {
    let yaml = valid_yaml().replace("initial_backoff_ms: 10", "initial_backoff_ms: 2000");
    let result = parse_config_state(&yaml);

    assert!(result.is_err());
}

/// Verifies that unknown supervision strategies are rejected.
/// Verifies that unknown supervision strategy names are rejected.
#[test]
fn yaml_config_rejects_invalid_supervision_strategy() {
    let yaml = valid_yaml().replace("strategy: RestForOne", "strategy: RestartEverything");
    let result = parse_config_state(&yaml);

    assert!(result.is_err());
}
