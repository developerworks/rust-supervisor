# ChildSpec and ChildDeclaration

Language: [中文](../zh/child-spec.html)

## How do ChildSpec and ChildDeclaration relate?

`ChildDeclaration` is the **external declaration** that arrives from configuration and RPC. `ChildSpec` is the **internal specification** the supervisor runtime uses to register, start, and restart children. The two share many fields but serve different roles. They are connected through `TryFrom` conversion, which also fills in defaults.

## What each one is

|                    | `ChildDeclaration`                   | `ChildSpec`                        |
| ------------------ | ------------------------------------ | ---------------------------------- |
| **Module**         | `src/spec/child_declaration.rs`      | `src/spec/child.rs`                |
| **Role**           | **Input model** for YAML, `add_child` payloads, and similar sources | **Runtime model** in the registry and control loop |
| **Typical source** | Config file deserialization, dynamic child add requests | Converted from a declaration, or built directly in code |
| **Can it run alone?** | No. It has no factory and no fully materialized policy objects | Yes. The supervisor manages lifecycle from it |

`ChildDeclaration` focuses on a **serializable, validatable declaration**: names, dependency names, environment variables, secret placeholders, `health_check` / `readiness` config blocks, and rules such as `validate_child_declaration` (name format, `${SECRET}` syntax, and so on).

Beyond declaration fields, `ChildSpec` also carries runtime essentials such as:

- A resolved `ChildId` derived from `name`
- `factory: Option<Arc<dyn TaskFactory>>`, the task factory that actually runs work (**not part of serde**)
- Materialized `HealthPolicy`, `ReadinessPolicy`, `ShutdownPolicy`, and `BackoffPolicy`
- Runtime fields such as `isolation` and `cleanup_paths`

## How they connect

The data flow looks like this:

```text
YAML / add_child RPC
        |
        v
  ChildDeclaration  ---- validate_child_declaration ----+
        |                                                  |
        | TryFrom<ChildDeclaration> for ChildSpec           |
        v                                                  |
     ChildSpec  --------------------------------------------+
        |
        v
  Register topology, start children, policy pipeline, restart / meltdown, etc.
```

The conversion lives in `TryFrom<ChildDeclaration> for ChildSpec` inside `child_declaration.rs`. It performs steps such as:

- `name` -> `ChildId::new(&decl.name)`
- dependency **names** in `dependencies` -> `Vec<ChildId>`
- `health_check` -> `HealthPolicy` with default intervals
- `readiness` present -> `ReadinessPolicy::Explicit`, otherwise `Immediate`
- `shutdown_policy` / `backoff_policy` and similar fields receive **defaults** during conversion even when the declaration omits them

When a child is added dynamically, `PendingChild` keeps **both** the `declaration` and the converted `child_spec`. Auditing also stores a SHA-256 of the declaration (`declaration_hash`) for reconciliation and compensation.

## Shared types

Shared enums and config structs such as `RestartPolicy`, `TaskKind`, and `HealthCheckConfig` are defined in `child.rs`. `ChildDeclaration` **reuses** them to avoid parallel type trees. The **top-level containers** remain separate: declaration container vs specification container.

## How to remember them

- Writing config, handling API input, validating declarations -> think **`ChildDeclaration`**
- Seeing how the supervisor manages a child or what the policy engine reads -> think **`ChildSpec`**
- Asking whether YAML and runtime use the same thing -> **same underlying information, different lifecycle stage**: declaration is input, spec is the landed form

## In-code construction

Configuration and RPC should still use `ChildDeclaration`. When you construct a runtime spec directly in Rust, prefer `ChildSpecBuilder`:

```rust
use rust_supervisor::id::types::ChildId;
use rust_supervisor::policy::task_role_defaults::TaskRole;
use rust_supervisor::spec::child::TaskKind;
use rust_supervisor::spec::child_builder::ChildSpecBuilder;
use rust_supervisor::task::factory::{TaskResult, service_fn};
use std::sync::Arc;

let factory = service_fn(|_ctx| async { TaskResult::Succeeded });
let spec = ChildSpecBuilder::worker(
    ChildId::new("worker"),
    "worker",
    TaskKind::AsyncWorker,
    Arc::new(factory),
)
.task_role(TaskRole::Worker)
.tag("invoice")
.build()?;
```

Entry methods:

| Method                              | Purpose                                                                 |
| ----------------------------------- | ----------------------------------------------------------------------- |
| `ChildSpecBuilder::worker(...)`     | Async or blocking worker; defaults match `ChildSpec::worker`            |
| `ChildSpecBuilder::service(...)`    | Long-running service; sets `TaskRole::Service`                          |
| `ChildSpecBuilder::job(...)`        | Finite job; sets `TaskRole::Job`                                         |
| `ChildSpecBuilder::sidecar(...)`    | Sidecar; sets sidecar binding and the primary child dependency           |
| `ChildSpecBuilder::supervisor(...)` | Nested supervisor; no factory                                           |
| `ChildSpecBuilder::new(...)`        | Minimal skeleton; caller must set `kind` and, for workers, `factory`    |

Build exit:

| Method    | Behavior                                                                 |
| --------- | ------------------------------------------------------------------------ |
| `build()` | Calls `ChildSpec::validate()` after construction; returns `SupervisorError` on failure |

`ChildSpec::worker(...)` remains available. It delegates to `ChildSpecBuilder::worker(...).build()` and also returns `Result<ChildSpec, SupervisorError>`.

For field-by-field mapping and defaults through `TryFrom`, see [`child-spec-builder.md`](child-spec-builder.md) for builder details, or inspect `child_declaration.rs` directly.
