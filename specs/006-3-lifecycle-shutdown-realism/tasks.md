# Tasks(任务): 真实生命周期与无孤儿关停

**Input(输入)**: 设计文档来自 `specs/006-3-lifecycle-shutdown-realism/`
**Prerequisites(前置文档)**: plan.md(必需), spec.md(用户故事必需), data-model.md(实体定义), contracts/child-slot-api.md(ChildSlot API 契约), contracts/shutdown-phase-enum.md(关停阶段契约), research.md(并发安全研究), quickstart.md(阅读顺序)

**Tests(测试)**: 行为变化必须先有测试任务, 再有实现任务. 所有测试放在外部 `tests/` 目录.

**Organization(组织方式)**: 任务按用户故事分组: US1(关停信号真实传递), US2(单活动执行线), US3(join 可达性). 每个故事可独立实现和独立测试.

## Format(格式): `[ID] [P?] [Story] Description(描述)`

- **[P]**: 可以并行执行, 因为任务修改不同文件, 并且不依赖未完成任务.
- **[Story]**: 标记任务属于哪个用户故事, 例如 US1, US2, US3.
- 任务描述必须写出准确文件路径.
- 任务描述使用中文; 英文术语写成 `English(中文说明)`.
- 所有测试放在外部 `tests/` 目录, 不在 `src/` 模块文件中写测试.

---

## Phase 1(阶段一): Setup(共享基础设施)

**Purpose(目的)**: 初始化项目结构和类型模块.

- [x] T001 在 `src/runtime/` 下创建 `child_slot.rs`, `admission.rs`, `shutdown.rs` 三个空模块骨架, 并在 `src/runtime/mod.rs` 中声明 `pub mod child_slot; pub mod admission; pub mod shutdown;`.
- [x] T002 [P] 在 `src/types/` 下创建 `mod.rs`, `running_instance_id.rs` 两个文件: `mod.rs` 声明 `pub mod running_instance_id;`, `running_instance_id.rs` 定义 `RunningInstanceId` 类型(Newtype 包裹 `(Generation, ChildStartCount)`), 实现 `Display`, `Serialize`, `Deserialize`.
- [x] T003 [P] 运行 `cargo check` 和 `cargo fmt` 确认零编译错误.

---

## Phase 2(阶段二): Foundational(阻塞前置基础)

**Purpose(目的)**: 完成所有用户故事都需要的核心数据结构 `ChildSlot(子任务槽)` 和 `AdmissionSet(承认集合)`. 本阶段完成前, 任何用户故事实现都不能开始.

**Critical(关键要求)**: `ChildSlot` 结构体字段定义必须与 `data-model.md` 冻结的一致.

- [x] T004 在 `src/runtime/child_slot.rs` 中定义 `ChildSlot` 结构体, 字段按 `data-model.md` 实体定义: 包含 `child_id`, `path`, `status`, `operation`, `generation`, `attempt`, `restart_count`, `cancellation_token`, `abort_handle`, `completion_receiver`, `heartbeat_receiver`, `readiness_receiver`, `last_exit`, `last_ready_at`, `last_heartbeat_at`, `restart_window`, `pending_restart`, `attempt_cancel_delivered`, `abort_requested`. 所有字段带英文文档注释.
- [x] T005 [P] 在 `src/runtime/child_slot.rs` 中定义 `ChildExitSummary` 结构体, 字段按 `data-model.md`: `exit_code: Option<i32>`, `exit_reason: String`, `exited_at_unix_nanos: u128`. 实现 `Display` 和 `from_report()` 构造方法.
- [x] T006 [P] 在 `src/runtime/child_slot.rs` 中为 `ChildSlot` 实现构造和生命周期方法, 按 `contracts/child-slot-api.md` 契约: `new()` 创建空槽位, `activate()` 激活尝试, `deactivate()` 停用并记录退出摘要, `cancel() -> bool` 传递取消令牌, `abort() -> bool` 中止任务, `has_active_attempt() -> bool` 查询.
- [x] T007 [P] 在 `src/runtime/admission.rs` 中定义 `AdmissionSet` 结构体(内部 `HashSet<ChildId>`), 按 `data-model.md`: 实现 `try_admit()`, `try_admit_or_idempotent()`(幂等准入 per `research.md` 问题 2), `release()`, `is_admitted()`.
- [x] T008 [P] 在 `src/runtime/admission.rs` 中定义 `AdmissionConflict` 结构化错误, 按 `data-model.md` 字段: `child_id`, `active_generation`, `active_attempt`, `conflicting_request`. 实现 `Display` 与 `std::error::Error`.
- [x] T009 更新 `src/runtime/control_loop.rs` 中的 `RuntimeControlState` 结构体: 新增 `slots: HashMap<ChildId, ChildSlot>` 和 `admission_set: AdmissionSet` 字段. 保留 `child_runtime_states` 标记 `#[allow(dead_code)]` 以保持编译. 所有新增字段在 `new()` 中正确初始化.
- [x] T010 运行 `cargo check` 确认 Foundational(基础) 阶段所有新增类型编译无错. 执行 `cargo fmt`.

