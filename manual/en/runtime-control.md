# Runtime Control

Language: [中文](../zh/runtime-control.html)

## Control Entry Point

`SupervisorHandle` is the runtime control entry point. It sends requests to the runtime control loop through a command channel and returns `CommandResult`.

## Control Commands

- `add_child`: accept a dynamic child manifest when `DynamicSupervisorPolicy` allows another child.
- `remove_child`: mark the target child runtime state record as `Removed`, deliver cancellation to the active attempt, and remove the runtime state record after the attempt exits.
- `restart_child`: request a restart for the target child.
- `pause_child`: mark the target child runtime state record as `Paused`, deliver cancellation to the active attempt, and pause automatic restarts.
- `resume_child`: resume governance for the target child.
- `quarantine_child`: mark the target child runtime state record as `Quarantined`, deliver cancellation to the active attempt, and block automatic restarts.
- `shutdown_tree`: shut down the whole supervisor tree.
- `current_state`: return the current `SupervisorState` and expose each child runtime fact through `CurrentState.child_runtime_records`.
- `subscribe_events`: subscribe to lifecycle events.
- `is_alive`: quickly check whether the runtime control loop can still accept ordinary control commands.
- `health`: return `RuntimeHealthReport`, including control-plane state, start time, latest observation time, and final failure reason.
- `join`: wait until the runtime control plane reaches a final state and repeatedly return the same `RuntimeExitReport`.
- `shutdown`: shut down only the runtime control plane. It does not replace `shutdown_tree`.

## Child Runtime State Control

`PauseChild`, `RemoveChild`, and `QuarantineChild` are stop-style control commands defined by this feature. All 3 commands return `CommandResult::ChildControl`, and the result contains `ChildControlResult`. The old `CommandResult::ChildState` shape is no longer part of the public result model.

`PauseChild` writes `ChildRuntimeState.operation` as `Paused`. If an active attempt exists, the runtime control loop delivers cancellation to that attempt and moves stop progress to `CancelDelivered`. While the child is paused, the supervision strategy does not automatically restart that child.

`RemoveChild` writes `ChildRuntimeState.operation` as `Removed`. If an active attempt exists, the runtime control loop first delivers cancellation and then physically removes the record from `child_runtime_states` after the attempt exits. If no active attempt exists, the runtime control loop returns a `NoActiveAttempt` result and then removes the runtime state record.

`QuarantineChild` writes `ChildRuntimeState.operation` as `Quarantined`. If an active attempt exists, the runtime control loop delivers cancellation. The quarantined runtime state record remains visible, but the supervision strategy no longer automatically restarts that child. An operator can still run `RemoveChild` later.

These 3 stop-style control commands do not synchronously wait for the child future to end. If a child ignores cancellation for too long, a later `CurrentState` call or repeated stop-style command triggers `reconcile_stop_deadlines` and exposes the stop failure through `ChildControlFailure`.

`CurrentState` returns `child_runtime_records`. Each `ChildRuntimeRecord` is ordered by declaration order. Construction performs only non-blocking reads, does not wait for a child future, and does not perform extra I/O. This collection is the main entry point for reading runtime state facts.

`RestartChild` and `ResumeChild` remain existing commands. This feature only requires them not to corrupt runtime state facts. It does not define new lifecycle semantics for them.

See the full contract in [`child-runtime-state-control.md`](../../specs/004-3-child-runtime-state-control/contracts/child-runtime-state-control.md).

## `ChildControlResult` Fields

- `child_id`: stable identifier of the controlled child.
- `attempt`: active attempt targeted by the command. It is `None` when no active attempt exists.
- `generation`: generation targeted by the command. It is `None` when no active attempt exists.
- `operation_before`: `ChildControlOperation` observed when the command arrived.
- `operation_after`: `ChildControlOperation` after command handling.
- `status`: current `ChildAttemptStatus` for the attempt. It is `None` when no active attempt exists.
- `cancel_delivered`: whether this command actually delivered cancellation.
- `stop_state`: `ChildStopState` after command handling.
- `restart_limit`: current `RestartLimitState`, including window, limit, used count, remaining count, and exhaustion flag.
- `liveness`: current `ChildLivenessState`, including last heartbeat time, heartbeat stale flag, and readiness.
- `idempotent`: whether this command reused an already existing target state.
- `failure`: current control failure. It is `None` when no failure exists.

## `ChildRuntimeRecord` Fields

- `child_id`: stable identifier of the child represented by this runtime state record.
- `path`: child path in the supervisor tree.
- `generation`: current active generation. It is `None` when no active attempt exists.
- `attempt`: current active attempt. It is `None` when no active attempt exists.
- `status`: current `ChildAttemptStatus` for the attempt.
- `operation`: current `ChildControlOperation`, which can be `Active`, `Paused`, `Quarantined`, or `Removed`.
- `liveness`: current `ChildLivenessState`.
- `restart_limit`: current `RestartLimitState`.
- `stop_state`: current `ChildStopState`.
- `failure`: most recent `ChildControlFailure`. When `stop_state` is `Failed`, this must be `Some`.

## Idempotent Behavior

Repeated control commands should not create unrecoverable errors. Pausing an already paused child returns the current state. Quarantining an already quarantined child returns the current state. Shutting down an already completed tree returns the existing shutdown result.

`join` caches the final `RuntimeExitReport` from the control loop. Repeated calls to `join` on the same handle return the same result every time and do not consume the underlying `JoinHandle` again.

`shutdown` only asks the runtime control loop to exit normally. If the control plane has already completed or failed, another `shutdown` call directly returns the existing final report. `shutdown_tree` remains responsible for child task and full supervisor tree shutdown semantics.

## Runtime Health

`is_alive` is a low-cost state check. It returns `true` when the control plane is alive. It returns `false` when the control plane is starting, shutting down, completed, or failed.

`health` returns structured state. After an abnormal control-plane exit, `health` can still read the failed state, failure phase, reason, panic flag, and recoverable flag. Ordinary control commands after the control plane has ended return `SupervisorError` with the same exit reason.

## Dynamic Additions

Dynamic additions are governed before the manifest is accepted. The runtime rejects `add_child` when dynamic supervision is disabled or when the declared child count plus dynamic child count has reached the configured limit. `current_state.child_count` includes accepted dynamic manifests.

## Audit Data

Each control command carries `requested_by`, `reason`, `target_path`, `accepted_at`, and `command_id`. These fields support audit events and incident review.

`requested_by` and `reason` must be non-empty text. `SupervisorHandle` rejects empty values before the command enters the channel, and the runtime control loop validates them again before execution. This preserves traceable audit sources for manual operations, dashboard IPC forwarding, and internal control calls.
