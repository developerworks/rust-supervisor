# 运维手册

语言: [English](../en/operations-runbook.html)

> **说明**: 每个步骤都列出了关键节点的期望 metrics(指标) 取值.
> 如果观测值与期望值不符, 请按升级路径处理或参考链接章节.

## P1-001: 监督器进程崩溃

**症状**: supervisor(监督器) 进程意外退出; child(子任务) 变为孤儿进程.

| 步骤 | 操作                                                                   | 期望指标                                                        | 预计耗时 |
| ---- | ---------------------------------------------------------------------- | --------------------------------------------------------------- | -------- |
| 1    | 检查进程状态: `pgrep -x supervisor`                                    | `exit code == 0`(运行中) 或 `exit code == 1`(未运行)            | 1min     |
| 2    | 如未运行, 检查最近日志: `journalctl -u supervisor -n 50`               | 日志以 `ShutdownPhase::Completed`(计划内) 或 `Panic`(意外) 结尾 | 2min     |
| 3    | 如意外崩溃: 收集 core dump 和 backtrace(回溯)                          | Core dump 文件存在于 `/tmp/`                                    | 2min     |
| 4    | 重启 supervisor: `cargo run --release --example supervisor_quickstart` | `health.status == "ready"` 在 30s 内                            | 5min     |
| 5    | 验证子任务重连: 检查 dashboard IPC(看板进程间通信)                     | `dashboard_link == "connected"`                                 | 2min     |

**升级路径**: 如果重启失败两次, 携带 core dump 和日志升级到 L2 工程团队.
**总预计耗时**: 12min(在 15min SLA(服务等级协议) 内).

## P1-002: 子任务崩溃循环

**症状**: child(子任务) 反复失败并重启; `current_state` 显示重启计数过高.

| 步骤 | 操作                                               | 期望指标                                                                | 预计耗时 |
| ---- | -------------------------------------------------- | ----------------------------------------------------------------------- | -------- |
| 1    | 查询当前状态: `handle.current_state()`             | `children.failed > 0` 且 `child_runtime_records[].restart_count > 阈值` | 1min     |
| 2    | 检查 journal(事件日志) 中的子任务退出原因          | `TaskExit::Panicked` 或 `TaskExit::Failed` 附带原因字符串               | 2min     |
| 3    | 如重启预算耗尽: `restart_budget.tokens == 0`       | 预算耗尽, 子任务自动 quarantine(隔离)                                   | 1min     |
| 4    | 移除或替换故障 child spec: `handle.remove_child()` | `CommandResult::Accepted`                                               | 2min     |
| 5    | 验证无残留 slot: 检查 `current_state()`            | `children.running == target_count`                                      | 2min     |

**升级路径**: 如果在 10min 内未确定根因, 携带退出原因和 journal 片段提单.
**总预计耗时**: 8min.

## P1-003: Dashboard IPC(看板进程间通信) 断开

**症状**: `health.dashboard_link == "disconnected"`; dashboard(看板) UI(用户界面) 无数据.

| 步骤 | 操作                                                         | 期望指标                                                             | 预计耗时 |
| ---- | ------------------------------------------------------------ | -------------------------------------------------------------------- | -------- |
| 1    | 检查 IPC socket(套接字) 路径: `ls -la /tmp/supervisor.sock`  | socket(套接字) 文件存在且权限正确                                    | 1min     |
| 2    | 检查 relay(中继) 进程: `pgrep -x relay`                      | 进程在运行                                                           | 1min     |
| 3    | 重启 relay: `kill -TERM <relay_pid>` 等待自动重启            | Supervisor 自动拉起 relay; `dashboard_link == "connected"` 在 10s 内 | 3min     |
| 4    | 如仍然断开, 重启 dashboard IPC: `handle.restart_dashboard()` | `health.dashboard_link == "connected"`                               | 2min     |

**升级路径**: 如果 IPC socket 路径冲突(错误包含 `field_path="ipc.path"`), 检查 deployment guide(部署指南) 的 socket path 配置.
**总预计耗时**: 7min.

## P1-004: 运行时饥饿

**症状**: 控制循环迭代停滞; `health.uptime_secs` 仍在增长但事件未处理.

| 步骤 | 操作                                                                                    | 期望指标                                            | 预计耗时 |
| ---- | --------------------------------------------------------------------------------------- | --------------------------------------------------- | -------- |
| 1    | 检查 Tokio runtime(异步运行时) metrics(指标): `handle.health().control_loop_iterations` | `iterations_per_sec > 0`                            | 1min     |
| 2    | 如停滞, 检查是否有 blocking(阻塞) 任务: 审查 child task(子任务) 列表                    | 无 child 处于 `BlockForever` 或 `IgnoreCancel` 状态 | 2min     |
| 3    | Quarantine(隔离) 可疑子任务: `handle.quarantine_child()`                                | Child 标记为 `Quarantined`                          | 2min     |
| 4    | 验证恢复: `health.control_loop_iterations` 增长                                         | 5s 后 `iterations_per_sec > 0`                      | 3min     |

**升级路径**: 如果 quarantine(隔离) 所有非关键子任务后饥饿仍未解除, 携带运行时 metrics(指标) 快照升级到 L2.
**总预计耗时**: 8min.