**Checkpoint(检查点)**: `ChildSlot`, `AdmissionSet`, `AdmissionConflict`, `ChildExitSummary` 类型已就绪, `RuntimeControlState.slots` 已可在后续阶段使用. 用户故事实现可以开始.

---

## Phase 3(阶段三): User Story 1(用户故事一) - 关停信号真实传给目标任务 (Priority(优先级): P1) MVP(最小可用产品)

**Goal(目标)**: 每次 shutdown(关停) 或 cancel(取消) 指令必须真实触发目标 `ChildSlot` 的 `CancellationToken`, 超时后执行 `abort()`, 并能在事件流中观察到阶段名与截止时刻.

**Independent Test(独立测试)**: 为每类生命周期指令各装一个低成本探针(进程存活位图, 显式 sleep(休眠) 任务收到 cancel(取消) 的时刻戳). 对照事件流与 status(状态视图) 行验证一致性.

### Tests for User Story 1(用户故事一的测试)

> **已实现, 全部通过**: 4 个测试覆盖 cancel 投递, 超时 abort, remove cleanup, pause/resume 传播.

- [x] T011 [P] [US1] 在 `tests/lifecycle_integration.rs` 中添加 `test_shutdown_tree_delivers_cancel_to_sleeping_child`: 创建 sleep(休眠) 子任务, 下发 `ShutdownTree`, 断言 `CancellationToken` 在 5 秒内被触发.
- [x] T012 [P] [US1] 在 `tests/lifecycle_integration.rs` 中添加 `test_shutdown_tree_aborts_after_graceful_timeout`: 创建忽略 cancel(取消) 信号的子任务, 设置 `graceful_timeout` 为 2 秒, 断言 `abort()` 被调用且无悬挂 `JoinHandle`.
- [x] T013 [P] [US1] 在 `tests/lifecycle_integration.rs` 中添加 `test_cancel_command_delivers_token_to_active_child`: 对运行中的子任务下发 cancel(取消) 指令, 断言 `CancellationToken.is_cancelled()` 为 `true`, 子任务退出.
- [x] T014 [P] [US1] 在 `tests/lifecycle_integration.rs` 中添加 `test_pause_resume_commands_propagate_to_child_slot`: 下发 `PauseChild` 后下发 `ResumeChild`, 断言 `ChildSlot.operation` 在 `Paused` 与 `Active` 间切换.

### Implementation for User Story 1(用户故事一的实现)

