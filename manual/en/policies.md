# Policies

## Supervision Strategy

`SupervisionStrategy` decides the restart scope after a failure. `OneForOne` selects only the failed child. `OneForAll` selects every child in the same group. `RestForOne` selects the failed child and every child declared after it.

`restart_scope` calculates the restart scope from `SupervisorTree`, the strategy, and the failed child identifier.

## Restart Policy

`RestartPolicy` contains `Permanent`, `Transient`, and `Temporary`. `PolicyEngine` reads `TaskExit`, the failure category, and the restart policy, then returns `RestartDecision`.

## Backoff And Jitter

`BackoffPolicy` describes initial delay, maximum delay, jitter mode, and reset-after behavior. Tests can use deterministic jitter so coverage does not depend on random output.

## Fuse And Quarantine

`MeltdownPolicy` limits restarts or failures inside configured windows. Crossing a child-level fuse places the child in quarantine. Crossing a supervisor-level fuse escalates the failure to the parent.

## Task Exit Classification

`TaskExit` distinguishes success, cancellation, typed failure, panic, and timeout. The policy layer must read typed classifications instead of inferring behavior from strings.
