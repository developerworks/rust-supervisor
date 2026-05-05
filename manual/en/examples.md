# Examples

## Quick Start

```bash
cargo run --example supervisor_quickstart
```

`supervisor_quickstart` reads `examples/config/supervisor.yaml`, derives `SupervisorSpec`, starts a supervisor, queries current state, and shuts down the tree.

## Configuration Tree

```bash
cargo run --example config_tree_supervisor
```

`config_tree_supervisor` shows the rust-config-tree v0.1.9 YAML loading path and prints the derived `SupervisorSpec`.

## Restart Policy Lab

```bash
cargo run --example restart_policy_lab
```

`restart_policy_lab` shows the basic shapes of `TaskFailure`, `TaskFailureKind`, `RestartPolicy`, `SupervisionStrategy`, and `RestartDecision`.

## Shutdown Tree

```bash
cargo run --example shutdown_tree
```

`shutdown_tree` demonstrates request stop, graceful drain, abort stragglers, and reconcile before calling `shutdown_tree`.

## Observability Probe

```bash
cargo run --example observability_probe
```

`observability_probe` subscribes to events, queries current state, prints one event, and shuts down. It checks the observability integration path.

## Supervisor Tree Story

```bash
cargo run --example supervisor_tree_story
```

`supervisor_tree_story` declares market feed, risk engine, and audit sink children. It shows dependencies, tags, criticality, explicit readiness, startup order, shutdown order, and `RestForOne` restart scope.

## Runtime Control Story

```bash
cargo run --example runtime_control_story
```

`runtime_control_story` starts a real supervisor and runs `add_child`, `pause_child`, `resume_child`, `quarantine_child`, `current_state`, `subscribe_events`, and `shutdown_tree`. It combines operator control with audit events.

## Policy Failure Matrix

```bash
cargo run --example policy_failure_matrix
```

`policy_failure_matrix` feeds success, external dependency failure, fatal bug failure, and panic into `Permanent`, `Transient`, and `Temporary` restart policies. It also shows deterministic jitter and meltdown tracking.

## Diagnostic Replay

```bash
cargo run --example diagnostic_replay
```

`diagnostic_replay` builds deterministic events, writes them into the event journal, replays failure, backoff, and restart facts, then generates metric samples and `RunSummary`.
