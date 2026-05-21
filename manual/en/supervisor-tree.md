# Supervisor Tree

Language: [中文](../zh/supervisor-tree.html)

## Declaration Model

`SupervisorSpec` describes one supervisor node. It contains:

- `path` — stable path for this supervisor
- `strategy` — restart scope strategy (`OneForOne`, `OneForAll`, `RestForOne`)
- `children` — child specifications in declaration order
- `config_version` — configuration version that produced this spec
- `default_restart_policy`, `default_backoff_policy`, `default_health_policy`, `default_shutdown_policy` — policies inherited by children that do not override
- `supervisor_failure_limit` — maximum supervisor failures before parent escalation
- `restart_limit` — optional supervisor-level restart limit
- `escalation_policy` — optional supervisor-level escalation policy
- `group_strategies` — group-level strategy overrides
- `group_configs` — group-level restart budget, membership, and isolation configs
- `group_dependencies` — cross-group dependency edges for fault propagation
- `severity_defaults` — default severity class per work role for escalation bifurcation
- `child_strategy_overrides` — per-child strategy and governance overrides
- `dynamic_supervisor_policy` — runtime add_child acceptance policy
- `control_channel_capacity` — mpsc command channel capacity
- `event_channel_capacity` — broadcast event channel capacity

`ChildSpec` describes one child. It contains:

- `id`, `name`, `kind` — stable identity and task kind
- `factory` — optional `Arc<dyn TaskFactory>` for worker children
- `restart_policy`, `shutdown_policy`, `health_policy`, `readiness_policy`, `backoff_policy` — per-child policy overrides
- `dependencies` — child IDs that must become ready before this child starts
- `tags` — low-cardinality grouping labels
- `criticality` — `Critical` or `Optional`
- `work_role` — optional `WorkRole` that selects default lifecycle policy semantics
- `sidecar_config` — optional sidecar binding (required when role is `Sidecar`)
- `severity` — optional explicit severity override
- `group` — optional group name for group-level isolation and budget tracking
- `health_check`, `readiness` — optional health/readiness check configurations
- `resource_limits` — optional resource limits
- `command_permissions` — command permissions granted to this child
- `environment`, `secrets` — environment variables and secret references

## Tree Building

`SupervisorTree::build` validates `SupervisorSpec` and converts children into path-aware nodes. Each child path is derived from the parent path and `ChildId`.

`SupervisorPath::root` returns the root path. `SupervisorPath::join` appends a child path segment. `SupervisorPath::parent` returns the parent path when it exists.

## Startup And Shutdown Order

`startup_order` returns nodes in declaration order. `shutdown_order` returns nodes in reverse declaration order. This ordering is the basis for Shutdown Without Orphaned Tasks.

## Restart Planning

`restart_execution_plan` resolves the runtime restart scope from the tree and `SupervisorSpec`. It keeps per-child overrides, group strategies, restart limits, escalation policies, and dynamic supervisor policy in one plan so the runtime control loop does not duplicate strategy selection logic.

## Registry

`RegistryStore` stores `ChildRuntime` values by child identifier, supervisor path, and declaration order. Runtime control and current state queries should go through the registry instead of bypassing it.
