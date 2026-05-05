# Runtime Control

## Control Entry Point

`SupervisorHandle` is the runtime control entry point. It sends requests to the runtime control loop through a command channel and returns `CommandResult`.

## Control Commands

- `add_child`: accept a dynamic child manifest when `DynamicSupervisorPolicy` allows another child.
- `remove_child`: stop the target child before removing its registry record.
- `restart_child`: request a restart for the target child.
- `pause_child`: pause governance for the target child.
- `resume_child`: resume governance for the target child.
- `quarantine_child`: place the target child into quarantine.
- `shutdown_tree`: shut down the whole supervisor tree.
- `current_state`: return the current `SupervisorState`.
- `subscribe_events`: subscribe to lifecycle events.

## Idempotent Behavior

Repeated control commands should not create unrecoverable errors. Pausing an already paused child returns the current state. Quarantining an already quarantined child returns the current state. Shutting down an already completed tree returns the existing shutdown result.

## Dynamic Additions

Dynamic additions are governed before the manifest is accepted. The runtime rejects `add_child` when dynamic supervision is disabled or when the declared plus dynamic child count has reached the configured limit. `current_state` includes accepted dynamic manifests in `child_count`.

## Audit Data

Each control command carries `requested_by`, `reason`, `target_path`, `accepted_at`, and `command_id`. These fields support audit events and incident review.
