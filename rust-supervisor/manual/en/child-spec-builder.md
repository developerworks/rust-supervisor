# ChildSpecBuilder

Language: [中文](../zh/child-spec-builder.html)

## One-sentence summary

`ChildSpecBuilder` is the fluent API for constructing `ChildSpec` values in Rust code. Configuration and RPC should still use `ChildDeclaration`. The build exit is `build() -> Result<ChildSpec, SupervisorError>`, which calls `ChildSpec::validate()` internally.

Relationship to [`child-spec.md`](child-spec.md): that page explains how declarations and specs divide responsibility. This page focuses on builder entry points, setters, and common usage patterns.

## Module path

```rust
use rust_supervisor::spec::child_builder::ChildSpecBuilder;
```

The module is defined in [`src/spec/child_builder.rs`](../../src/spec/child_builder.rs). Per project module-boundary rules, there is **no** `pub use` re-export.

## When to use the builder

| Scenario | Recommended approach |
| --- | --- |
| YAML config, `add_child` RPC payloads | `ChildDeclaration` + `TryFrom` |
| Tests, examples, hand-built runtime specs in code | `ChildSpecBuilder` |
| Worker default bundle only, no fluent chain | `ChildSpec::worker(...)?` (delegates to the builder internally) |

Legacy code may still mutate fields after construction. New code should prefer the builder.

## Entry methods

| Method | Purpose | Default highlights |
| --- | --- | --- |
| `worker(id, name, kind, factory)` | Async or blocking worker | Matches `ChildSpec::worker`: `Transient` restart, `Critical` criticality, `TaskRole::Worker`, and so on |
| `service(id, name, kind, factory)` | Long-running service | Based on worker defaults: `TaskRole::Service`, `Critical` criticality |
| `job(id, name, kind, factory)` | Finite job | Based on worker defaults: `TaskRole::Job`, `Optional` criticality |
| `sidecar(id, name, kind, factory, sidecar_config)` | Sidecar attached to a primary child | Based on worker defaults: `TaskRole::Sidecar`, writes `sidecar_config`, and automatically adds the primary child dependency |
| `supervisor(id, name)` | Nested supervisor | `kind = Supervisor`, `factory = None`, `task_role = Supervisor`, `criticality = Critical` |
| `new(id, name)` | Minimal skeleton | Sets only `id` / `name` plus baseline policies; caller must add `kind` and, for workers, `factory` |

## Build exit

| Method | Behavior |
| --- | --- |
| `build()` | Takes the inner `ChildSpec`, calls `validate()`, returns `Ok(spec)` or `SupervisorError` |

All entry methods and setters return `ChildSpecBuilder`, which means construction is still in progress. Only `build()` consumes the builder and returns the final `ChildSpec`.

There is **no** `build_validated()`. Validation is always performed inside `build()`.

`ChildSpec::worker(...)` also returns `Result<ChildSpec, SupervisorError>` via `ChildSpecBuilder::worker(...).build()`.

## Basic usage

```rust
use rust_supervisor::error::types::SupervisorError;
use rust_supervisor::id::types::ChildId;
use rust_supervisor::policy::task_role_defaults::TaskRole;
use rust_supervisor::spec::child::TaskKind;
use rust_supervisor::spec::child_builder::ChildSpecBuilder;
use rust_supervisor::task::factory::{TaskResult, service_fn};
use std::sync::Arc;

fn build_worker() -> Result<ChildSpec, SupervisorError> {
    let factory = Arc::new(service_fn(|_ctx| async { TaskResult::Succeeded }));
    ChildSpecBuilder::worker(
        ChildId::new("invoice-worker"),
        "Invoice Worker",
        TaskKind::AsyncWorker,
        factory,
    )
    .task_role(TaskRole::Worker)
    .tag("invoice")
    .build()
}
```

Propagate errors with `?`, or use `build().expect("...")` in tests.

## Fluent setter coverage

Each setter consumes `self` and returns `Self`. You can chain them in any order that remains semantically valid.

**Policy fields**: `isolation`, `restart_policy`, `shutdown_policy`, `health_policy`, `readiness_policy`, `backoff_policy`

**Topology and classification**: `dependencies`, `dependency`, `tags`, `tag`, `criticality`, `task_role`, `without_task_role`, `sidecar_config`, `without_sidecar_config`, `severity`, `without_severity`, `group`, `without_group`

