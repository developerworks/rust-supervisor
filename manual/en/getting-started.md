# Getting Started

Language: [中文](../zh/getting-started.html)

> **Walkthrough**: This guide has 5 steps (Step 1 of 5 to Step 5 of 5).
> Estimated completion time: 5 minutes.

## Step 1 of 5: Prerequisites

This project is a Rust library. The examples require Cargo and a Tokio application environment. Repository examples include their required dependencies.

The primary configuration file is `examples/config/supervisor.yaml`. The loader uses rust-config-tree v0.1.9, reads YAML, and produces `ConfigState`.

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

The example validates the integration path. It is not a business task template. Application workers should live inside the `ChildSpec` and `TaskFactory` boundaries instead of being started as unmanaged background tasks.

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
