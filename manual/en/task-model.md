# Task Model

## Task Kinds

`TaskKind` distinguishes `AsyncWorker`, `BlockingWorker`, and `Supervisor`. A blocking worker must not be treated as a normal asynchronous worker that can always be aborted immediately.

## Task Factory

`TaskFactory` is the core construction contract. Every attempt must create a fresh future. `service_fn` is an ergonomic adapter that still targets `TaskFactory`; it does not replace the core model.

`TaskResult` distinguishes `Succeeded`, `Cancelled`, and `Failed`. The `Failed` variant carries `TaskFailure` and `TaskFailureKind`.

## Task Context

`TaskContext` contains child identifier, supervisor path, generation, attempt, cancellation token, heartbeat sender, and readiness sender.

Workers should use `TaskContext::heartbeat` to report health, `TaskContext::mark_ready` to report explicit readiness, and `TaskContext::is_cancelled` or `TaskContext::cancellation_token` to react to shutdown.

## Readiness

`ReadinessPolicy` supports `Immediate` and `Explicit`. An explicitly ready child should not appear as ready in current state or events until it reports readiness.