**Config blocks**: `health_check`, `without_health_check`, `readiness`, `without_readiness`, `resource_limits`, `without_resource_limits`, `command_permissions`, `environment`, `env_var`, `secrets`, `secret`, `cleanup_paths`, `cleanup_path`

**Runtime**: `kind`, `factory`, `without_factory` (for `new()` or supervisor paths)

Naming convention: plural fields use `dependencies(...)`, `tags(...)`; singular helpers use `dependency(...)`, `tag(...)`. The same pattern applies to `environment` / `env_var`, `secrets` / `secret`, and `cleanup_paths` / `cleanup_path`.

## Common combinations

### Service

Long-running services should prefer `service(...)`; callers do not need to set `TaskRole::Service` by hand:

```rust
ChildSpecBuilder::service(id, "API Service", TaskKind::AsyncWorker, factory)
    .tag("service")
    .build()?;
```

### Job

Finite work should prefer `job(...)`. You can still override `restart_policy` for one-shot behavior:

```rust
ChildSpecBuilder::job(id, "Nightly Export", TaskKind::AsyncWorker, factory)
    .restart_policy(RestartPolicy::Temporary)
    .build()?;
```

### Sidecar

Sidecars attached to a primary child should prefer `sidecar(...)`. This entry writes `sidecar_config` and automatically adds the primary child dependency:

```rust
use rust_supervisor::policy::task_role_defaults::SidecarConfig;

ChildSpecBuilder::sidecar(
    id,
    "Metrics Sidecar",
    TaskKind::AsyncWorker,
    factory,
    SidecarConfig::new(primary_id.clone(), false),
)
.build()?;
```

If you still configure `task_role = Sidecar` manually with setters, you must also set `sidecar_config`, or `build()` validation fails.

### Worker from `new()`

```rust
ChildSpecBuilder::new(ChildId::new("custom"), "custom")
    .kind(TaskKind::AsyncWorker)
    .factory(factory)
    .build()?;
```

## Data flow (short)

```text
ChildSpecBuilder::worker / service / job / sidecar / supervisor / new
        |
        v
   fluent setters (policy, role, deps, env, ...)
        |
        v
   build()  -->  ChildSpec::validate()
        |
        +-- Ok(ChildSpec)  -->  Supervisor::start / register topology
        +-- Err(SupervisorError)
```

## Example program

Runnable demo:

```bash
cargo run --example child_spec_builder
```

Source: [`examples/child_spec_builder.rs`](../../examples/child_spec_builder.rs). Covers worker, service, job, sidecar, supervisor, the `new()` path, and an intentionally invalid sidecar combination.

## Tests and regression

External tests: [`src/spec/tests/child_builder_test.rs`](../../src/spec/tests/child_builder_test.rs)

| Test | What it verifies |
| --- | --- |
| `worker_builder_matches_child_spec_worker_defaults` | Builder output matches `ChildSpec::worker` field-for-field |
| `supervisor_builder_produces_valid_supervisor_child` | Supervisor entry has no factory and validates |
| `service_builder_sets_service_role` | Service entry sets `TaskRole::Service` and `Critical` criticality |
| `job_builder_sets_job_role_and_optional_criticality` | Job entry sets `TaskRole::Job` and `Optional` criticality |
| `sidecar_builder_sets_sidecar_role_binding_and_dependency` | Sidecar entry sets the binding and automatically adds the primary child dependency |
| `builder_setters_apply_expected_fields` | Sidecar, dependency, tag, and related setters |
| `build_rejects_invalid_sidecar_combination` | Missing `sidecar_config` makes `build()` fail |
| `new_builder_can_build_valid_worker_with_factory` | `new()` path works after required fields are set |

Run:

```bash
cargo test --test child_builder_test
```

## Known boundaries

- Default policy bundles for `TryFrom<ChildDeclaration>` are not fully shared with the builder yet. The two paths may evolve independently; review both when changing defaults.
- The builder does not handle serde. Dynamic child adds still flow through `ChildDeclaration`.
- Legacy examples and tests were not bulk-migrated from `ChildSpec::worker`. Both styles are runtime-equivalent when callers handle `Result`.

## Further reading

- [`child-spec.md`](child-spec.md) — how `ChildDeclaration` and `ChildSpec` relate, plus a short builder introduction
- [`docs/architecture.md`](../../docs/architecture.md) — module boundaries and the no re-export rule
