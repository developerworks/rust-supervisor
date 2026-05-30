# Frequently Asked Questions (FAQ)

Language: [中文](../zh/faq.html)

## Basics

### What is the difference between ChildDeclaration and ChildSpec?

`ChildDeclaration` is the **input model** used in YAML configuration and `add_child` RPC payloads. It focuses on serializable, validatable declarations. `ChildSpec` is the **runtime model** used by the supervisor to register, start, and restart children. It carries resolved `ChildId`, `Arc<dyn TaskFactory>`, and materialized policy objects.

See [ChildSpec and ChildDeclaration](child-spec.md) for details.

### What are the entry methods after Supervisor starts?

`Supervisor` provides 3 entry methods:

| Method                                       | Input                             | When to use                     |
| -------------------------------------------- | --------------------------------- | ------------------------------- |
| `Supervisor::start(spec)`                    | `SupervisorSpec` (pre-built spec) | Programmatic startup            |
| `Supervisor::start_from_config_state(state)` | `ConfigState` (validated config)  | Start from config loader output |
| `Supervisor::start_from_config_file(path)`   | YAML file path                    | Start directly from YAML file   |

All three converge into `start_with_policy()`, which validates, creates channels, spawns the control loop, and returns a `SupervisorHandle`.

### What does "Shutdown Without Orphaned Tasks" mean?

This is the core shutdown goal of the project. After the root supervisor completes shutdown, no orphan tasks may remain in the runtime. This is achieved through the four-stage shutdown protocol (request stop -> graceful drain -> abort stragglers -> reconcile) and by shutting down children in reverse declaration order, ensuring every child is properly terminated.

## Configuration

### What child fields does the YAML `children` entry support?

`children` is a YAML array backed by `ChildrenConfigSection` in Rust. Access items with `.as_slice()`. Each declaration supports these fields:

| Category            | Field                 | Description                                         |
| ------------------- | --------------------- | --------------------------------------------------- |
| Identity            | `name`                | Child name, required, non-empty                     |
| Kind                | `kind`                | `async_worker`, `blocking_worker`, or `supervisor`  |
| Criticality         | `criticality`         | `critical` or `optional`                            |
| Restart policy      | `restart_policy`      | `permanent`, `transient`, or `temporary`            |
| Dependencies        | `dependencies`        | List of dependent child names                       |
| Health check        | `health_check`        | Health check interval, timeout, etc.                |
| Readiness           | `readiness`           | Explicit readiness check config                     |
| Resource limits     | `resource_limits`     | CPU, memory and other resource constraints          |
| Command permissions | `command_permissions` | Commands this child is allowed to execute           |
| Environment         | `environment`         | Key-value environment variable list                 |
| Secrets             | `secrets`             | `${SECRET_NAME}`-format secret references           |
| Tags                | `tags`                | Low-cardinality grouping tags                       |
| Task role           | `task_role`           | `service`, `worker`, `job`, `sidecar`, `supervisor` |

