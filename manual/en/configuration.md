# Configuration and Schema

## Entry Point

The configuration entry point is `rust_supervisor::config::loader::load_config_state`. It accepts only the YAML primary configuration file. The repository example path is `examples/config/supervisor.yaml`.

The current configuration shape contains `supervisor`, `policy`, `shutdown`, and `observability` groups. They map into `SupervisorRootConfig`, `PolicyConfig`, `ShutdownConfig`, and `ObservabilityConfig`.

## Configuration State

`rust_supervisor::config::configurable::SupervisorConfig` is the public root configuration struct. It supports `confique::Config`, `schemars::JsonSchema`, `serde::Serialize`, and `serde::Deserialize`. Users can reuse the same model for YAML loading, template generation, and JSON Schema generation.

`ConfigState` is the validated immutable state. Runtime modules must not keep separate runtime tunable constants.

`ConfigState::to_supervisor_spec` derives `SupervisorSpec`. The implementation fills the supervision strategy, policy defaults, shutdown budgets, health timing, and observability capacity from configuration values.

## Template Boundary

The official template is `examples/config/supervisor.template.yaml`. It remains a single YAML file by default and covers `supervisor`, `policy`, `shutdown`, and `observability`.

This crate does not add `x-tree-split` to the public configuration structs, official schema, or official template. Projects that want split configuration files can wrap or reuse `SupervisorConfig` in their own crate and decide their own tree split layout.

## Error Boundary

Configuration loading returns `SupervisorError::FatalConfig` when startup must be rejected:

- The file extension is not YAML.
- The file cannot be read.
- YAML cannot be parsed into `SupervisorConfig`.
- The supervision strategy is not one of `OneForOne`, `OneForAll`, or `RestForOne`.
- A required numeric value is zero.
- The initial backoff is greater than the maximum backoff.
- The jitter ratio is outside the accepted range.

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
```