- [x] T015 [US1] 在 `src/runtime/shutdown.rs` 中实现 `shutdown_tree_fanout(slots: &mut HashMap<ChildId, ChildSlot>, policy: &ShutdownPolicy, admission: &mut AdmissionSet) -> Vec<ChildShutdownOutcome>` 函数: 按 `contracts/shutdown-phase-enum.md` 的 5 阶段执行: cancel → wait graceful → abort → wait abort → force-deactivate. 返回每个子任务的关停结果. 同时实现 `reconcile_shutdown_slots()` 对账函数和 `drain_one_slot()` 辅助函数.
- [x] T016 [US1] 在 `src/runtime/control_loop.rs` 中实现 `handle_shutdown_tree` 方法: 调用 `shutdown_tree_fanout`, 将结果写入 `shutdown_pipeline.cache_report()`, 更新 `shutdown` 状态机, 通过 `observability` 发射 `ShutdownPhase` 每个阶段事件. 按 `contracts/shutdown-phase-enum.md` 的事件发射契约实现.
- [x] T017 [US1] 在 `src/runtime/control_loop.rs` 中实现 `handle_command_on_slot` 辅助方法: 将原来的 `child_runtime_states` 查找替换为 `slots` 查找, 对 `ChildSlot` 实例调用 `cancel()`, `abort()`, 修改 `operation` 字段等. 覆盖 pause, resume, remove, quarantine 命令.
- [x] T018 [US1] 在 `src/runtime/control_loop.rs` 中实现 `process_child_exit_on_slot` 方法: 当子任务 `JoinHandle` 完成后, 从 `ChildSlot.completion_receiver` 取出报告, 调用 `deactivate()` 记录 `ChildExitSummary`, 通过 `admission_set.release()` 释放准入.
- [x] T019 [US1] 在 `src/runtime/control_loop.rs` 中实现 `observe_slot_liveness` 方法: 遍历 `slots` 中有活动尝试的 `ChildSlot`, 检查 `heartbeat_receiver`, 心跳陈旧(超过 `DEFAULT_HEARTBEAT_TIMEOUT_SECS`) 时通过 `observability` 发射 `ChildLivenessStale` 事件.
- [x] T020 [US1] 更新 `src/control/command.rs` 中的 `ManagedChildState` 枚举: 标记 `#[deprecated(note = "migrated to ChildSlot and ChildAttemptStatus in src/runtime/child_slot.rs")]`. 更新所有匹配 `ManagedChildState` 的代码路径为 `#[allow(deprecated)]` 以保持编译.
- [x] T021 [US1] 运行 `cargo test --test lifecycle_integration` 确认 US1 全部 4 个测试通过. 运行 `cargo test` 确认无回归.

**Checkpoint(检查点)**: 所有关停和取消指令都真实传递 `CancellationToken`, 超时后执行 `abort()`, `ChildSlot` 状态与外部可观察副作用一致. US1 可独立演示.

---

## Phase 4(阶段四): User Story 2(用户故事二) - 同一 child id 最多一条活动执行线 (Priority(优先级): P1)

**Goal(目标)**: 对于同一 `ChildId`, `ChildSlot` 的 `pending_restart` 与 active attempt(活动尝试) 始终互斥. 并发重启风暴不得产生多条仍在跑的执行线.

**Independent Test(独立测试)**: 仿真并发 restart(重启) 请求. 统计 `ChildSlot` 快照行数, 对照日志里冲突或幂等命中次数.

### Tests for User Story 2(用户故事二的测试)

> **已实现, 5 个测试(含幂等) 全部通过**.

- [x] T022 [P] [US2] 在 `tests/concurrent_restart_test.rs` 中添加 `test_concurrent_restart_only_one_active_attempt`: 对同一 `ChildId` 两次 `try_admit()`, 断言首次成功、二次返回 `AdmissionConflict`, `release()` 后可再准入.
- [x] T023 [P] [US2] 在 `tests/concurrent_restart_test.rs` 中添加 `test_concurrent_restart_and_remove_serialize`: 模拟两个操作竞争同一 `ChildId`, 断言谁先准入谁成功, 另一个被拒.
- [x] T024 [P] [US2] 在 `tests/concurrent_restart_test.rs` 中添加 `test_concurrent_restart_preserves_generation_monotonicity`: 连续 5 次 activate/deactivate 循环, 断言 `restart_count == 5`.
- [x] T025 [P] [US2] 在 `tests/concurrent_restart_test.rs` 中添加 `test_admission_conflict_error_contains_running_instance_id`: 断言 `AdmissionConflict` 携带当前活跃尝试的 `generation` 和 `attempt` 值, `Display` 格式化包含 `gen{value}-attempt{value}`.
- [x] T025a [P] [US2] 在 `tests/concurrent_restart_test.rs` 中添加 `test_try_admit_or_idempotent_accepts_same_generation_attempt`: 断言同一 gen/att 的幂等重试成功, 不同 gen/att 仍冲突.

### Implementation for User Story 2(用户故事二的实现)

