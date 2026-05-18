# Policies

Language: [中文](../zh/policies.html)

## Supervision Strategy

`SupervisionStrategy` decides the restart scope after a failure. `OneForOne` selects only the failed child. `OneForAll` selects every child in the selected scope. `RestForOne` selects the failed child and every child declared after it in the selected scope.

`restart_scope` calculates the restart scope from `SupervisorTree`, the strategy, and the failed child identifier.

`restart_execution_plan` combines the supervisor strategy, `GroupStrategy`, `ChildStrategyOverride`, `RestartLimit`, `EscalationPolicy`, and `DynamicSupervisorPolicy` into a `StrategyExecutionPlan`. Child overrides take precedence over group strategies, and group strategies take precedence over the supervisor-wide strategy.

The runtime control loop now receives child exits and applies the selected `StrategyExecutionPlan` automatically when policy returns a restart decision. Runtime lifecycle events use `restart_plan` so operators can see the selected strategy, group, and child scope.

## Group Strategy And Overrides

`GroupStrategy` uses child `tags` to define a smaller restart scope. A child can belong to at most one configured strategy group. `ChildStrategyOverride` applies a per-child strategy and governance override when one child needs stricter restart behavior than its group or supervisor.

## Restart Limit And Escalation

`RestartLimit` records the maximum restart count and the counting window selected for a plan. `EscalationPolicy` records the follow-up action when restart governance cannot remain local, including parent escalation, tree shutdown, or scope quarantine.

## Dynamic Supervisor Policy

`DynamicSupervisorPolicy` controls runtime `add_child` acceptance. The current command accepts child manifests and tracks the dynamic manifest count. It rejects additions when dynamic supervision is disabled or the configured child limit has already been reached.

## Restart Policy

`RestartPolicy` contains `Permanent`, `Transient`, and `Temporary`. `PolicyEngine` reads `TaskExit`, the failure category, and the restart policy, then returns `RestartDecision`.

## Backoff And Jitter

`BackoffPolicy` describes initial delay, maximum delay, jitter mode, and reset-after behavior. Tests can use deterministic jitter so coverage does not depend on random output.

## Fuse And Quarantine

`MeltdownPolicy` limits restarts or failures inside configured windows. Crossing a child-level fuse places the child in quarantine. Crossing a supervisor-level fuse escalates the failure to the parent.

## Task Exit Classification

`TaskExit` distinguishes success, cancellation, typed failure, panic, and timeout. The policy layer must read typed classifications instead of inferring behavior from strings.
