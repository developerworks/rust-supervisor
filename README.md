# rust-supervisor

`rust-supervisor` is a Rust task supervision core for Tokio services. It provides declarative supervisor trees, child lifecycle governance, restart policies, four-stage shutdown, current state queries, event journal storage, and observability signals.

Terminology: rust-config-tree v0.1.9 is the centralized configuration loader, and Shutdown Without Orphaned Tasks is the formal shutdown term.

## Capability Boundary

- Declare `ChildSpec` and `SupervisorSpec`.
- Start fresh futures through `TaskFactory` or `service_fn`.
- Use `OneForOne`, `OneForAll`, and `RestForOne` supervision strategies.
- Produce `RestartDecision` values from typed failures, backoff, jitter, fuse rules, and the policy engine.
- Control a running tree through `SupervisorHandle` operations such as `add_child`, `remove_child`, `restart_child`, `pause_child`, `resume_child`, `quarantine_child`, `shutdown_tree`, `current_state`, and `subscribe_events`.
- Load the primary YAML configuration from `examples/config/supervisor.yaml`.
- Emit structured logs, tracing spans, metrics, audit events, event journal entries, and `RunSummary` diagnostics.

## No Compatibility

No Compatibility: this crate is a new project with no legacy API aliases. Consumers should import public types from their owning module paths, for example `rust_supervisor::runtime::supervisor::Supervisor`.

## Quick Start

```bash
cargo run --example supervisor_quickstart
```

The example follows this path:

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

## Examples

```bash
cargo run --example supervisor_quickstart
cargo run --example config_tree_supervisor
cargo run --example restart_policy_lab
cargo run --example shutdown_tree
cargo run --example observability_probe
cargo run --example supervisor_tree_story
cargo run --example runtime_control_story
cargo run --example policy_failure_matrix
cargo run --example diagnostic_replay
```

## Manuals

- `manual/en/index.md`: English user manual.
- `manual/zh/index.md`: Chinese user manual.
- `docs/en/index.md`: English engineering documentation.
- `docs/zh/index.md`: Chinese engineering documentation.

## Quality Gates

```bash
cargo fmt --check
cargo check
cargo test
cargo doc --no-deps
cargo package --list
scripts/check-coding-standard.sh
scripts/check-maintainability.sh
scripts/generate-sbom.sh
scripts/validate-sbom.sh
cargo publish --dry-run
```

Engineering gate details are documented in `docs/en/quality-gates.md` and `docs/zh/quality-gates.md`. Parallel implementation governance is documented in `docs/en/parallel-governance.md` and `docs/zh/parallel-governance.md`.

## License

This project is licensed under the MIT license. See `LICENSE`.
