# Supervisor Tree

Language: [中文](../zh/supervisor-tree.html)

## Declaration Model

`SupervisorSpec` describes one supervisor node. It contains `path`, `strategy`, `children`, `config_version`, default restart policy, default backoff policy, default health policy, default shutdown policy, supervisor-level fuse limits, `restart_budget`, `escalation_policy`, `group_strategies`, `child_strategy_overrides`, and `dynamic_supervisor_policy`.

`ChildSpec` describes one child. It contains `id`, `name`, `kind`, `factory`, `restart_policy`, `shutdown_policy`, `health_policy`, `readiness_policy`, `backoff_policy`, `dependencies`, `tags`, and `criticality`.

## Tree Building

`SupervisorTree::build` validates `SupervisorSpec` and converts children into path-aware nodes. Each child path is derived from the parent path and `ChildId`.

`SupervisorPath::root` returns the root path. `SupervisorPath::join` appends a child path segment. `SupervisorPath::parent` returns the parent path when it exists.

## Startup And Shutdown Order

`startup_order` returns nodes in declaration order. `shutdown_order` returns nodes in reverse declaration order. This ordering is the basis for Shutdown Without Orphaned Tasks.

## Restart Planning

`restart_execution_plan` resolves the runtime restart scope from the tree and `SupervisorSpec`. It keeps per-child overrides, group strategies, restart budgets, escalation policies, and dynamic supervisor policy in one plan so the runtime control loop does not duplicate strategy selection logic.

## Registry

`RegistryStore` stores `ChildRuntime` values by child identifier, supervisor path, and declaration order. Runtime control and current state queries should go through the registry instead of bypassing it.
