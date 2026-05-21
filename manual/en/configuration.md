# Configuration and Schema

Language: [中文](../zh/configuration.html)

## Entry Point

The configuration entry point is `rust_supervisor::config::loader::load_config_from_yaml_file`. It accepts only the YAML primary configuration file. The repository example path is `examples/config/supervisor.yaml`.

The configuration struct `SupervisorConfig` contains these top-level groups:

| Group | Type | Description |
|---|---|---|
| `include` | `Vec<PathBuf>` | Additional config files included by `rust-config-tree` |
| `supervisor` | `SupervisorRootConfig` | Root supervision strategy |
| `policy` | `PolicyConfig` | Restart, backoff, heartbeat, failure window, restart budget, meltdown fuse, and supervision pipeline capacities |
| `shutdown` | `ShutdownConfig` | Graceful timeout and abort wait budgets |
| `observability` | `ObservabilityConfig` | Event journal capacity and metric/audit switches |
| `audit` | `AuditConfig` | Audit storage backend, JSON Lines file path, and write failure strategy |
| `backpressure` | `BackpressureConfig` | Backpressure strategy, thresholds, window, and audit channel capacity for observability subscribers |
| `groups` | `Vec<GroupConfig>` | Group membership and group-level restart budget overrides |
| `group_strategies` | `Vec<GroupStrategyConfig>` | Group-level supervision strategies, restart limits, and escalation policies |
| `group_dependencies` | `Vec<GroupDependencyConfig>` | Cross-group failure propagation edges |
| `child_strategy_overrides` | `Vec<ChildStrategyOverrideConfig>` | Child-level supervision strategies, restart limits, and escalation policies |
| `severity_defaults` | `Vec<SeverityDefaultConfig>` | Default severity class per task role |
| `dashboard` | `Option<DashboardIpcConfig>` | Optional dashboard IPC socket (Unix only) |
| `children` | `Vec<ChildDeclaration>` | Declarative child specifications |

## Configuration State

`rust_supervisor::config::configurable::SupervisorConfig` is the public root configuration struct. It supports `confique::Config`, `schemars::JsonSchema`, `serde::Serialize`, and `serde::Deserialize`. Users can reuse the same model for YAML loading, template generation, and JSON Schema generation.

`ConfigState` is the validated immutable state. Runtime modules must not keep separate runtime tunable constants.

`ConfigState::to_supervisor_spec` derives `SupervisorSpec`. The implementation fills the supervision strategy, policy defaults, shutdown budgets, health timing, observability capacity, backpressure policy, dynamic supervisor policy, restart budget, failure window, meltdown fuse, supervision pipeline capacities, group policies, and child strategy overrides from configuration values.

## Template Boundary

The official template is `examples/config/supervisor.template.yaml`. It covers `supervisor`, `policy`, `shutdown`, `observability`, `audit`, `backpressure`, `groups`, `group_strategies`, `group_dependencies`, `child_strategy_overrides`, `severity_defaults`, `dashboard`, and `children`.

This crate does not add `x-tree-split` to the public configuration structs, official schema, or official template. Projects that want split configuration files can wrap or reuse `SupervisorConfig` in their own crate and decide their own tree split layout.

## Error Boundary

Configuration loading returns `SupervisorError::FatalConfig` when startup must be rejected:

Root-level checks:
- The file extension is not YAML.
- The file cannot be read.
- YAML cannot be parsed into `SupervisorConfig`.
- The supervision strategy is not one of `OneForOne`, `OneForAll`, or `RestForOne`.
- A required numeric value is zero.
- The initial backoff is greater than the maximum backoff.
- The jitter ratio is outside the accepted range.
- `policy.restart_budget.window_secs`, `policy.restart_budget.max_burst`, or `policy.restart_budget.recovery_rate_per_sec` is invalid.
- `policy.failure_window.window_secs`, `policy.failure_window.max_count`, or `policy.failure_window.threshold` is invalid.
- A `policy.meltdown.*` window or threshold is zero.
- A `policy.supervision_pipeline.*` capacity or concurrent restart limit is zero.
- `supervisor.dynamic_supervisor.child_limit` is zero.
- `backpressure.warn_threshold_pct` is not between 1 and 100.
- `backpressure.critical_threshold_pct` is not between 1 and 100.
- `backpressure.warn_threshold_pct` is greater than or equal to `backpressure.critical_threshold_pct`.
- `backpressure.window_secs` or `backpressure.audit_channel_capacity` is zero.

