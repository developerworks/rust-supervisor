# Split Configuration and Transparent Array Sections

Language: [中文](../zh/split-config.html)

## Overview

`groups` and `children` on `SupervisorConfig` use **transparent array sections**. They appear as YAML arrays on disk, are stored in Rust behind an `items` field, and are loaded or templated by `rust-config-tree` 0.3.0.

In short: use `children: [...]` in a single file, or write only `[...]` in a split file. Do not wrap split bodies with `items:`.

## Fields

| Field | Rust type | Split file | Section schema top-level type |
|-------|-----------|------------|-------------------------------|
| `groups` | `GroupsConfigSection` | `groups.yaml` | `array` |
| `children` | `ChildrenConfigSection` | `children.yaml` | `array` |

Access items with:

```rust
config.children.len();
config.children.as_slice();
config.groups.as_slice();
```

## Single-file layout

```yaml
groups:
  - name: core
children:
  - name: api
    kind: async_worker
```

## Split layout

Root config:

```yaml
include:
  - groups.yaml
  - children.yaml
supervisor:
  strategy: OneForAll
policy:
  child_restart_limit: 10
  # ... remaining policy / shutdown / observability fields
```

Split files contain **only the array body**:

```yaml
# groups.yaml
- name: core
  budget:
    window_secs: 60
    max_burst: 10
    recovery_rate_per_sec: 0.5
```

```yaml
# children.yaml
- name: worker
  kind: async_worker
  criticality: optional
  restart_policy: permanent
```

Repository references:

- Generated template tree: `config/supervisor_config/`
- Example inputs: `examples/config/split/`
- Runnable example: `cargo run --example split_config_supervisor`

## Supported YAML shapes

The loader accepts all three forms:

| Shape | Example |
|-------|---------|
| Transparent array | `children: [{ name: api }]` |
| Body-only split file | `children.yaml` contains only `- name: api` |
| Legacy `items` wrapper | `children:\n  items: [...]` |

Do not use flow-style `[{ name: worker }]`. Template generation emits block YAML.

## CLI

Use the `rust-tokio-supervisor` binary or `cargo run`. Subcommands are top-level; there is no `config` prefix. `--config` belongs to the `run` and `validate-config` subcommands, not the global CLI.

```bash
# Validate and print a summary (default `run` command)
cargo run -- run --config examples/config/supervisor.yaml

# Validate the full config tree (includes, defaults, runtime validation)
cargo run -- validate-config --config examples/config/split/supervisor.yaml

# Generate split templates (includes groups.yaml / children.yaml)
# Template source defaults to examples/config/supervisor.yaml
cargo run -- generate-template \
  --output config/supervisor_config/supervisor_config.example.yaml

# Generate JSON Schemas
cargo run -- generate-schema \
  --output config/supervisor_config/supervisor.schema.json
```

No post-processing is required after generation. The library strips section root keys and rewrites flow arrays to block YAML.

## Loading in code

```rust
use rust_config_tree::config::load_config;
use rust_supervisor::config::{
    configurable::SupervisorConfig,
    loader::load_config_from_yaml_file,
};

let config = load_config::<SupervisorConfig>("supervisor.yaml")?;
let state = load_config_from_yaml_file("supervisor.yaml")?;
```

## Runtime defaults vs template samples

| Scenario | `children` at runtime |
|----------|------------------------|
| `children` omitted from every config file | `[]`; template sample `worker` is not injected |
| `generate-template` command | `children.yaml` may include a `worker` sample from `#[config(default = ...)]` |
| Body-only split file | Loads the array contents normally |

`groups` template defaults to `[]`.

## IDE completion

Bind the section schema in split files:

```yaml
# yaml-language-server: $schema=./children.schema.json

- name: worker
  kind: async_worker
```

`children.schema.json` is a top-level `array`, not an `{ items: [...] }` object.

## Reusing the pattern in another crate

To add another transparent array section:

1. Declare a dedicated struct per transparent array section with an `items: Vec<T>` field, and implement transparent-array `Serialize`, `Deserialize`, `JsonSchema`, and accessors (see `ChildrenConfigSection` and `GroupsConfigSection`).
2. Mark the root field with `#[schemars(extend("x-tree-split" = true, "x-tree-transparent-array" = true))]`.
3. Implement `ConfigSchema::include_paths`.
4. Use `load_config`, `write_config_templates`, and `write_config_schemas`.

See the `x-tree-transparent-array` section in the [rust-config-tree documentation](https://github.com/developerworks/rust-config-tree).
