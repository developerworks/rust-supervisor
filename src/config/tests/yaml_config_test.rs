//! Tests for YAML configuration loading and validation.

use rust_supervisor::config::configurable::SupervisorConfig;
use rust_supervisor::config::state::ConfigState;
use rust_supervisor::config::yaml::parse_config_state;
use rust_supervisor::policy::failure_window::WindowMode;
use rust_supervisor::policy::group::PropagationPolicy;
use rust_supervisor::policy::task_role_defaults::{SeverityClass, TaskRole};
use rust_supervisor::spec::supervisor::{BackpressureStrategy, SupervisionStrategy};

/// Valid YAML configuration document for parser tests.
const VALID_YAML: &str = r#"
supervisor:
  strategy: RestForOne
  escalation_policy: escalate_to_parent
  dynamic_supervisor:
    enabled: true
    child_limit: 16
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
  restart_budget:
    window_secs: 45
    max_burst: 7
    recovery_rate_per_sec: 0.25
  failure_window:
    mode: time_sliding
    window_secs: 90
    max_count: 8
    threshold: 4
  meltdown:
    child_max_restarts: 4
    child_window_secs: 11
    group_max_failures: 6
    group_window_secs: 31
    supervisor_max_failures: 12
    supervisor_window_secs: 61
    reset_after_secs: 121
  supervision_pipeline:
    journal_capacity: 123
    subscriber_capacity: 17
    concurrent_restart_limit: 3
shutdown:
  graceful_timeout_ms: 1000
  abort_wait_ms: 100
observability:
  event_journal_capacity: 64
  metrics_enabled: true
  audit_enabled: true
audit:
  enabled: true
  backend: memory
  failure_strategy: fail_closed
  max_defer_queue: 1000
backpressure:
  strategy: alert_and_block
  warn_threshold_pct: 80
  critical_threshold_pct: 95
  window_secs: 30
  audit_channel_capacity: 1024
groups:
  - name: core
    children:
      - api
    budget:
      window_secs: 30
      max_burst: 3
      recovery_rate_per_sec: 0.5
  - name: upstream
    children: []
group_strategies:
  - group: core
    strategy: OneForOne
    restart_limit:
      max_restarts: 4
      window_ms: 30000
    escalation_policy: quarantine_scope
group_dependencies:
  - from_group: core
    to_group: upstream
    propagation: Full
child_strategy_overrides:
  - child_id: api
    strategy: RestForOne
    restart_limit:
      max_restarts: 2
      window_ms: 20000
    escalation_policy: shutdown_tree
severity_defaults:
  - task_role: service
    severity: Critical
children:
  - name: api
    kind: supervisor
    criticality: critical
    tags:
      - core
    task_role: supervisor
    severity: Critical
    group: core
    restart_policy: permanent
"#;

/// Returns a valid YAML configuration document.
fn valid_yaml() -> &'static str {
    VALID_YAML
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
    assert_eq!(state.audit.backend, "memory");
    assert_eq!(state.audit.failure_strategy, "fail_closed");
    assert_eq!(
        state.backpressure.strategy,
        BackpressureStrategy::AlertAndBlock
    );
    assert_eq!(state.backpressure.warn_threshold_pct, 80);
    assert_eq!(state.backpressure.critical_threshold_pct, 95);
    assert_eq!(state.supervisor.dynamic_supervisor.child_limit, Some(16));
    assert_eq!(state.policy.restart_budget.window_secs, 45);
    assert_eq!(state.policy.restart_budget.max_burst, 7);
    assert_eq!(state.policy.failure_window.threshold, 4);
    assert_eq!(state.policy.meltdown.child_max_restarts, 4);
    assert_eq!(state.policy.supervision_pipeline.journal_capacity, 123);
    assert_eq!(state.groups.len(), 2);
    assert_eq!(state.group_strategies.len(), 1);
    assert_eq!(
        state.group_dependencies[0].propagation,
        PropagationPolicy::Full
    );
    assert_eq!(state.child_strategy_overrides.len(), 1);
    assert_eq!(state.severity_defaults[0].task_role, TaskRole::Service);
    assert_eq!(state.children[0].task_role, Some(TaskRole::Supervisor));
    assert_eq!(state.children[0].severity, Some(SeverityClass::Critical));
    assert_eq!(state.children[0].group.as_deref(), Some("core"));
}

/// Verifies that raw configuration input converts into validated state.
#[test]
fn supervisor_config_converts_into_config_state_and_spec() {
    let config: SupervisorConfig = serde_yaml::from_str(valid_yaml()).expect("deserialize config");
    let state = ConfigState::try_from(config).expect("validate config");
    let spec = state.to_supervisor_spec().expect("derive supervisor spec");

    assert_eq!(state.supervisor.strategy, SupervisionStrategy::RestForOne);
    assert_eq!(spec.strategy, SupervisionStrategy::RestForOne);
    assert_eq!(spec.supervisor_failure_limit, 30);
    assert_eq!(
        spec.backpressure_config.strategy,
        BackpressureStrategy::AlertAndBlock
    );
    assert_eq!(spec.dynamic_supervisor_policy.child_limit, Some(16));
    assert_eq!(spec.restart_budget_config.window.as_secs(), 45);
    assert_eq!(spec.restart_budget_config.max_burst, 7);
    assert_eq!(spec.restart_budget_config.recovery_rate_per_sec, 0.25);
    assert!(matches!(
        spec.failure_window_config.mode,
        WindowMode::TimeSliding { window_secs: 90 }
    ));
    assert_eq!(spec.failure_window_config.threshold, 4);
    assert_eq!(spec.meltdown_policy.child_max_restarts, 4);
    assert_eq!(spec.meltdown_policy.group_window.as_secs(), 31);
    assert_eq!(spec.pipeline_journal_capacity, 123);
    assert_eq!(spec.pipeline_subscriber_capacity, 17);
    assert_eq!(spec.concurrent_restart_limit, 3);
    assert_eq!(spec.children.len(), 1);
    assert_eq!(spec.group_configs.len(), 2);
    assert_eq!(spec.group_strategies.len(), 1);
    assert_eq!(spec.group_dependencies.len(), 1);
    assert_eq!(spec.child_strategy_overrides.len(), 1);
    assert_eq!(
        spec.severity_defaults.get(&TaskRole::Service),
        Some(&SeverityClass::Critical)
    );
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

/// Verifies that JSON Lines audit storage requires an explicit file path.
#[test]
fn yaml_config_rejects_file_audit_without_path() {
    let yaml = valid_yaml().replace("backend: memory", "backend: file");
    let result = parse_config_state(&yaml);

    assert!(result.is_err());
}

/// Verifies that backpressure warning threshold must be lower than critical.
#[test]
fn yaml_config_rejects_invalid_backpressure_thresholds() {
    let yaml = valid_yaml().replace("warn_threshold_pct: 80", "warn_threshold_pct: 95");
    let result = parse_config_state(&yaml);

    assert!(result.is_err());
}

/// Verifies that lower-level policy values loaded from YAML are validated.
#[test]
fn yaml_config_rejects_invalid_restart_budget() {
    let yaml = valid_yaml().replace("max_burst: 7", "max_burst: 0");
    let result = parse_config_state(&yaml);

    assert!(result.is_err());
}

/// Verifies that group-level YAML references must target declared children.
#[test]
fn yaml_config_rejects_unknown_group_child() {
    let yaml = valid_yaml().replace("- api", "- missing-child");
    let result = parse_config_state(&yaml);

    assert!(result.is_err());
}
