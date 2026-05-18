# Quickstart(快速开始): 真实生命周期与无孤儿关停

**Feature(功能)**: 006-3-lifecycle-shutdown-realism | **Date(日期)**: 2026-05-18

## 阅读顺序锚点

本文件为 `src/` 下与本次实现相关的关键文件提供阅读顺序. 按代码依赖关系排列, 从最底层类型到最上层编排.

---

## 第 1 步: 核心类型定义

### `src/id/types.rs` (已存在)

- `ChildId` — 子任务标识符.
- `Generation` — 代次, 单调递增.
- `ChildStartCount` — 尝试编号, 单调递增.
- `SupervisorPath` — 监督树路径.

### `src/types/running_instance_id.rs` (新增)

- `RunningInstanceId` — `(Generation, ChildStartCount)` 的 Newtype, 唯一标识一次活动尝试.
- 格式化: `gen{value}-attempt{value}`.

### `src/control/outcome.rs` (已存在, 本次扩展)

- `ChildAttemptStatus` — 子任务尝试状态枚举: `Starting`, `Running`, `Ready`, `Cancelling`, `Stopped`.
- `ChildControlOperation` — 操作员控制操作: `Active`, `Paused`, `Quarantined`, `Removed`.
- `ChildControlResult` — 控制命令结果枚举(本次新增 `Conflict` 变体).

### `src/shutdown/stage.rs` (已存在)

- `ShutdownPolicy` — 关停时间策略(graceful_timeout + abort_wait).
- `ShutdownPhase` — 关停阶段枚举: `Idle → RequestStop → GracefulDrain → AbortStragglers → Reconcile → Completed`.

---

## 第 2 步: ChildSlot 数据结构

### `src/runtime/child_slot.rs` (新增)

核心新类型. 阅读顺序:

1. `ChildExitSummary` — 退出摘要结构体.
2. `ChildSlot` 结构体字段 — 理解每个字段的含义.
3. `ChildSlot::new()` — 创建空槽位.
4. `ChildSlot::activate()` — 激活一次尝试.
5. `ChildSlot::deactivate()` — 停用并记录退出.
6. `ChildSlot::cancel()` / `abort()` — 取消与中止操作.
7. `ChildSlot::has_active_attempt()` — 查询方法.

关键概念: 一个 `ChildSlot` 在任何时刻至多绑定一个 active attempt(活动尝试). 当尝试结束时, `deactivate()` 清除句柄并保留 `last_exit`.

---

## 第 3 步: 准入控制

### `src/runtime/admission.rs` (新增)

1. `AdmissionConflict` — 并发冲突结构化错误.
2. `AdmissionSet::try_admit()` — 准入检查.
3. `AdmissionSet::try_admit_or_idempotent()` — 幂等准入(US2).
4. `AdmissionSet::release()` / `is_admitted()` — 释放与查询.

关键概念: 在激活 `ChildSlot` 前必须先通过 `AdmissionSet` 准入, 在尝试结束后释放. 这保证了"同一 child id 至多一条活动执行线"不变式.

---

## 第 4 步: 关停扇出

### `src/runtime/shutdown.rs` (新增)

1. `shutdown_tree_fanout()` — 核心扇出函数. 按 5 个阶段执行: cancel → wait graceful → abort → wait abort → force-deactivate.
2. `reconcile_shutdown_slots()` — 关停后对账, 检测残余句柄.
3. `drain_one_slot()` — 内部辅助: 等待单个槽位完成.

关键概念: 全局超时 = `graceful_timeout + abort_wait`. force-deactivate 是最后保障路径.

---

## 第 5 步: 控制循环改造

### `src/runtime/control_loop.rs` (修改)

`RuntimeControlState` 结构体:

- 原 `child_runtime_states: HashMap<ChildId, ChildRuntimeState>` → 新增 `slots: HashMap<ChildId, ChildSlot>`.
- 新增 `admission_set: AdmissionSet`.

控制方法(随各 US 阶段逐步实现):

- `handle_shutdown_tree()` — 调用 `shutdown_tree_fanout`.
- `handle_restart_child()` — 准入 → 激活 → 释放.
- `handle_pause_child()` / `handle_resume_child()` — 修改 `operation` 字段.
- `handle_remove_child()` / `handle_quarantine_child()` — 关停 + 移除.

---

## 第 6 步: 事件扩展

### `src/event/payload.rs` (修改)

新增 `What` 变体:

- `ChildRestartConflict { child_id, active_generation, active_attempt, request_generation, request_attempt }` — 并发重启冲突审计事件.

---

## 第 7 步: 测试

### `tests/lifecycle_integration.rs` (新增)

- US1 验收: 关停信号传递, 超时中止, 取消命令, pause/resume.

### `tests/concurrent_restart_test.rs` (新增)

- US2 验收: 准入互斥, 并发冲突, 代次单调性, AdmissionConflict 完整性.

### `tests/shutdown_orphan_test.rs` (新增)

- US3 验收: 无悬挂句柄, 对账报告.

### `tests/join_timeout_test.rs` (新增)

- US3 验收: 各生命周期路径的 join 可达性.

---

## 快速验证命令

```bash
# 编译检查
cargo check

# US1 测试
cargo test --test lifecycle_integration

# US2 测试
cargo test --test concurrent_restart_test

# US3 测试
cargo test --test shutdown_orphan_test --test join_timeout_test

# 全量回归
cargo test

# 代码风格
cargo fmt --all
cargo clippy --all-targets -- -D warnings
```
