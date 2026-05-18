# Contract(契约): ChildSlot 公开方法 API(接口)

**Feature(功能)**: 006-3-lifecycle-shutdown-realism | **Date(日期)**: 2026-05-18
**Module(模块)**: `src/runtime/child_slot.rs`

## 概述

`ChildSlot` 是监督器运行时持有的每个子任务的状态槽位. 控制循环通过本文定义的公开方法操作 `ChildSlot`, 禁止直接修改字段.

---

## 构造方法

### `new(child_id, path, restart_window) -> ChildSlot`

**说明**: 创建一个空槽位, 无活动尝试.

**参数**:

- `child_id: ChildId` — 稳定子任务标识.
- `path: SupervisorPath` — 监督树路径.
- `restart_window: Duration` — 重启记账窗口时长.

**返回**: 一个新 `ChildSlot`, 状态为 `Stopped`, 所有句柄字段为 `None`.

**前置条件**: 无.

**后置条件**: `generation.is_none()`, `attempt.is_none()`, `cancellation_token.is_none()`, `restart_count == 0`, `pending_restart == false`.

---

## 生命周期方法

### `activate(generation, attempt, status, handle)`

**说明**: 激活一次尝试, 绑定运行时句柄到此槽位.

**参数**:

- `generation: Generation` — 本次激活的代次.
- `attempt: ChildStartCount` — 单调尝试编号.
- `status: ChildAttemptStatus` — 初始状态(通常为 `Starting` 或 `Running`).
- `handle: ChildRunHandle` — 包含取消令牌, 中止句柄, 完成/心跳/就绪接收器.

**返回**: 无.

**前置条件**: 槽位必须无活动尝试 (`generation.is_none()`), `pending_restart` 必须为 `false`.

**后置条件**:

- `generation == Some(generation)`
- `attempt == Some(attempt)`
- `status == status`
- `cancellation_token == Some(handle.cancellation_token)`
- `abort_handle == Some(handle.abort_handle)`
- 所有 receiver 被设置
- `last_exit == None`
- `pending_restart == false`

### `deactivate(exit_summary)`

**说明**: 停用当前尝试, 记录退出摘要, 清除所有句柄, 递增 `restart_count`.

**参数**:

- `exit_summary: ChildExitSummary` — 本次退出的摘要.

**返回**: 无.

**前置条件**: 调用方必须已消费 `completion_receiver`(即已 await 完成或确认不再需要). 此方法不清空 receiver 中的未读数据.

**后置条件**:

- `last_exit == Some(exit_summary)`
- `restart_count` 递增 1
- 所有句柄字段为 `None`
- `status == Stopped`
- `attempt_cancel_delivered == false`, `abort_requested == false`

---

## 取消与中止方法

### `cancel() -> bool`

**说明**: 向活跃尝试投递取消信号.

**参数**: 无.

**返回**:

- `true` — 本次调用首次投递取消信号.
- `false` — 无活跃尝试, 或取消信号已投递.

**前置条件**: 无(调用安全, 即使无活跃尝试也返回 `false`).

**后置条件**: 若返回 `true`, 则 `attempt_cancel_delivered == true`, `status == Cancelling`.

**并发安全**: `CancellationToken::cancel()` 是原子操作, 可跨线程安全调用.

### `abort() -> bool`

**说明**: 请求中止活跃尝试.

**参数**: 无.

**返回**:

- `true` — 本次调用首次请求中止.
- `false` — 无活跃尝试, 或中止已请求.

**前置条件**: 无.

**后置条件**: 若返回 `true`, 则 `abort_requested == true`.

**注意**: `AbortHandle::abort()` 是尽力而为的, 不保证立即终止.

---

## 查询方法

### `has_active_attempt() -> bool`

**说明**: 检查槽位是否持有活跃尝试.

**返回**: `true` 当 `attempt.is_some() && cancellation_token.is_some()`.

---

## Pending Restart(待重启) 互斥契约

`pending_restart` 标志标志着一次重启已被批准但尚未激活(通常因为正在等待前次尝试完全退出). 互斥规则:

1. `pending_restart == true` → 拒绝新的 `activate()` 调用.
2. 控制循环在 `pending_restart == true` 时不接受新的 `RestartChild` 命令.
3. 当等待的前次尝试 `JoinHandle` 完成 → 设置 `pending_restart = false` → 允许下一次激活.

---

## 错误处理契约

- `ChildSlot` 方法不返回 `Result`, 因为它们操作的都是内部状态; 调用方的责任是确保前置条件满足.
- 若调用方在无活跃尝试时调用 `deactivate()`, 效果是幂等的(仅更新 `last_exit`).
- 前置条件违反(如在已有活跃尝试时调用 `activate()`) 是调用方的 bug, 应在开发阶段通过 `AdmissionSet` 防止, 不在 `ChildSlot` 内部检查.
