//! Invalid configuration rejection tests.

use rust_supervisor::config::yaml::parse_config_state;
use rust_supervisor::error::types::SupervisorError;
use rust_supervisor::spec::child::TaskKind;
use rust_supervisor::task::factory::{TaskResult, service_fn};
use rust_supervisor::task::factory_registry::{TaskFactoryDescriptor, TaskFactoryRegistry};
use std::sync::Arc;

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

/// Returns a valid YAML document with one declarative worker child.
fn worker_yaml(factory_key_line: &str) -> String {
    format!(
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
children:
  - name: worker
    kind: async_worker
{factory_key_line}
"#
    )
}

/// Builds a registry with one async worker factory.
fn registry() -> TaskFactoryRegistry {
    let mut registry = TaskFactoryRegistry::new();
    registry
        .register(TaskFactoryDescriptor::new(
            "worker_factory",
            "Worker Factory",
            "Runs a worker.",
            [TaskKind::AsyncWorker],
            Arc::new(service_fn(|_ctx| async { TaskResult::Succeeded })),
        ))
        .expect("register worker factory");
    registry
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

/// Verifies that worker factory binding rejects missing factory_key values.
#[test]
fn worker_factory_binding_rejects_missing_factory_key() {
    let yaml = worker_yaml("");
    let state = parse_config_state(&yaml).expect("config load should allow declarative worker");
    let result = state
        .to_supervisor_spec_with_factories(&registry())
        .map(|_| ());

    assert_fatal_config(result, "requires factory_key");
}

/// Verifies that worker factory binding rejects unknown factory keys.
#[test]
fn worker_factory_binding_rejects_unknown_factory_key() {
    let yaml = worker_yaml("    factory_key: missing_factory");
    let state = parse_config_state(&yaml).expect("config load should allow factory key");
    let result = state
        .to_supervisor_spec_with_factories(&registry())
        .map(|_| ());

    assert_fatal_config(result, "unknown task factory key");
}

/// Verifies that supervisor children cannot declare factory_key values.
#[test]
fn supervisor_child_rejects_factory_key_during_binding() {
    let yaml = worker_yaml("    factory_key: worker_factory")
        .replace("kind: async_worker", "kind: supervisor");
    let state = parse_config_state(&yaml).expect("config load should allow supervisor child");
    let result = state
        .to_supervisor_spec_with_factories(&registry())
        .map(|_| ());

    assert_fatal_config(result, "must not declare factory_key");
}

/// Verifies that worker factory binding succeeds for a registered key.
#[test]
fn worker_factory_binding_accepts_registered_factory_key() {
    let yaml = worker_yaml("    factory_key: worker_factory");
    let state = parse_config_state(&yaml).expect("config load should allow factory key");
    let spec = state
        .to_supervisor_spec_with_factories(&registry())
        .expect("factory binding should succeed");

    assert_eq!(spec.children.len(), 1);
    assert!(spec.children[0].factory.is_some());
}
