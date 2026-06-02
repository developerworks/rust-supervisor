# Service Role Example

[中文说明](README.zh.md)

This example shows a supervised `TaskRole::Service` child. A service role is a long-running task that should stay online, report readiness, emit heartbeats, and stop cooperatively when the supervisor shuts down.

Run it with:

```bash
cargo run --package rust-tokio-supervisor --example service
```

The process keeps running until you press `Ctrl+C`. The signal is treated as the operator stop request, then the example calls `shutdown_tree` and prints the graceful shutdown outcome.

## What It Shows

- Initialization: `service_task.rs` builds a `quote-service` child and marks it as ready.
- Running: the service prints the current UNIX time once per second, then emits heartbeat and running tick facts.
- Observation: `observation.rs` prints periodic `current_state`, runtime event text, and shutdown report data.
- Stop: after `Ctrl+C`, `main.rs` calls `shutdown_tree`, cancellation reaches the service, and the service returns `TaskResult::Cancelled`.

## File Layout

- `main.rs`: wires the supervisor, event subscriptions, state snapshots, and shutdown command.
- `service_task.rs`: declares the `TaskRole::Service` child and the async service body.
- `observation.rs`: prints service facts, runtime events, current state records, and shutdown outcomes.

## Expected Output Shape

The output should include these stages:

```text
service initialization: initialized child=quote-service path=/quote-service
state after-initialization: child_count=1 shutdown_completed=false
service example: running until Ctrl+C
service business: child=quote-service tick=... now_unix=...
service while-running: running child=quote-service tick=...
operator signal=ctrl_c
service during-stop: stopping child=quote-service
shutdown outcome: child=quote-service status=Graceful phase=GracefulDrain cancel_delivered=true
state after-shutdown: child_count=1 shutdown_completed=true
```

The important behavior is that the service does not exit by itself. It stays active while running, reports readiness and heartbeat status through `current_state`, receives cancellation only after the operator signal, and finishes as a graceful child shutdown outcome.
