# Sidecar Role Example

[中文说明](README.zh.md)

This example shows a `WorkRole::Sidecar` child attached to a primary `WorkRole::Service` child. A sidecar role is an auxiliary process that runs beside a primary service and should stop with the supervisor tree.

Run it with:

```bash
cargo run --package rust-tokio-supervisor --example sidecar
```

The process keeps running until you press `Ctrl+C`.

## What It Shows

- Primary service: `api-service` is declared as `WorkRole::Service`.
- Sidecar binding: `metrics-sidecar` is declared as `WorkRole::Sidecar` and uses `SidecarConfig`.
- Running: both children print one business tick per second.
- Stop: after `Ctrl+C`, both children receive cancellation and stop through `shutdown_tree`.

## File Layout

- `main.rs`: wires the supervisor, event subscriptions, state snapshots, and shutdown command.
- `sidecar_task.rs`: declares the primary service, sidecar child, and sidecar business logic.
- `observation.rs`: prints child facts, runtime events, current state records, and shutdown outcomes.
