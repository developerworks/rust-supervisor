# Policies

Language: [中文](../zh/policies.html)

## Supervision Strategy

`SupervisionStrategy` decides the restart scope after a failure. `OneForOne` selects only the failed child. `OneForAll` selects every child in the selected scope. `RestForOne` selects the failed child and every child declared after it in the selected scope.

`restart_scope` calculates the restart scope from `SupervisorTree`, the strategy, and the failed child identifier.

`restart_execution_plan` combines the supervisor strategy, `GroupStrategy`, `ChildStrategyOverride`, `RestartLimit`, `EscalationPolicy`, and `DynamicSupervisorPolicy` into a `StrategyExecutionPlan`. Child overrides take precedence over group strategies, and group strategies take precedence over the supervisor-wide strategy. The plan stores a `dynamic_supervisor_enabled` boolean; the full `DynamicSupervisorPolicy` struct (including `child_limit`) is evaluated by the control loop at add_child time, not embedded in the plan.

The runtime control loop now receives child exits and applies the selected `StrategyExecutionPlan` automatically when policy returns a restart decision. Runtime lifecycle events include restart scope information so operators can see the selected strategy, group, and child scope.

## Group Strategy And Overrides

`GroupStrategy` uses child `tags` to define a smaller restart scope. A child can belong to at most one configured strategy group. `ChildStrategyOverride` applies a per-child strategy and governance override when one child needs stricter restart behavior than its group or supervisor.

`GroupConfig` configures restart budget, membership, and isolation at the group level. `GroupDependencyEdge` defines cross-group dependency edges for fault propagation.

## Restart Limit And Escalation

`RestartLimit` records the maximum restart count and the counting window selected for a plan. `EscalationPolicy` records the follow-up action when restart governance cannot remain local, including parent escalation, tree shutdown, or scope quarantine.

## Dynamic Supervisor Policy

`DynamicSupervisorPolicy` controls runtime `add_child` acceptance. It is a stateless configuration object with `enabled` and `child_limit` fields. The control loop maintains the current child count externally and calls `allows_addition(current_child_count)` at add_child time. Additions are rejected when dynamic supervision is disabled or the configured child limit has already been reached.

## Restart Policy

`RestartPolicy` contains `Permanent`, `Transient`, and `Temporary`. `PolicyEngine` reads `TaskExit`, the failure category, and the restart policy, then returns `RestartDecision`.

## Backoff And Jitter

`BackoffPolicy` describes initial delay, maximum delay, and jitter ratio. There are two types with this name:

- `spec::child::BackoffPolicy` — used in `ChildSpec`, fields are `initial_delay`, `max_delay`, `jitter_ratio` (a 0.0-1.0 float ratio).
- `policy::backoff::BackoffPolicy` — used by the runtime policy engine, fields are `initial`, `max`, `jitter_mode` (an enum: `Disabled`, `Deterministic`, `FullJitter`, `DecorrelatedJitter`), `jitter_percent`, and `reset_after`.

Tests can use `JitterMode::Deterministic` so coverage does not depend on random output.

## Fuse And Quarantine

`MeltdownPolicy` limits restarts or failures inside configured windows at three levels: child, group, and supervisor. Crossing a child-level fuse places the child in quarantine. Crossing a group-level fuse escalates to the supervisor level. Crossing a supervisor-level fuse escalates the failure to the parent.

## Task Exit Classification

`TaskExit` distinguishes success, cancellation, typed failure, panic, and timeout. The policy layer reads typed classifications from `TaskFailureKind` (which includes `Panic`, `Timeout`, and typed failure categories) instead of inferring behavior from strings.
