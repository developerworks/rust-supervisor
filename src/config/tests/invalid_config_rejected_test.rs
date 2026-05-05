//! Invalid configuration rejection tests.

use rust_supervisor::config::yaml::parse_config_state;
use rust_supervisor::error::types::SupervisorError;

/// Returns a valid YAML configuration document for rejection tests.
fn valid_yaml() -> &'static str {
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

/// Asserts that parsing fails with a fatal configuration error.
fn assert_fatal_config(result: Result<(), SupervisorError>, expected: &str) {
    match result {
        Err(SupervisorError::FatalConfig { message }) => {
            assert!(
                message.contains(expected),
                "expected {expected:?} in {message:?}"
            );
        }
        other => panic!("expected FatalConfig, got {other:?}"),
    }
}

/// Verifies that missing required sections are rejected.
#[test]
fn missing_required_section_is_rejected() {
    let result = parse_config_state("policy: {}\n").map(|_| ());

    assert_fatal_config(result, "failed to parse YAML config");
}

/// Verifies that an invalid supervision strategy is rejected.
#[test]
fn invalid_enum_value_is_rejected() {
    let yaml = valid_yaml().replace("OneForAll", "RestartEverything");
    let result = parse_config_state(&yaml).map(|_| ());

    assert_fatal_config(result, "failed to parse YAML config");
}

/// Verifies that zero capacity values are rejected.
#[test]
fn zero_capacity_is_rejected() {
    let yaml = valid_yaml().replace("event_journal_capacity: 256", "event_journal_capacity: 0");
    let result = parse_config_state(&yaml).map(|_| ());

    assert_fatal_config(result, "observability.event_journal_capacity");
}

/// Verifies that zero timeout values are rejected.
#[test]
fn zero_timeout_is_rejected() {
    let yaml = valid_yaml().replace("graceful_timeout_ms: 5000", "graceful_timeout_ms: 0");
    let result = parse_config_state(&yaml).map(|_| ());

    assert_fatal_config(result, "shutdown.graceful_timeout_ms");
}

/// Verifies that jitter ratio values outside the allowed range are rejected.
#[test]
fn out_of_range_jitter_ratio_is_rejected() {
    let yaml = valid_yaml().replace("jitter_ratio: 0.10", "jitter_ratio: 1.20");
    let result = parse_config_state(&yaml).map(|_| ());

    assert_fatal_config(result, "policy.jitter_ratio");
}

/// Verifies that reverse backoff ranges are rejected.
#[test]
fn reversed_backoff_is_rejected() {
    let yaml = valid_yaml().replace("initial_backoff_ms: 100", "initial_backoff_ms: 6000");
    let result = parse_config_state(&yaml).map(|_| ());

    assert_fatal_config(result, "policy.initial_backoff_ms");
}
