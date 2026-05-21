# Job Role Example

[中文说明](README.zh.md)

This example shows a one-shot `WorkRole::Job` child. A job role is intended for finite work that should finish and then stay stopped.

Run it with:

```bash
cargo run --package rust-tokio-supervisor --example job
```

## What It Shows

- Initialization: `job_task.rs` builds a `daily-report-job` child and marks it as ready.
- Running: the job runs one report-generation business function and prints the current UNIX time.
- Completion: the job returns `TaskResult::Succeeded`, matching the job role's one-shot behavior.
- Cleanup: `main.rs` calls `shutdown_tree` after the job has stopped itself.

## File Layout

- `main.rs`: wires the supervisor, event subscription, state snapshots, and cleanup shutdown.
- `job_task.rs`: declares the `WorkRole::Job` child and the one-shot job body.
- `observation.rs`: prints job facts, runtime events, current state records, and shutdown outcomes.
