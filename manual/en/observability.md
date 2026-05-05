# Observability

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

## Diagnostic Replay

The event journal stores a fixed number of recent events. `RunSummary` is built from the event journal, current state, and policy decisions so operators can explain meltdown, shutdown timeout, or parent escalation.
