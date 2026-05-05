# rust-supervisor Manual

## Project Scope

`rust-supervisor` is a Rust task supervision core for Tokio services. It uses declarative models to manage child startup, stop, restart, quarantine, state query, event recording, health checks, and Shutdown Without Orphaned Tasks.

The configuration boundary uses rust-config-tree v0.1.9 with YAML files. Runtime tunable values must enter the system through this centralized configuration path.

This project has no legacy interface burden. Users should import public types from owning module paths, such as `rust_supervisor::runtime::supervisor::Supervisor`.

## Reading Path

- [Getting Started](getting-started.md): start a minimal supervisor from YAML configuration.
- [Configuration](configuration.md): understand `SupervisorConfig`, `ConfigState`, and startup rejection boundaries.
- [Supervisor Tree](supervisor-tree.md): understand `SupervisorSpec`, `SupervisorTree`, and registry ownership.
- [Task Model](task-model.md): understand `ChildSpec`, `TaskFactory`, `TaskContext`, and readiness.
- [Policies](policies.md): understand restart decisions, backoff, fuse rules, quarantine, and task exit classification.
- [Runtime Control](runtime-control.md): understand `SupervisorHandle` commands and idempotent behavior.
- [Shutdown](shutdown.md): understand four-stage shutdown and blocking worker boundaries.
- [Observability](observability.md): understand events, logs, tracing, metrics, audit data, and run summaries.
- [Examples](examples.md): run each learning example under `examples/`.
- [Quality Gates](quality-gates.md): run formatting, build, test, documentation, SBOM, and release checks.

## Runtime Boundary

The supervisor core governs lifecycle behavior only. High-frequency business messages belong in the data plane. The control plane handles lifecycle commands, current state queries, events, and governance decisions.