See [Configuration](configuration.md#example-configuration) for a complete config sample.

### How do I split `groups` and `children` into separate YAML files?

Add `include` in the root config and write body-only split files:

```yaml
include:
  - groups.yaml
  - children.yaml
```

```yaml
# children.yaml
- name: worker
  kind: async_worker
```

See [Split Configuration and Transparent Array Sections](split-config.md). Run `cargo run --example split_config_supervisor`.

### What happens when `children` is omitted from a config file?

Runtime loading yields an empty list `[]`. Template sample entries such as `worker` are not injected at runtime. Only `generate-template` writes sample entries.

### What configurations cause rejection at startup?

Configuration loading returns `SupervisorError::FatalConfig` when startup must be rejected. Rejection reasons include:

- The file is not YAML format or cannot be read
- Supervision strategy is not `OneForOne`, `OneForAll`, or `RestForOne`
- Numeric values are zero or out of valid range
- Initial backoff is greater than max backoff
- Jitter ratio is not between 0.0 and 1.0
- Restart budget, failure window, or meltdown config is invalid
- Child declaration has circular dependencies
- Child ID or name is empty
- Sidecar task role is missing `sidecar_config`
- Dashboard IPC path is not absolute

See [Configuration](configuration.md#error-boundary) for the full rejection list.

## Runtime Control

### What is the five-step add_child transaction?

`add_child` chains five steps into a single transaction:

1. **Parse**: Deserialize the RPC payload into a `ChildDeclaration`
2. **Validate**: Run `validate_child_declaration`, checking name format, dependency name existence, secret placeholder syntax, etc.
3. **Register**: Update topology, insert the new child into the registry, and run cycle detection
4. **Launch**: Create and start the child future via `TaskFactory`
5. **Audit Persist**: Write audit records including the declaration SHA-256 hash

If any step fails, the entire transaction rolls back to the pre-call topology view, or writes a compensating record for post-recovery handling.

### Which runtime control commands are idempotent?

Repeated control commands do not create unrecoverable errors:

- Pausing an already paused child returns the current state
- Quarantining an already quarantined child returns the current state
- Calling shutdown after shutdown is complete returns the existing result
- `join` caches the final `RuntimeExitReport`; repeated calls return the same result

### What is the difference between pause, quarantine, and remove?

All three are stop-type control commands, but they behave differently:

| Command            | `operation` set to | Record kept                            | Auto-restart           |
| ------------------ | ------------------ | -------------------------------------- | ---------------------- |
| `pause_child`      | `Paused`           | Kept                                   | Suspended while paused |
| `quarantine_child` | `Quarantined`      | Kept                                   | Disabled permanently   |
| `remove_child`     | `Removed`          | Physically deleted after attempt exits | N/A                    |

Pause can be resumed via `resume_child`. Quarantined children can be removed later. Remove is final — the runtime record is physically deleted.

## Policies & Failure Handling

### When should each RestartPolicy value be used?

| Value       | Behavior                                    | When to use                                                |
| ----------- | ------------------------------------------- | ---------------------------------------------------------- |
| `Permanent` | Always restart                              | Critical services like API servers, database connections   |
| `Transient` | Restart only for certain failure categories | Restart on external dependency failures, not on fatal bugs |
| `Temporary` | Restart at most once                        | One-shot jobs, do not retry after failure                  |

### How do the three meltdown levels cascade?

The meltdown policy (`MeltdownPolicy`) limits restarts or failures within a window, across three levels:

1. **Child-level**: Exceeds `child_max_restarts` / `child_window_secs` -> enters quarantine
2. **Group-level**: Exceeds `group_max_failures` / `group_window_secs` -> escalates to supervisor
3. **Supervisor-level**: Exceeds `supervisor_max_failures` / `supervisor_window_secs` -> escalates to parent

After meltdown triggers, it auto-resets after `reset_after_secs`.

## Observability

### How do I subscribe to lifecycle events?

Call `SupervisorHandle::subscribe_events()` to get a `broadcast::Receiver`. Events are of type `SupervisorEvent`, containing `When` (wall time, monotonic time, uptime, generation, attempt), `Where` (supervisor path, child ID, task name), and `What` (state transitions, policy decisions, health status, exit reasons, or control commands).

### What happens when the event journal is full?

The event journal is a fixed-capacity ring buffer. When full, it overwrites the oldest entries. Capacity is configured via `observability.event_journal_capacity`. However, the add_child-dedicated audit channel does **not** silently overwrite — it returns `Err(AuditStorageFailure)` when full.

## Dashboard

### Which three repositories does the Dashboard feature require?

The dashboard feature spans three repositories:

| Repository                       | Responsibility                                |
| -------------------------------- | --------------------------------------------- |
| `rust-supervisor` (this project) | Target process local IPC and shared contracts |
| `~/rust-supervisor-relay`        | Relay and external `wss://` sessions          |
| `~/rust-supervisor-ui`           | Browser dashboard client                      |

The target process exposes only a local Unix domain socket. IPC must never be exposed to external networks.

### What IPC methods are supported?

Supported methods: `hello`, `state`, `events.subscribe`, `logs.tail`, `command.restart_child`, `command.pause_child`, `command.resume_child`, `command.quarantine_child`, `command.remove_child`, `command.add_child`, and `command.shutdown_tree`.

## Project & Build

### What does `target/debug/rust-tokio-supervisor generate-template` do without arguments?

`generate-template` with no arguments **does not output to stdout**. It writes to `config/<root-config-name>/<root-config-name>.example.yaml` by default.

For this project:

```bash
# No terminal output after running
./target/debug/rust-tokio-supervisor generate-template

# But files are actually written
ls config/supervisor_config/
# supervisor_config.example.yaml
# supervisor_config.schema.json
```

Options:

```bash
# Specify output path
./target/debug/rust-tokio-supervisor generate-template --output /tmp/my-config.yaml

# Also generate JSON Schema
./target/debug/rust-tokio-supervisor generate-template --schema /tmp/schema.json
```

The output format is inferred from the file extension; unknown or missing extensions use YAML by default.

### Why does `Cargo.toml` declare only one `[[bin]]` (rust-tokio-supervisor) but there are multiple binaries in `target/debug/`?

Cargo supports two ways to declare binary targets:

1. **Explicit declaration**: via `[[bin]]` entries in `Cargo.toml`, e.g., `src/main.rs` -> `rust-tokio-supervisor`
2. **Auto-discovery**: each `.rs` file in `src/bin/` automatically becomes a binary target, using the filename as the target name

So `Cargo.toml` shows only `[[bin]] name = "rust-tokio-supervisor"`, but `src/bin/generate_supervisor.rs` and `src/bin/generate_supervisor_config.rs` are auto-discovered by Cargo, producing additional binaries.

> Note: The `src/bin/` directory may be cleaned up or moved after feature completion to keep the project structure tidy.

## Common Errors

### What is `SupervisorError::FatalConfig`?

`FatalConfig` indicates an unrecoverable error during configuration loading. The error includes `field_path` (JSON Pointer format) and a human-readable `hint` to help locate the specific problem.

### What should I do when add_child returns `Err(SupervisorShuttingDown)`?

The supervisor is currently executing its shutdown sequence and cannot accept new add_child requests. Wait for the supervisor to complete shutdown, restart it, then retry the add operation.

### What should I do when add_child returns `Err(ChildLimitExceeded)`?

The runtime child count has reached its limit (currently 1000). Either remove unnecessary children via `remove_child`, or adjust the `dynamic_supervisor.child_limit` configuration.

### What happens when audit storage fails?

When the audit channel (ring buffer) write fails:

- add_child enters compensating flow and returns `Err(AuditStorageFailure)`
- The topology view rolls back to its pre-call state
- No orphaned semi-parsed state is left behind