- [x] T026 [US2] 在 `src/runtime/control_loop.rs` 的 `handle_restart_child` 方法中集成 `admission_set.try_admit_or_idempotent`: 在调用 `ChildSlot.activate()` 前检查准入; 拒绝时通过 `command_sender` 返回 `CommandResult::ChildControl` 携带 `ChildControlResult::Conflict`.
- [x] T027 [US2] 在 `src/runtime/control_loop.rs` 中实现 `ChildSlot` 的 `pending_restart` 互斥逻辑: 当 `slot.pending_restart == true` 时拒绝新的 `RestartChild` 命令, 等待 `process_child_exit_on_slot`(T018) 完成后清除标志. 新增 `check_slot_restart_eligibility` 辅助方法.
- [x] T028 [US2] 更新 `src/control/outcome.rs` 中的 `ChildControlResult` 枚举: 新增 `Conflict { conflict: AdmissionConflict }` 变体. 更新 `Display` 实现, 确保格式化包含 `gen{}-attempt{}`.
- [x] T029 [US2] 在 `src/event/payload.rs` 的 `What` 枚举中确保 `ChildRestartConflict` 变体存在(如已存在则验证字段完整): 包含 `child_id`, `active_generation`, `active_attempt`, `request_generation`, `request_attempt`. 在 `handle_restart_child` 冲突路径中发射此事件.
- [x] T030 [US2] 运行 `cargo test --test concurrent_restart_test` 确认 US2 全部 5 个测试通过. 运行 `cargo test` 确认 US1 测试无回归.

**Checkpoint(检查点)**: 同一 `ChildId` 在任何时刻至多 1 条活动执行线. 并发冲突产生可审计的 `AdmissionConflict` 结构化错误. US1 和 US2 均可独立测试.

---

## Phase 5(阶段五): User Story 3(用户故事三) - join 在所有生命周期路径上都可达 (Priority(优先级): P2)

**Goal(目标)**: 关停流程结束后, 所有 `JoinHandle` 集合要么清空, 要么只剩文档写明的延迟释放窗口. 不得在宿主机上留下 orphan(孤儿) 进程.

**Independent Test(独立测试)**: 比对关停前后的句柄计数与事件里声明的 `ShutdownPhase` 完成位. 断言在窗口边界外计数回到基线.

### Tests for User Story 3(用户故事三的测试)

> **NOTE(说明): 必须先写这些测试, 并确认它们在实现前失败.**

- [x] T031 [P] [US3] 在 `tests/shutdown_orphan_test.rs` 中添加 `test_shutdown_completion_no_orphan_join_handles`: 创建 5 个子任务(含 1 个慢任务忽略 cancel(取消) 信号), 执行 `shutdown_tree`, 断言关停完成后所有 `ChildSlot` 的 `join_handle`, `completion_receiver`, `cancellation_token` 均为 `None`, `reconcile_shutdown_slots().verified_clean` 为 `true`.
- [x] T032 [P] [US3] 在 `tests/shutdown_orphan_test.rs` 中添加 `test_shutdown_reconcile_report_lists_residual_slots`: 创建一个在全局超时后仍持有句柄的 `ChildSlot`(模拟 force-deactivate 未覆盖的情况), 断言 `reconcile_shutdown_slots()` 返回 `verified_clean == false` 且 `orphan_slots` 非空.
- [x] T033 [P] [US3] 在 `tests/join_timeout_test.rs` 中添加 `test_join_timeout_respected_even_with_never_ending_task`: 创建一个永不检查 cancel(取消) 的死循环子任务, 设置 `abort_wait` 为 500ms, 断言整体关停在 `graceful_timeout + abort_wait` 总和内完成.
- [x] T034 [P] [US3] 在 `tests/join_timeout_test.rs` 中添加 `test_remove_command_cleans_slot_completely`: 对运行中的子任务调用等效的 remove(移除) 操作, 断言操作返回后 `ChildSlot.has_active_attempt() == false`.
- [x] T035 [P] [US3] 在 `tests/join_timeout_test.rs` 中添加 `test_all_lifecycle_paths_join_to_terminal`: 测试正常退出, cancel(取消) 退出, timeout(超时) abort(中止) 三种生命周期路径, 断言每种路径下 `ChildSlot` 最终都无活跃尝试且 `last_exit` 被记录.

### Implementation for User Story 3(用户故事三的实现)

