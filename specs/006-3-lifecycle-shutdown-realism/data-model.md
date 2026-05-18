# Data Model(数据模型): ChildSlot 与关停阶段

**Feature(功能)**: 006-3-lifecycle-shutdown-realism | **Date(日期)**: 2026-05-18
**Status(状态)**: Frozen(已冻结)

## 概述

本文定义本切片引入的新数据结构及其字段, 生命周期状态枚举, 以及实体间关系. 字段命名以本文为准, 代码实现必须一致.

---

## 实体

### ChildSlot(子任务槽)

取代当前 `RuntimeControlState` 中的 `ManagedChildState`. 每个 `ChildSlot` 绑定到一个 `ChildId`, 拥有该 child 的活动尝试的全部运行时句柄.

| 字段                       | 类型                                       | 必填 | 说明                                          |
| -------------------------- | ------------------------------------------ | ---- | --------------------------------------------- |
| `child_id`                 | `ChildId`                                  | 是   | 稳定子任务标识                                |
| `path`                     | `SupervisorPath`                           | 是   | 监督树路径                                    |
| `status`                   | `ChildAttemptStatus`                       | 是   | 当前活动尝试状态                              |
| `operation`                | `ChildControlOperation`                    | 是   | 操作员请求的操作状态                          |
| `generation`               | `Option<Generation>`                       | 否   | 活跃尝试的代次, 无活动尝试时为 `None`         |
| `attempt`                  | `Option<ChildStartCount>`                  | 否   | 活跃尝试的单调尝试编号, 无活动尝试时为 `None` |
| `restart_count`            | `u64`                                      | 是   | 累计重启计数, 跨所有代次单调递增              |
| `cancellation_token`       | `Option<CancellationToken>`                | 否   | 活跃尝试的取消令牌                            |
| `abort_handle`             | `Option<AbortHandle>`                      | 否   | 活跃尝试的中止句柄                            |
| `completion_receiver`      | `Option<watch::Receiver<...>>`             | 否   | 活跃尝试的完成通知接收器                      |
| `heartbeat_receiver`       | `Option<watch::Receiver<Option<Instant>>>` | 否   | 活跃尝试的心跳时间戳接收器                    |
| `readiness_receiver`       | `Option<watch::Receiver<ReadinessState>>`  | 否   | 活跃尝试的就绪状态接收器                      |
| `last_exit`                | `Option<ChildExitSummary>`                 | 否   | 最近一次退出摘要, 无历史退出时为 `None`       |
| `last_ready_at`            | `Option<u128>`                             | 否   | 最近一次就绪时刻(Unix 纳秒时间戳)             |
| `last_heartbeat_at`        | `Option<u128>`                             | 否   | 最近一次心跳时刻(Unix 纳秒时间戳)             |
| `restart_window`           | `Duration`                                 | 是   | 重启记账窗口时长                              |
| `pending_restart`          | `bool`                                     | 是   | 是否有待激活的重启请求                        |
| `attempt_cancel_delivered` | `bool`                                     | 是   | 是否已向活跃尝试投递取消信号                  |
| `abort_requested`          | `bool`                                     | 是   | 是否已请求中止活跃尝试                        |

**不变式**:

1. 任意时刻, `generation.is_some() == attempt.is_some() == cancellation_token.is_some()`.
2. `restart_count` 在每次 `deactivate()` 后单调递增.
3. `pending_restart == true` 时, 禁止新 `activate()` 调用.

### ChildExitSummary(子任务退出摘要)

记录一次子任务尝试退出时的摘要信息.

| 字段                   | 类型          | 必填 | 说明                                      |
| ---------------------- | ------------- | ---- | ----------------------------------------- |
| `exit_code`            | `Option<i32>` | 否   | 进程退出码, 无退出码时(如被中止)为 `None` |
| `exit_reason`          | `String`      | 是   | 人类可读退出原因                          |
| `exited_at_unix_nanos` | `u128`        | 是   | 退出时刻(Unix 纳秒时间戳)                 |

**构造来源**: 从 `ChildRunReport` 的 `TaskExit` 枚举转换.

### AdmissionSet(承认集合)

维护当前已被调度器准许进入真实执行阶段的 activity attempt(活动尝试) 主键集合.

