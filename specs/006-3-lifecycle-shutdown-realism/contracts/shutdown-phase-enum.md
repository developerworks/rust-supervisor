# Contract(契约): ShutdownPhase 枚举取值与迁移表

**Feature(功能)**: 006-3-lifecycle-shutdown-realism | **Date(日期)**: 2026-05-18
**Module(模块)**: `src/shutdown/stage.rs`

## 概述

`ShutdownPhase` 是关停扇出流程的外显阶段枚举, 用于事件流与状态视图中向运维人员解释当前关停进度. 每个阶段有明确的进入条件, 退出条件和下一个阶段.

---

## 枚举取值

| 变体              | 值(serde)            | 进入条件                                       | 退出条件                                   | 下一个阶段        |
| ----------------- | -------------------- | ---------------------------------------------- | ------------------------------------------ | ----------------- |
| `Idle`            | `"idle"`             | 初始状态; 或上一次关停已 `Reconcile` 完成      | 收到 `ShutdownTree` 命令                   | `RequestStop`     |
| `RequestStop`     | `"request_stop"`     | `cancel()` 已对所有活跃槽位调用                | 取消信号已传播(即 `cancel()` 返回)         | `GracefulDrain`   |
| `GracefulDrain`   | `"graceful_drain"`   | 取消信号已投递, 等待协作退出                   | `graceful_timeout` 耗尽 或 所有槽位已退出  | `AbortStragglers` |
| `AbortStragglers` | `"abort_stragglers"` | 存在未完成槽位且 `abort_after_timeout == true` | `abort_wait` 耗尽 或 所有中止槽位已退出    | `Reconcile`       |
| `Reconcile`       | `"reconcile"`        | 所有槽位已退出或已被强制清理                   | 对账完成, `ShutdownReconcileReport` 已生成 | `Completed`       |
| `Completed`       | `"completed"`        | 对账完成                                       | 无(终态)                                   | 无                |

---

## Serde(序列化) 迁移表

所有变体使用 `#[serde(rename_all = "snake_case")]`, 默认序列化名如上述.

**迁移注意**:

- 历史日志中可能出现旧命名, 新代码只识别上述 snake_case 值.
- 反序列化时对未知变体应返回 `DeserializeError` (通过 `#[serde(deny_unknown_fields)]` 或自定义 `Deserialize` 实现).

---

## 事件发射契约

控制循环在进入每个阶段时必须通过 `ObservabilityPipeline` 发射一个 `SupervisorEvent`, 其 `What` 负载包含:

| 阶段              | 事件负载                                    | 携带信息                                                                       |
| ----------------- | ------------------------------------------- | ------------------------------------------------------------------------------ |
| `RequestStop`     | `What::ShutdownPhaseEntered`                | `phase: ShutdownPhase`, `child_count: usize`                                   |
| `GracefulDrain`   | `What::ShutdownPhaseEntered`                | `phase: ShutdownPhase`, `remaining_active: usize`, `deadline_unix_nanos: u128` |
| `AbortStragglers` | `What::ShutdownPhaseEntered`                | `phase: ShutdownPhase`, `aborted_count: usize`                                 |
| `Reconcile`       | `What::ShutdownReconcileWarning` (若有残余) | `orphan_slots: Vec<ChildId>`                                                   |
| `Completed`       | `What::ShutdownPhaseEntered`                | `phase: ShutdownPhase`, `total_restart_count: u64`                             |

---

## 与 ChildSlot 的交互

| 阶段              | 对 ChildSlot 的操作                               |
| ----------------- | ------------------------------------------------- |
| `RequestStop`     | (`cancel()` 调用)                                 |
| `GracefulDrain`   | 等待 `completion_receiver`                        |
| `AbortStragglers` | `abort()` 调用                                    |
| `Reconcile`       | `deactivate()` 残余, `reconcile_shutdown_slots()` |
| `Completed`       | 无操作, 状态可查询                                |

---

## 使用约定

1. 禁止在 `Idle` 阶段对 `ChildSlot` 调用 `cancel()` 或 `abort()`.
2. `GracefulDrain` 的超时时间由 `ShutdownPolicy.graceful_timeout` 控制.
3. `AbortStragglers` 的超时时间由 `ShutdownPolicy.abort_wait` 控制.
4. 全局关停总时限 = `graceful_timeout + abort_wait`.
5. `Reconcile` 阶段发现的残余槽位必须记录在 `ShutdownReconcileReport` 中.
