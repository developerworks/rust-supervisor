# Observability

Language: [中文](../zh/observability.html)

## Event Model

`SupervisorEvent` describes one lifecycle fact. It contains `When`, `Where`, `What`, sequence, and correlation identifier.

`When` records wall-clock time, monotonic time, uptime, generation, and attempt. `Where` records supervisor path, child identifier, parent identifier, and task name. `What` records state transition, policy decision, health state, exit reason, or control command.

## Pipeline Outputs

The observability pipeline publishes the same lifecycle fact as these signals:

- `SupervisorEvent`.
- Structured log.
- Tracing span and tracing event.
- Metrics.
- Audit event.
- Event journal entry.
- Test recorder entry.

## Metric Labels

Metric labels must stay low-cardinality. Acceptable labels include supervisor path, child identifier, state, decision, and failure category. Full error text, user input, and unbounded dynamic values should not become labels.

## Real Shutdown Pipeline

After `ShutdownTree` runs the real shutdown pipeline, the observability pipeline must surface lifecycle facts for each stage. `ChildShutdownCancelDelivered` means the runtime delivered `CancellationToken` to the in-flight child attempt. `ChildShutdownGraceful` means the child task returned inside the graceful drain budget. `ChildShutdownAborted` means the runtime requested `abort` for a stuck task. `ChildShutdownLateReport` means the child task returned after the normal shutdown accounting window. `ShutdownCompleted` means the pipeline emitted the final reconcile report.

Metrics record shutdown facts with low-cardinality labels. `supervisor_shutdown_duration_seconds` measures full pipeline duration. `supervisor_shutdown_child_outcomes_total` counts outcomes by `status` and `phase` and must not place `child_id` on metric labels. `supervisor_shutdown_abort_total` counts abort paths by bounded reason. `supervisor_shutdown_late_reports_total` counts late reports by `phase`.

Audit events record cancel delivered, graceful outcome, abort outcome, late report, and completed reconcile. When the core runtime does not own the dashboard IPC socket, the reconcile report records socket status as `NotOwned`.

## Diagnostic Replay

The event journal stores a fixed number of recent events. `RunSummary` is built from the event journal, current state, and policy decisions so operators can explain meltdown, shutdown timeout, or parent escalation.