| 内部字段   | 类型               | 说明                       |
| ---------- | ------------------ | -------------------------- |
| `admitted` | `HashSet<ChildId>` | 当前已准入的 child id 集合 |

**公开方法**:

- `try_admit(child_id, generation, attempt) -> Result<(), AdmissionConflict>`: 尝试准入. 若 child 已在集合中则返回冲突错误.
- `try_admit_or_idempotent(child_id, req_gen, req_att, active_gen, active_att) -> Result<(), AdmissionConflict>`: 尝试准入, 当请求与当前活跃实例相同时视为幂等成功.
- `release(child_id)`: 从集合中移除.
- `is_admitted(child_id) -> bool`: 检查是否已准入.

### AdmissionConflict(准入冲突)

结构化错误, 携带当前活跃尝试的标识信息.

| 字段                  | 类型              | 说明                      |
| --------------------- | ----------------- | ------------------------- |
| `child_id`            | `ChildId`         | 已有活跃尝试的 child 标识 |
| `active_generation`   | `Generation`      | 当前活跃尝试的代次        |
| `active_attempt`      | `ChildStartCount` | 当前活跃尝试的尝试编号    |
| `conflicting_request` | `String`          | 被拒绝的请求描述          |

### RunningInstanceId(运行实例标识)

唯一标识 `ChildSlot` 中的一次活动尝试, 由代次与尝试编号配对组成.

| 字段         | 类型              | 说明                 |
| ------------ | ----------------- | -------------------- |
| `generation` | `Generation`      | 激活时的代次         |
| `attempt`    | `ChildStartCount` | 激活时的单调尝试编号 |

**格式化**: `gen{generation}-attempt{attempt}` (例如 `gen1-attempt3`).

---

## 枚举

### ChildAttemptStatus(子任务尝试状态)

子任务活动尝试的生命周期阶段.

| 变体         | 说明             |
| ------------ | ---------------- |
| `Starting`   | 尝试正在启动     |
| `Running`    | 尝试正在运行     |
| `Ready`      | 尝试已报告就绪   |
| `Cancelling` | 尝试正在取消     |
| `Stopped`    | 尝试已停止(终态) |

**状态转移**:

```
Starting → Running → Ready
Running → Cancelling → Stopped
Running → Stopped
Ready → Cancelling → Stopped
```

### ChildControlOperation(子任务控制操作)

操作员请求的子任务控制操作.

| 变体          | 说明                             |
| ------------- | -------------------------------- |
| `Active`      | 正常运行(接受自动重启)           |
| `Paused`      | 已暂停(不接受自动重启)           |
| `Quarantined` | 已隔离(不接受自动重启, 保留记录) |
| `Removed`     | 已移除(等待清理或已清理)         |

### ShutdownPhase(关停阶段枚举)

关停扇出流程的外显阶段标签.

| 变体              | 说明                       | 下一个阶段        |
| ----------------- | -------------------------- | ----------------- |
| `Idle`            | 未关停                     | `RequestStop`     |
| `RequestStop`     | 已请求停止, 取消信号传播中 | `GracefulDrain`   |
| `GracefulDrain`   | 等待协作完成               | `AbortStragglers` |
| `AbortStragglers` | 强制中止未完成单元         | `Reconcile`       |
| `Reconcile`       | 对账终态                   | `Completed`       |
| `Completed`       | 关停已结束(终态)           | 无                |

---

## 实体关系

```
RuntimeControlState
  ├── slots: HashMap<ChildId, ChildSlot>   (1:N, 每个 child 一个 slot)
  │     └── ChildSlot
  │           ├── cancellation_token: Option<CancellationToken>
  │           ├── abort_handle: Option<AbortHandle>
  │           ├── completion_receiver: Option<watch::Receiver>
  │           ├── last_exit: Option<ChildExitSummary>
  │           └── (generation, attempt) ≅ RunningInstanceId
  └── admission_set: AdmissionSet          (1:1)
        └── admitted: HashSet<ChildId>     (与 slots 键集等势)

shutdown_tree_fanout
  ├── 输入: slots, ShutdownPolicy, AdmissionSet
  ├── 阶段: RequestStop → GracefulDrain → AbortStragglers → Reconcile
  └── 输出: Vec<ChildShutdownOutcome>
```
