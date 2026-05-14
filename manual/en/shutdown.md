# Shutdown

Language: [中文](../zh/shutdown.html)

## Formal Term

This project uses Shutdown Without Orphaned Tasks to describe the shutdown goal. After root shutdown completes, the runtime should leave no orphaned task.

## Four Stages

The shutdown protocol has four stages:

- Request stop: accept the shutdown cause and propagate the cancellation token.
- Graceful drain: wait for each child to finish on its own.
- Abort stragglers: force or escalate asynchronous tasks that exceed their timeout.
- Reconcile: align registry state, current state, metrics, and the event journal.

## Order

Startup runs in declaration order. Shutdown runs in reverse declaration order. `startup_order` and `shutdown_order` expose this rule.

## Blocking Worker Boundary

`BlockingWorker` represents `spawn_blocking` work or other work that cannot be assumed to abort immediately. After shutdown timeout, the runtime should record the non-immediate termination boundary and follow the escalation policy.

## Shutdown Cause

`ShutdownCause` records `requested_by` and `reason`. The cause should appear in audit and diagnostic output.
