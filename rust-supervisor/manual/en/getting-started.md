# Getting Started

Language: [中文](../zh/getting-started.html)

> **Walkthrough**: This guide has 5 steps (Step 1 of 5 to Step 5 of 5).
> Estimated completion time: 5 minutes.

## Step 1 of 5: Prerequisites

This project is a Rust library. The examples require Cargo and a Tokio application environment. Repository examples include their required dependencies.

The primary configuration file is `examples/config/supervisor.yaml`. The loader uses rust-config-tree 0.3.0, reads YAML, and produces `ConfigState`.

## Step 2 of 5: Minimal Command

```bash
cargo run --example supervisor_quickstart
```

The example loads YAML through `load_config_from_yaml_file`, derives `SupervisorSpec` through `ConfigState::to_supervisor_spec`, starts the runtime through `Supervisor::start`, queries `current_state`, and then shuts down the tree through `shutdown_tree`.

## Step 3 of 5: Minimal Code Path

```rust
use rust_supervisor::config::loader::load_config_from_yaml_file;
use rust_supervisor::runtime::supervisor::Supervisor;

#[tokio::main]
async fn main() -> Result<(), rust_supervisor::error::types::SupervisorError> {
    let state = load_config_from_yaml_file("examples/config/supervisor.yaml")?;
    let spec = state.to_supervisor_spec()?;
    let handle = Supervisor::start(spec).await?;
    let current = handle.current_state().await?;
    println!("{current:#?}");
    handle.shutdown_tree("operator", "quickstart complete").await?;
    Ok(())
}
```

## Step 4 of 5: Result

The example validates the integration path. It is not a business task template. Application workers should live inside `ChildSpec` and `TaskFactory` boundaries instead of being started as unmanaged background tasks.

## Step 5 of 5: Health Self-Check

After startup, the supervisor prints a health self-check JSON to stdout.
The JSON schema is formally defined in [health-selfcheck-schema.md](../specs/006-8-product-bundle-runbooks/contracts/health-selfcheck-schema.md).

Expected output (example):

```json
{
  "status": "ready",
  "supervisor_version": "0.1.2",
  "uptime_secs": 3600,
  "children": { "total": 5, "running": 5, "failed": 0 },
  "dashboard_link": "connected"
}
```

If `status` is not `"ready"`, check the operations runbook for troubleshooting steps.

---

## Entry Points

The `Supervisor` struct in `src/runtime/supervisor.rs:36-83` provides 3 entry methods:

| Method | Input | When to Use |
|---|---|---|
| `Supervisor::start(spec)` | `SupervisorSpec` (built programmatically) | You already have a spec object |
| `Supervisor::start_from_config_state(state)` | `ConfigState` (validated config) | You loaded config via the loader |
| `Supervisor::start_from_config_file(path)` | YAML file path | Direct launch from a file |

All 3 converge on the private `start_with_policy()` (`src/runtime/supervisor.rs:95-126`), which:

1. Calls `spec.validate()` to verify all child declarations
2. Creates an mpsc command channel and a broadcast event channel
3. Creates `RuntimeControlPlane` and `ObservabilityPipeline`
4. Builds `RuntimeControlState`
5. Spawns the control loop via `tokio::spawn(run_control_loop(...))`
6. Starts `RuntimeWatchdog` to monitor control loop health
7. Returns `SupervisorHandle` for commands (restart, shutdown, etc.) and event subscriptions

### Usage Examples

#### From YAML file via ConfigState — `start_from_config_state`

Full example: [`examples/supervisor_quickstart.rs`](../../examples/supervisor_quickstart.rs). Config: [`examples/config/supervisor.yaml`](../../examples/config/supervisor.yaml).

```rust
use rust_supervisor::config::loader::load_config_from_yaml_file;
use rust_supervisor::runtime::supervisor::Supervisor;

#[tokio::main]
async fn main() -> Result<(), rust_supervisor::error::types::SupervisorError> {
    let state = load_config_from_yaml_file("examples/config/supervisor.yaml")?;
    let handle = Supervisor::start_from_config_state(state).await?;
    handle.shutdown_tree("operator", "quickstart complete").await?;
    Ok(())
}
```

`load_config_from_yaml_file` returns a `ConfigState`. Its `to_supervisor_spec()` is called internally by `start_from_config_state`.

#### Direct from YAML file path — `start_from_config_file`

One-step shortcut that calls `load_config_from_yaml_file` internally:

```rust
use rust_supervisor::runtime::supervisor::Supervisor;

#[tokio::main]
async fn main() -> Result<(), rust_supervisor::error::types::SupervisorError> {
    let handle = Supervisor::start_from_config_file("examples/config/supervisor.yaml").await?;
    handle.shutdown_tree("operator", "done").await?;
    Ok(())
}
```

#### Programmatic spec — `start`

Full example: [`examples/supervisor_tree_story.rs`](../../examples/supervisor_tree_story.rs).

```rust
use std::sync::Arc;
use rust_supervisor::id::types::ChildId;
use rust_supervisor::runtime::supervisor::Supervisor;
use rust_supervisor::spec::child::{ChildSpec, TaskKind};
use rust_supervisor::spec::supervisor::SupervisorSpec;
use rust_supervisor::task::factory::{TaskResult, service_fn};

#[tokio::main]
async fn main() -> Result<(), rust_supervisor::error::types::SupervisorError> {
    let factory = service_fn(|ctx| async move {
        ctx.heartbeat();
        ctx.mark_ready();
        println!("child running at path={}", ctx.path);
        TaskResult::Succeeded
    });

    let child = ChildSpec::worker(
        ChildId::new("demo-worker"),
        "Demo Worker",
        TaskKind::AsyncWorker,
        Arc::new(factory),
    );

    let spec = SupervisorSpec::root(vec![child]);
    let handle = Supervisor::start(spec).await?;

    let state = handle.current_state().await?;
    println!("{state:#?}");
    handle.shutdown_tree("operator", "demo complete").await?;
    Ok(())
}
```

`ChildSpec::worker()` automatically sets `task_role = Some(TaskRole::Worker)`, equivalent to `task_role: worker` in YAML.

## TaskRole Behavior

The 5 `TaskRole` variants dispatch to different default lifecycle policies via `RoleDefaultPolicy::for_role()`:

| Dimension | Service | Worker | Job | Sidecar | Supervisor |
|---|---|---|---|---|---|
| **On success** | `Restart` | `Stop` | `Stop` | `Restart` | `Restart` |
| **On timeout** | `RestartWithBackoff` | `RestartWithBackoff` | `StopAndEscalate` | `RestartWithBackoff` | `RestartWithBackoff` |
| **Max restarts** | 10 | 3 | 1 | 5 | 3 |
| **Default severity** | `Critical` | `Standard` | `Optional` | `Standard` | `Critical` |

The per-task role defaults are defined by 5 constructors in `src/policy/task_role_defaults.rs:418-464`:

- **Service**: long-running daemon, restart on success, 10 retries, Critical severity — expected to stay online forever.
- **Worker**: background task, stop on success, 3 retries, Standard severity — stops when done.
- **Job**: one-shot task, stop on success, timeout escalates immediately (no retry), 1 retry, Optional severity — runs once then exits.
- **Sidecar**: auxiliary process, same staying behavior as Service but lower restart budget (5), requires a `SidecarConfig` binding to a primary.
- **Supervisor**: nested supervision tree, same staying behavior as Service, 3 retries, Critical severity.

When `task_role` is `None`, `EffectivePolicy::merge()` falls back to `TaskRole::Worker` with a warning. `semantic_conflicts_for_child()` detects role violations (e.g., Job with permanent restart policy).