- [x] T036 [US3] 在 `src/runtime/shutdown.rs` 的 `shutdown_tree_fanout` 函数中验证全局超时控制已实现: 用 `tokio::time::timeout` 包裹整个扇出过程, 超时上限为 `graceful_timeout + abort_wait`. 对超时后剩余未完成的 `ChildSlot` 强制执行 `deactivate()` 并标记 `ShutdownPhase::AbortStragglers`.
- [x] T037 [US3] 在 `src/runtime/shutdown.rs` 中验证 `reconcile_shutdown_slots` 已扫描所有 `ChildSlot` 并返回 `SlotReconcileResult`, 包含 `orphan_slots`, `total_slots_checked`, `verified_clean` 字段.
- [x] T038 [US3] 在 `src/runtime/control_loop.rs` 中实现 `drain_all_join_handles` 方法: 在 `ShutdownPhase::Reconcile` 阶段对 `slots` 中每个仍持有句柄的 `ChildSlot` 执行 `deactivate()`, 确保所有句柄都被消费.
- [x] T039 [US3] 在 `src/runtime/control_loop.rs` 中更新 `handle_shutdown_tree` 方法(T016): 扇出完成后调用 `reconcile_shutdown_slots`, 若发现残余项则发射 `What::ShutdownReconcileWarning` 事件; 最后调用 `drain_all_join_handles`.
- [x] T040 [US3] 更新 `src/shutdown/report.rs` 中的 `ShutdownReconcileReport` 结构体: 新增 `orphan_slots: Vec<ChildId>`, `total_slots_checked: usize`, `verified_clean: bool` 字段, 保持向后兼容.
- [x] T041 [US3] 运行 `cargo test --test shutdown_orphan_test --test join_timeout_test` 确认 US3 全部 5 个测试通过. 运行 `cargo test` 确认 US1, US2 测试无回归.

**Checkpoint(检查点)**: 所有生命周期路径(正常退出, cancel(取消), timeout(超时) abort(中止), remove)最终都 join(等待收敛) 到终态. 无悬挂 `JoinHandle` 或 orphan(孤儿) 进程.

---

## Phase 6(最终阶段): Polish & Cross-Cutting Concerns(收尾和横向事项)

**Purpose(目的)**: 完成影响多个用户故事的改进和代码清理.

- [x] T042 **[已完成: 全量迁移策略]** `child_runtime_states: HashMap<ChildId, ChildRuntimeState>` 已全部替换为 `slots: HashMap<ChildId, ChildSlot>`。`ChildSlot` 结构体已补齐兼容字段(`stop_state`, `generation_fence`, `restart_limit`, `restart_limit_tracker`, `last_control_failure` 等)。`RuntimeTimeBase`, `RestartLimitTracker`, `DEFAULT_HEARTBEAT_TIMEOUT_SECS` 已迁移至 `src/runtime/child_slot.rs`。control_loop.rs 70+ 处引用全部迁移至 slots/ChildSlot API。
- [x] T043 [P] 移除 `src/runtime/child_runtime_state.rs` 文件。`ChildRuntimeState` 结构体已删除；`RuntimeTimeBase`, `RestartLimitTracker`, `DEFAULT_HEARTBEAT_TIMEOUT_SECS` 已迁移至 `src/runtime/child_slot.rs`。`src/runtime/mod.rs` 已移除 `pub mod child_runtime_state;`。
- [x] T044 [P] 在 `src/runtime/child_slot.rs` 中为 `ChildSlot` 实现 `Serialize` 和 `Deserialize`(通过 serde derive), 使状态视图 JSON(JavaScript 对象表示法) 可对账打印, 按 `data-model.md` 字段定义.
- [x] T045 [P] 为 `src/runtime/child_slot.rs`, `src/runtime/admission.rs`, `src/runtime/shutdown.rs` 补齐模块文档注释(符合 Rust 源码英文注释规范 `//!`).
- [x] T046 [P] 更新 `src/control/command.rs` 移除 `ManagedChildState` 枚举。`dashboard/model.rs` 中 `From<ManagedChildState>` 改为 `From<ChildControlOperation>`；`managed_child_state_from_operation()` 函数已删除；`tests/dashboard_protocol_shape_test.rs` 已适配。
- [x] T047 运行 `cargo test` 全量测试套件(含 `supervisor_*`, `dashboard_*`, `ipc_*`, `work_role_*`), 确保所有现有测试无回归.
- [x] T048 运行 `cargo clippy --all-targets` — 新代码无 clippy 警告；预存警告不在本切片范围。
- [x] T049 运行 `cargo fmt --all` 确保代码格式一致.
- [x] T050 运行 `cargo doc --no-deps --document-private-items` — 新文件（child_slot, admission, shutdown, running_instance_id）无警告；11 个预存警告不在本切片范围。

