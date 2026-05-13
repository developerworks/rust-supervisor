# rust-tokio-supervisor

`rust-tokio-supervisor` is the crates.io package for the rust-supervisor project. It is a Rust task supervision core for Tokio services. It provides declarative supervisor trees, child lifecycle governance, restart policies, four-stage shutdown, current state queries, event journal storage, and observability signals.

Terminology: rust-config-tree v0.1.9 is the centralized configuration loader, and Shutdown Without Orphaned Tasks is the formal shutdown term.

Package name: `rust-tokio-supervisor`. Library crate name: `rust_supervisor`.

## Capability Boundary

- Declare `ChildSpec` and `SupervisorSpec`.
- Start fresh futures through `TaskFactory` or `service_fn`.
- Use `OneForOne`, `OneForAll`, and `RestForOne` supervision strategies.
- Produce `RestartDecision` values from typed failures, backoff, jitter, fuse rules, and the policy engine.
- Control a running tree through `SupervisorHandle` operations such as `add_child`, `remove_child`, `restart_child`, `pause_child`, `resume_child`, `quarantine_child`, `shutdown_tree`, `current_state`, and `subscribe_events`.
- Load the primary YAML configuration from `examples/config/supervisor.yaml`.
- Reuse `rust_supervisor::config::configurable::SupervisorConfig` for YAML loading, template generation, and JSON Schema generation.
- Emit structured logs, tracing spans, metrics, audit events, event journal entries, and `RunSummary` diagnostics.
- Enable target-side dashboard IPC through the optional `ipc` configuration section. The target process owns only local Unix domain socket IPC, snapshot generation, event conversion, command mapping, and shared JSON contracts.

## Dashboard Boundary

The supervisor dashboard feature uses three directories.

- `/Users/0x00/Documents/rust-supervisor`: target process IPC and shared contracts.
- `/Users/0x00/Documents/rust-supervisor-relay`: relay server, dynamic registration, `wss://`, mTLS, session gating, and command audit.
- `/Users/0x00/Documents/rust-supervisor-ui`: Vue, shadcn-vue, Tailwind dashboard client.

The target process does not expose IPC to the network. It opens a local Unix domain socket only when `ipc.enabled=true`. A relay can read snapshots and can request `events.subscribe` or `logs.tail`, but those subscriptions must be triggered by an established remote dashboard session.

## No Compatibility

No Compatibility: this crate is a new project with no legacy API aliases. Consumers should import public types from their owning module paths, for example `rust_supervisor::runtime::supervisor::Supervisor`.

## Configuration Schema

`SupervisorConfig` is the public root configuration struct. It supports `confique::Config`, `schemars::JsonSchema`, `serde::Serialize`, and `serde::Deserialize`, so users can reuse one model for YAML loading, template generation, and schema generation.

The official YAML files stay single-file by default:

- `examples/config/supervisor.yaml`: complete runnable configuration.
- `examples/config/supervisor.template.yaml`: complete single-file template.

This crate does not bake in `x-tree-split`. Projects that want split configuration files can wrap or reuse `SupervisorConfig` in their own crate and decide their own tree split layout.

The optional dashboard IPC section has this shape:

```yaml
ipc:
  enabled: true
  target_id: payments-worker-a
  path: /run/rust-supervisor/payments-worker-a.sock
  permissions: "0600"
  bind_mode: create_new
  registration:
    enabled: true
    relay_registration_path: /run/rust-supervisor/dashboard-relay-registration.sock
    display_name: "payments worker a"
    lease_seconds: 30
    registration_heartbeat_interval_seconds: 15
```

When `ipc.enabled=true`, `ipc.path` and `ipc.registration.relay_registration_path` must be absolute local paths. Registration uses dynamic registration. The relay configuration must not hard-code target lists.

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
    let handle = Supervisor::start_from_config_file("examples/config/supervisor.yaml").await?;
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

Dashboard validation spans all three directories:

```bash
cargo test
cargo test --manifest-path /Users/0x00/Documents/rust-supervisor-relay/Cargo.toml
npm --prefix /Users/0x00/Documents/rust-supervisor-ui install
npm --prefix /Users/0x00/Documents/rust-supervisor-ui run test
npm --prefix /Users/0x00/Documents/rust-supervisor-ui run build
npm --prefix /Users/0x00/Documents/rust-supervisor-ui run test:e2e
```

Engineering gate details are documented in `docs/en/quality-gates.md` and `docs/zh/quality-gates.md`. Parallel implementation governance is documented in `docs/en/parallel-governance.md` and `docs/zh/parallel-governance.md`.

## License

This project is licensed under the MIT license. See `LICENSE`.
