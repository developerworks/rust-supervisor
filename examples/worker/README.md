# Worker Role Example

[中文说明](README.zh.md)

This example shows a bounded `WorkRole::Worker` child. A worker role is a background task that does a finite amount of work and then stops after success.

Run it with:

```bash
cargo run --package rust-tokio-supervisor --example worker
```

## What It Shows

- Initialization: `worker_task.rs` builds an `invoice-worker` child and marks it as ready.
- Running: the worker processes 3 invoice batches and prints the current UNIX time for each batch.
- Completion: the worker returns `TaskResult::Succeeded`, matching the worker role's stop-on-success behavior.
- Cleanup: `main.rs` calls `shutdown_tree` after the worker has stopped itself.

## File Layout

- `main.rs`: wires the supervisor, event subscription, state snapshots, and cleanup shutdown.
- `worker_task.rs`: declares the `WorkRole::Worker` child and the bounded worker body.
- `observation.rs`: prints worker facts, runtime events, current state records, and shutdown outcomes.
