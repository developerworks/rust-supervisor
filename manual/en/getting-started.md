# Getting Started

## Prerequisites

This project is a Rust library. The examples require Cargo and a Tokio application environment. Repository examples include their required dependencies.

The primary configuration file is `examples/config/supervisor.yaml`. The loader uses rust-config-tree v0.1.9, reads YAML, and produces `ConfigState`.

## Minimal Command

```bash
cargo run --example supervisor_quickstart
```

The example loads YAML through `load_config_state`, derives `SupervisorSpec` through `ConfigState::to_supervisor_spec`, starts the runtime through `Supervisor::start`, queries `current_state`, and then shuts down the tree through `shutdown_tree`.

## Minimal Code Path

```rust
use rust_supervisor::config::loader::load_config_state;
use rust_supervisor::runtime::supervisor::Supervisor;

#[tokio::main]
async fn main() -> Result<(), rust_supervisor::error::types::SupervisorError> {
    let state = load_config_state("examples/config/supervisor.yaml")?;
    let spec = state.to_supervisor_spec()?;
    let handle = Supervisor::start(spec).await?;
    let current = handle.current_state().await?;
    println!("{current:#?}");
    handle.shutdown_tree("operator", "quickstart complete").await?;
    Ok(())
}
```

## Result

The example validates the integration path. It is not a business task template. Application workers should live inside the `ChildSpec` and `TaskFactory` boundaries instead of being started as unmanaged background tasks.