---

## Dependencies & Execution Order(依赖和执行顺序)

### Phase Dependencies(阶段依赖)

````text
Phase 1 (Setup) ✅
  └─ Phase 2 (Foundational) ✅
       ├─ Phase 3 (US1) ✅ (11/11)
       │    └─ Phase 5 (US3) ✅ (11/11)
       ├─ Phase 4 (US2) ✅ (10/10)
       └─ Phase 6 (Polish) ✅ (9/9)
````

### User Story Dependencies(用户故事依赖)

- **US1(关停信号真实传递, P1)**: ✅ 完成.
- **US2(单活动执行线, P1)**: ✅ 完成.
- **US3(join 可达性, P2)**: ✅ 完成.

### Remaining Tasks Summary(剩余任务摘要)

| 阶段                   | 总任务 | 已完成 | 剩余   | 剩余任务 ID |
| ---------------------- | ------ | ------ | ------ | ----------- |
| Phase 1 (Setup)        | 3      | 3      | 0      | —           |
| Phase 2 (Foundational) | 7      | 7      | 0      | —           |
| Phase 3 (US1)          | 11     | 11     | 0      | —           |
| Phase 4 (US2)          | 10     | 10     | 0      | —           |
| Phase 5 (US3)          | 11     | 11     | 0      | —           |
| Phase 6 (Polish)       | 9      | 9      | 0      | —           |
| **合计**               | **51** | **51** | **0**  | —           |

### Implementation Strategy(实现策略)

### MVP First(先做最小可用产品)

1. ✅ Phase 1(Setup) 和 Phase 2(Foundational) 已完成.
2. → 完成 Phase 3(US1) 剩余任务 T016-T021 — 关停信号真实传递.
3. 停止并验证 US1: `cargo test --test lifecycle_integration`.
4. 此时即可演示: shutdown_tree 真实取消子任务并超时中止.

### Incremental Delivery(增量交付)

1. ✅ Setup + Foundational → 类型基础就绪.
2. ✅ US1 测试 + shutdown_tree_fanout → 核心关停函数(部分完成).
3. → US1 剩余 → 关停信号真实传递 (MVP).
4. → US2 → 并发安全, 单活动执行线保证.
5. → US3 → 无孤儿进程, join 可达性保证.
6. → Polish → 清理, 文档, 回归测试.

### Parallel Team Strategy(并行团队策略)

1. ✅ 团队已一起完成 Setup 和 Foundational.
2. → 开发者 A: US1 剩余(控制循环迁移, T016-T021).
3. → 开发者 B: US2 剩余(准入集成与事件, T026-T030).
4. → 开发者 C: US3 测试编写(T031-T035, 可立即开始).
5. US1 和 US2 完成后, 所有开发者一起完成 US3 实现和 Polish.

---

## Notes(说明)

- [P] 表示任务修改不同文件, 并且没有依赖.
- [Story] 标签把任务映射到具体用户故事, 方便追踪.
- 每个用户故事都必须能独立完成和独立测试.
- 实现前必须确认测试失败.
- 每个任务或逻辑组完成后可以提交.
- 可以在任何检查点停止并验证该故事.
- `ChildSlot` 字段命名以 `data-model.md` 冻结为准.
- `ChildSlot` API 契约以 `contracts/child-slot-api.md` 为准.
- `ShutdownPhase` 阶段过渡以 `contracts/shutdown-phase-enum.md` 为准.
- 并发安全设计决策参考 `research.md`.
- 所有 Rust 源码注释(`//`, `///`, `//!`) 必须使用英文.
- 所有规格, 计划, 任务文档正文必须使用中文, 英文术语写为 `English(中文说明)`.
