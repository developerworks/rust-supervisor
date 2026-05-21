# Supervisor Role Example

[中文说明](README.zh.md)

This example shows a runnable child classified as `TaskRole::Supervisor`. It models a nested supervisor unit as a long-running supervised task, so the example can demonstrate initialization, steady running, cooperative stop, and observation output.

Run it with:

```bash
cargo run --package rust-tokio-supervisor --example supervisor
```

The process keeps running until you press `Ctrl+C`.

## What It Shows

- Initialization: `supervisor_task.rs` builds `nested-supervisor-unit` and reports readiness.
- Running: the supervisor role unit prints one business tick per second.
- Stop: after `Ctrl+C`, the runtime calls `shutdown_tree` and the unit receives cancellation.
- Observation: `observation.rs` prints role facts, runtime events, current state records, and shutdown outcomes.

## File Layout

- `main.rs`: wires the supervisor, event subscriptions, state snapshots, and shutdown command.
- `supervisor_task.rs`: declares the `TaskRole::Supervisor` role unit and its business loop.
- `observation.rs`: prints lifecycle facts, runtime events, state records, and shutdown outcomes.
