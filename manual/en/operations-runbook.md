# Operations Runbook

Language: [中文](../zh/operations-runbook.html)

> **Note**: Each procedure lists expected metrics values at key steps.
> If the observed value differs, follow the escalation path or refer to the linked section.

## P1-001: Supervisor Process Crash

**Symptoms**: Supervisor process exits unexpectedly; children become orphaned.

| Step | Action                                                                    | Expected Metrics                                                           | Estimated Duration |
| ---- | ------------------------------------------------------------------------- | -------------------------------------------------------------------------- | ------------------ |
| 1    | Check process status: `pgrep -x supervisor`                               | `exit code == 0` (process running) or `exit code == 1` (not running)       | 1min               |
| 2    | If not running, check last log lines: `journalctl -u supervisor -n 50`    | Log ends with `ShutdownPhase::Completed` (planned) or `Panic` (unexpected) | 2min               |
| 3    | If unexpected crash: collect core dump and backtrace                      | Core dump file present in `/tmp/`                                          | 2min               |
| 4    | Restart supervisor: `cargo run --release --example supervisor_quickstart` | `health.status == "ready"` within 30s                                      | 5min               |
| 5    | Verify children reconnected: check dashboard IPC                          | `dashboard_link == "connected"`                                            | 2min               |

**Escalation**: If restart fails twice, escalate to L2 engineering with core dump and logs.
**Total estimated duration**: 12min (within 15min SLA).

## P1-002: Child Task Crash Loop

**Symptoms**: A child task repeatedly fails and restarts; `current_state` shows elevated restart counts.

| Step | Action                                                           | Expected Metrics                                                              | Estimated Duration |
| ---- | ---------------------------------------------------------------- | ----------------------------------------------------------------------------- | ------------------ |
| 1    | Query current state: `handle.current_state()`                    | `children.failed > 0` and `child_runtime_records[].restart_count > threshold` | 1min               |
| 2    | Check child exit reason in journal                               | `TaskExit::Panicked` or `TaskExit::Failed` with reason string                 | 2min               |
| 3    | If restart budget exhausted: `restart_budget.tokens == 0`        | Budget exhausted; child quarantined automatically                             | 1min               |
| 4    | Remove or replace the faulty child spec: `handle.remove_child()` | `CommandResult::Accepted`                                                     | 2min               |
| 5    | Verify no lingering slot: check `current_state()`                | `children.running == target_count`                                            | 2min               |

**Escalation**: If child root cause not identified in 10min, file a bug with the exit reason and journal snippet.
**Total estimated duration**: 8min.

## P1-003: Dashboard IPC Disconnected

**Symptoms**: `health.dashboard_link == "disconnected"`; dashboard UI shows no data.

| Step | Action                                                                     | Expected Metrics                                                           | Estimated Duration |
| ---- | -------------------------------------------------------------------------- | -------------------------------------------------------------------------- | ------------------ |
| 1    | Check IPC socket path existence: `ls -la /tmp/supervisor.sock`             | Socket file present with correct permissions                               | 1min               |
| 2    | Check relay process: `pgrep -x relay`                                      | Process running                                                            | 1min               |
| 3    | Restart relay: `kill -TERM <relay_pid>` and wait for auto-restart          | Supervisor auto-restarts relay; `dashboard_link == "connected"` within 10s | 3min               |
| 4    | If still disconnected, restart dashboard IPC: `handle.restart_dashboard()` | `health.dashboard_link == "connected"`                                     | 2min               |

**Escalation**: If IPC socket path contention (error contains `field_path="ipc.path"`), check deployment guide socket path configuration.
**Total estimated duration**: 7min.

## P1-004: Runtime Starvation

**Symptoms**: Control loop iterations stall; `health.uptime_secs` advances but events are not processed.

| Step | Action                                                                 | Expected Metrics                                   | Estimated Duration |
| ---- | ---------------------------------------------------------------------- | -------------------------------------------------- | ------------------ |
| 1    | Check Tokio runtime metrics: `handle.health().control_loop_iterations` | `iterations_per_sec > 0`                           | 1min               |
| 2    | If stalled, check for blocking tasks: review child task list           | No child in `BlockForever` or `IgnoreCancel` state | 2min               |
| 3    | Quarantine suspicious children: `handle.quarantine_child()`            | Child marked as `Quarantined`                      | 2min               |
| 4    | Verify recovery: `health.control_loop_iterations` increases            | `iterations_per_sec > 0` after 5s                  | 3min               |

**Escalation**: If starvation persists after quarantining all non-critical children, escalate to L2 with runtime metrics snapshot.
**Total estimated duration**: 8min.
