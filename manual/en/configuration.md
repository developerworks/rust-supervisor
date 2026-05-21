# Configuration and Schema

Language: [中文](../zh/configuration.html)

## Entry Point

The configuration entry point is `rust_supervisor::config::loader::load_config_from_yaml_file`. It accepts only the YAML primary configuration file. The repository example path is `examples/config/supervisor.yaml`.

The configuration struct `SupervisorConfig` contains these top-level groups:

| Group | Type | Description |
|---|---|---|
| `include` | `Vec<PathBuf>` | Additional config files included by `rust-config-tree` |
| `supervisor` | `SupervisorRootConfig` | Root supervision strategy |
| `policy` | `PolicyConfig` | Restart, backoff, heartbeat, and fuse limits |
| `shutdown` | `ShutdownConfig` | Graceful timeout and abort wait budgets |
| `observability` | `ObservabilityConfig` | Event journal capacity and metric/audit switches |
| `ipc` | `Option<DashboardIpcConfig>` | Optional dashboard IPC socket (Unix only) |
| `children` | `Vec<ChildDeclaration>` | Declarative child specifications |

## Configuration State

`rust_supervisor::config::configurable::SupervisorConfig` is the public root configuration struct. It supports `confique::Config`, `schemars::JsonSchema`, `serde::Serialize`, and `serde::Deserialize`. Users can reuse the same model for YAML loading, template generation, and JSON Schema generation.

`ConfigState` is the validated immutable state. Runtime modules must not keep separate runtime tunable constants.

`ConfigState::to_supervisor_spec` derives `SupervisorSpec`. The implementation fills the supervision strategy, policy defaults, shutdown budgets, health timing, and observability capacity from configuration values.

## Template Boundary

The official template is `examples/config/supervisor.template.yaml`. It covers `supervisor`, `policy`, `shutdown`, `observability`, `ipc`, and `children` (the latter two are commented out by default).

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

Child declaration checks:
- Child ID and name must be non-empty.
- Tags must be non-empty.
- A child with `kind: Supervisor` must not have a factory; a child with `kind: AsyncWorker` or `kind: BlockingWorker` must have one.
- Sidecar work role requires `sidecar_config`, and vice versa.
- Dependency cycles are rejected.
- Group names referenced by `child_strategy_overrides` must exist in `group_strategies`.

IPC checks (when `ipc.enabled = true`):
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
ipc:
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
ipc:
  security_config:
    peer_identity:
      allowed_uids: [ "${SUPERVISOR_UID}" ]
```

The supervisor does not resolve placeholders at runtime; replacement must happen
before configuration loading (e.g., via `envsubst` or your deployment pipeline).

TLS is handled by the relay layer (`rust-supervisor-relay`) using `wss://`. The supervisor
target process exposes only a local Unix domain socket and does not terminate TLS.

## Upgrade

This version does not support in-place upgrades. To upgrade, deploy a fresh instance
with the new version and migrate state through the external IPC interface.