Child declaration checks:
- Child ID and name must be non-empty.
- Tags must be non-empty.
- A child with `kind: Supervisor` must not have a factory; a child with `kind: AsyncWorker` or `kind: BlockingWorker` must have one.
- Sidecar task role requires `sidecar_config`, and vice versa.
- Dependency cycles are rejected.
- Child names referenced by `groups.children` must exist.
- Group names referenced by `group_strategies` and `group_dependencies` must exist.
- Child names referenced by `child_strategy_overrides` must exist.
- `severity_defaults` must not declare the same task role more than once.

IPC checks (when `dashboard.enabled = true`):
- `target_id` must be non-empty.
- `path` is required and must be absolute.
- Registration `relay_registration_path` is required and must be absolute.
- `lease_seconds` must be greater than zero.
- `heartbeat_interval_seconds` must be positive and less than `lease_seconds`.

`Supervisor::start_from_config_file` rejects invalid configuration before it creates runtime channels or spawns the control loop.

## Example Configuration

```yaml
supervisor:
  strategy: OneForAll
  escalation_policy: escalate_to_parent
  dynamic_supervisor:
    enabled: true
    child_limit: 16
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
  restart_budget:
    window_secs: 60
    max_burst: 10
    recovery_rate_per_sec: 0.50
  failure_window:
    mode: time_sliding
    window_secs: 60
    max_count: 5
    threshold: 5
  meltdown:
    child_max_restarts: 3
    child_window_secs: 10
    group_max_failures: 5
    group_window_secs: 30
    supervisor_max_failures: 10
    supervisor_window_secs: 60
    reset_after_secs: 120
  supervision_pipeline:
    journal_capacity: 100
    subscriber_capacity: 10
    concurrent_restart_limit: 5
shutdown:
  graceful_timeout_ms: 5000
  abort_wait_ms: 1000
observability:
  event_journal_capacity: 256
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
      window_secs: 60
      max_burst: 10
      recovery_rate_per_sec: 0.50
  - name: upstream
    children: []
group_strategies:
  - group: core
    strategy: OneForOne
    restart_limit:
      max_restarts: 5
      window_ms: 60000
    escalation_policy: quarantine_scope
group_dependencies:
  - from_group: core
    to_group: upstream
    propagation: Full
child_strategy_overrides:
  - child_id: api
    strategy: RestForOne
    restart_limit:
      max_restarts: 3
      window_ms: 30000
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
    restart_policy: transient
dashboard:
  enabled: true
  target_id: payments-worker-a
  path: /tmp/rust-supervisor-demo/payments-worker-a.sock
  permissions: "0600"
  bind_mode: replace_stale
  registration:
    enabled: true
    relay_registration_path: /tmp/rust-supervisor-demo/dashboard-relay-registration.sock
    display_name: "payments worker a"
    lease_seconds: 30
    registration_heartbeat_interval_seconds: 15
```

## Secret Placeholders

Configuration values that reference secrets use the `${SECRET_NAME}` placeholder format.
Replace these placeholders with environment variables or your secret management solution
before starting the supervisor. Example:

```yaml
dashboard:
  security_config:
    peer_identity:
      allowed_uids: [ "${SUPERVISOR_UID}" ]
```

`dashboard.security_config` does not carry audit settings. IPC audit persistence uses the root `audit` section so there is one authoritative `AuditConfig`.

The supervisor does not resolve placeholders at runtime; replacement must happen
before configuration loading (e.g., via `envsubst` or your deployment pipeline).

TLS is handled by the relay layer (`rust-supervisor-relay`) using `wss://`. The supervisor
target process exposes only a local Unix domain socket and does not terminate TLS.

## Upgrade

This version does not support in-place upgrades. To upgrade, deploy a fresh instance
with the new version and migrate state through the external IPC interface.
