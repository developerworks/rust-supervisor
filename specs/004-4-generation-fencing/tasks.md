---
description: "Tasks(任务列表): 代次隔离重启 generation fencing(代次隔离)"
---

# Tasks(任务): 代次隔离重启

**Input(输入)**: 设计文档来自仓库根相对路径 `specs/004-4-generation-fencing/`
**Prerequisites(前置文档)**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/generation-fencing.md`, `quickstart.md`

**Tests(测试)**: 本功能改变 `RestartChild`(重启子任务), 自动重启与退出报告语义, 触发宪章原则 III 中条文 '行为变化必须先有测试'. 每个用户故事节内必须先列测试任务, 再列实现任务.

**Organization(组织方式)**: 任务按用户故事分组, MVP(最小可用产品)为 Phase 3(阶段三) US1(用户故事一).

## Format(格式): `[ID] [P?] [Story] Description(描述)`

- **[P]**: 仅当任务修改不同文件且逻辑上不依赖未完成前置任务时可标. **同一文件多条连续修改一律不标 [P]**.
- **[Story]**: Setup, Foundational, Polish 阶段不带 story 标签. 本功能自 Phase 3(阶段三) 起每个用户故事任务必须带 `[US1]`, `[US2]` 或 `[US3]` 标签.
- 任务描述使用完整中文句子, 英文标识符写成 `identifier`(中文说明), 例如 `RestartChild`(重启子任务), 每条描述至少包含一处准确仓库路径.

## Path Conventions(路径约定)

本仓库为 Rust single crate(Rust 单包), 根路径含 `src/`, `tests/` 与 `Cargo.toml`. 新增外部集成测试必须放在 `src/tests/` 并在 `Cargo.toml` 中注册 `[[test]]`. 根目录 `tests/dashboard_protocol_shape_test.rs` 由 Cargo(构建工具) 自动发现, 本功能只允许原地扩展断言, 不得在仓库内复制同名测试目标路径.

## Event Timing(事件时序) 与 `contracts/generation-fencing.md`

以下分工用于消除 '事件全部堆到用户故事三' 的理解歧义. `ChildRestartFenceEntered`(子任务重启隔离已进入) 在 T014 接线, `ChildRestartFenceAbortRequested`(子任务重启隔离已请求中止) 在 T017, `ChildRestartFenceReleased`(子任务重启隔离已释放) 在 T018, `ChildRestartConflict`(子任务重启冲突) 在 Phase 4(阶段四) T023, `ChildAttemptStaleReport`(子任务过期报告) 在 Phase 5(阶段五) T026. Phase 6(阶段六) T028 仅做事件形状回归. 用户故事三的实现任务不得以「`fence`(隔离) `lifecycle`(生命周期) 其余事件」为理由推迟已在 US1 列出的三类 `fence`(隔离)事件. **Metrics Contract(指标契约) 默认口径**: 实现时在 `ObservabilityPipeline`(可观测流水线) 收到本条所列类型化事件或等价分支时应就地递增契约对应的 `counter`(计数器) 或更新 `gauge`(仪表), 避免围栏事件已发出而仪表盘长期为零的反直觉窗口; Phase 5(阶段五) **T027** 负责对照 `contracts/generation-fencing.md` 做 Metrics Contract 与 Audit Contract(审计契约) 全表收口与缺口补齐; Phase 6(阶段六) **T028** 配合事件形状与 `observability`(可观测性) 烟雾测试做交叉核验.

---

## Phase 1(阶段一): Setup(共享基础设施)

**Purpose(目的)**: 注册集成测试骨架并保持仓库基线可构建可测试.

- [x] T001 在根目录 `Cargo.toml` 增加 `[[test]]`, `name = "supervisor_generation_fencing_test"`, `path = "src/tests/supervisor_generation_fencing_test.rs"`, 并新建 `src/tests/supervisor_generation_fencing_test.rs`, 仅含可编译的烟雾 `#[test]`, 此时不得编写代次隔离契约行为断言.
- [x] T002 在 T001 完成后于仓库根运行 `cargo fmt --check` 与 `cargo test`, 用作 post-setup verification(占位后验证), 失败则先修正再进入下一阶段.

---

## Phase 2(阶段二): Foundational(阻塞前置基础)

**Purpose(目的)**: 贯通类型与序列化字段, `RestartChild` 行为可先保持既有实现或仅填充 `generation_fence = None`(无代次隔离结果).

### Foundational Implementation(基础阶段实现)

> **NOTE(说明)**: 必须先完成本节, 否则与 `generation_fence` 字段相关的 JSON(JavaScript对象表示序列)断言在 Foundational Tests(基础阶段测试)中无法有意义地编译.

- [x] T003 在 `src/control/outcome.rs` 依照 `specs/004-4-generation-fencing/data-model.md` 定义 `GenerationFencePhase`, `GenerationFenceDecision`, `GenerationFenceOutcome`, `StaleReportHandling`, `StaleAttemptReport`, `PendingRestart`, 含英文注释与文档并与 `serde`(序列化)派生配合, 禁止新增宪章定义的 **兼容导出(compatibility exports)** (例如仅为兼容外部调用而加的 `pub use` 重导出, 别名模块路径或薄封装).
- [x] T004 在 `src/runtime/child_runtime_state.rs` 嵌入 `GenerationFenceState`, 默认占位安全, 完整 `RestartChild` 语义推迟到 Phase 3(阶段三).
- [x] T005 扩展 `ChildControlResult` 增加 `generation_fence: Option<GenerationFenceOutcome>`, 并批量修正 `src/dashboard/model.rs`, `src/dashboard/protocol.rs`, `src/runtime/control_loop.rs`, `src/control/handle.rs` 与 `src/tests/` 中所有字面量构造, 直至 `cargo test --no-run` 编译通过, 非 `RestartChild` 路径统一填 `None`.
- [x] T006 在 `src/dashboard/model.rs` 与 `src/dashboard/ipc_server.rs` 增加 `dashboard`(仪表盘) 契约所需的 `generation_fence` 与 `pending restart`(待重启)摘要字段映射.
- [x] T007 在 `tests/dashboard_protocol_shape_test.rs` 为新返回字段增加形状断言, 并继续断言控制命令请求体未漂移.

### Foundational Tests(基础阶段测试)

- [x] T008 [P] 在 `src/tests/supervisor_generation_fencing_test.rs` 增加 `generation_fence_optional_field_present_in_dashboard_child_control_projection_test`, 允许 `generation_fence` 为空的 JSON(JavaScript对象表示序列)占位.
- [x] T009 [P] 在 `src/tests/naming_contract_test.rs` 批准名称集合增补 `GenerationFencePhase`, `GenerationFenceDecision`, `GenerationFenceOutcome`, `GenerationFenceState`, `PendingRestart`, `StaleAttemptReport`, `StaleReportHandling`, `ChildRestartFenceEntered`, `ChildRestartFenceAbortRequested`, `ChildRestartFenceReleased`, `ChildRestartConflict`, `ChildAttemptStaleReport`.

**Checkpoint(检查点)**: `cargo test` 全绿, `RestartChild` 可先保持旧语义.

---

## Phase 3(阶段三): User Story 1(用户故事一) - 重启前停止旧尝试 (Priority: P1) MVP

**Goal(目标)**: 必须先向当前活动 `attempt`(尝试)送达 `cancel`(取消), 在旧尝试完成报告到达前不得启动目标 `generation`(代次)的新实例, US1 Phase(阶段)内必须把三类 `fence`(隔离)事件接入 `ObservabilityPipeline`(可观测流水线).

### Tests for User Story 1

- [x] T010 [US1] 在 `src/tests/supervisor_generation_fencing_test.rs` 实现 `restart_child_sends_cancel_before_second_spawn_test`, 验证取消已送达且同一时间只有一个活动 `attempt`(尝试).
- [x] T011 [US1] 在同文件实现 `restart_child_queues_after_stop_decision_test`, 断言 `GenerationFenceDecision::QueuedAfterStop` 且 `ChildControlResult` 中 `generation` 与 `attempt` 仍指向旧活动身份.
- [x] T012 [US1] 在 `src/tests/supervisor_generation_fencing_test.rs` 或与关闭语义相容的 `src/tests/supervisor_shutdown_test.rs` 实现 `restart_child_blocked_during_tree_shutdown_test`, 当监督树处于禁止新活动的关闭窗口时调用 `RestartChild`, 断言得到 `BlockedByShutdown` 或契约等价的 `ChildControlFailure`, 且无新的 `spawn_once` 调度, 与 `contracts/generation-fencing.md` 中 Public API(公开接口)一致.
- [x] T013 [US1] 在同文件或对 `spawn_child_start` 错误路径的最小共用测试中实现 `pending_restart_target_spawn_failure_retains_prior_outcomes_test`, 覆盖 `spec.md` 边界情形 '新尝试启动失败时, 运行状态记录必须保留旧尝试的最终结果与新尝试失败原因', 要求在 `ChildRuntimeState` 或 `ChildControlResult` 可追溯旧停止结论与新 `spawn`(派生)失败原因.

### Implementation for User Story 1

- [x] T014 [US1] 在 `src/event/payload.rs` 增补 `ChildRestartFenceEntered` 变体必填字段, 并在 `src/runtime/control_loop.rs` 的 `RestartChild` 路径中, 于写入 `PendingRestart` 并对旧尝试送达 `cancel`(取消) 之后, 经 `src/observe/pipeline.rs` 发出类型化事件, 禁止仅以 `broadcast`(广播) 字符串顶替. 同任务要求在监督树已进入禁止新活动的关闭语义时 `RestartChild` 返回结构化 `BlockedByShutdown` 或等价 `ChildControlFailure`, 符合 `contracts/generation-fencing.md`.
- [x] T015 [US1] 在 `src/runtime/control_loop.rs` 收敛 `RestartChild`(重启子任务) 主干: 创建 `PendingRestart`, `phase = WaitingForOldStop`, 计算截止时间, 返回携带 `GenerationFenceOutcome` 的 `ChildControlResult`, 禁止再调用旧路径直接 `spawn_child_start` 起新实例. **实现提示**: 本条可与 T014 在同一提交中合并审阅, 以便 `control_loop.rs` 上 `RestartChild` 分支不出现长期双头实现.
- [x] T016 [US1] 在 `src/runtime/control_loop.rs` 的 `spawn_child_start` 与 `prepare_child_start` 增加单实例与 `pending restart`(待重启)互斥门禁, 删除或收敛不当的 `existing.abort()` 覆盖窗口. `ChildRunner::spawn_once` 失败时必须写回结构化失败且不丢失待重启语义所需的旧退出事实, 满足 T013 测试口径.
- [x] T017 [US1] 在 `src/runtime/control_loop.rs` 的 `reconcile_stop_deadlines` 或等价调度点实现 Abort Escalation(强制中止升级), 超时后触发 `abort` 与 `AbortingOld` 阶段, 增补或接线 `ChildRestartFenceAbortRequested` 并经 `observe pipeline`(可观测流水线)发送, 禁止与目标代次启动落在同一回合.
- [x] T018 [US1] 在 `src/runtime/control_loop.rs` 完成报告 `handler`(处理器)路径中处理匹配 `PendingRestart` 的旧 `(generation, attempt)` 三元组: 清理旧句柄后进入可启动窗口, `spawn_once` 成功则发出 `ChildRestartFenceReleased`, 若 `spawn_once` 失败则满足 T013 对归因的要求.

**Checkpoint(检查点)**: T010–T013 测试通过且 `cargo test --test supervisor_generation_fencing_test` 通过.

---

## Phase 4(阶段四): User Story 2(用户故事二) - 每个子任务只有一个活动尝试 (P2)

**Goal(目标)**: 手动 `RestartChild` 与策略自动重启共用启动门禁, 重复请求合并为 `AlreadyPending` 且不创建第二活动实例.

### Tests for User Story 2

- [x] T019 [US2] 在 `src/tests/supervisor_generation_fencing_test.rs` 实现 `duplicate_restart_child_merges_to_already_pending_test`, 断言第二次调用行为与契约一致.
- [x] T020 [US2] 在同文件实现 `auto_restart_and_manual_restart_share_fence_gate_test`, 竞态场景下不出现双 `spawn_once` 句柄, 失败路径必须通过 `ChildRestartConflict` 或等价可观测链路证明.

### Implementation for User Story 2

- [x] T021 [US2] 在 `src/runtime/control_loop.rs` 串联 `execute_restart_decision`, `restart_strategy_scope` 与 `handle_child_exit`: 在任何 `spawn_child_start` 前检查 `GenerationFenceState` 与 `PendingRestart`, 若 `pending restart`(待重启) 已存在则禁止自动重启路径抢先分配新的 `generation`.
- [x] T022 [US2] 在同文件 `RestartChild` 分支实现 `AlreadyPending` 合并语义, `duplicate_request_count` 递增且 `command_id` 对齐 `research.md`.
- [x] T023 [US2] 在 `src/event/payload.rs` 定义 `ChildRestartConflict` 并在 `src/observe/pipeline.rs` 接线 `emit`(发送).

**Checkpoint(检查点)**: T019,T020 通过, FR-002 与 SC-001 被测试间接支撑.

---

## Phase 5(阶段五): User Story 3(用户故事三) - 处理迟到的旧代报告 (P3)

**Goal(目标)**: 迟到退出报告归为 `stale report`(过期报告), 不覆盖当前代次真相, `event`(事件), `audit`(审计), `metrics`(指标) 可追踪.

### Tests for User Story 3

- [x] T024 [US3] 在 `src/tests/supervisor_generation_fencing_test.rs` 实现 `stale_exit_report_never_overwrites_current_attempt_test`.

### Implementation for User Story 3

- [x] T025 [US3] 在 `src/runtime/control_loop.rs` 的完成报告入口处对 `(child_id, generation, attempt)` 做三分支分发: 当前活动, `pending restart`(待重启)中的旧实例, 或降级为 `stale`(过期).
- [x] T026 [US3] 在 `src/event/payload.rs` 增补 `ChildAttemptStaleReport`, 并经 `src/observe/pipeline.rs` 发送. US1 `fence`(隔离) 三件事已在 T014–T018 完成, 本节不得再以「`fence`(隔离) `lifecycle`(生命周期) 其余事件」为理由回填 US1 义务.
- [x] T027 [US3] 在 `src/observe/pipeline.rs` 与 `src/observe/metrics.rs` 落实 `contracts/generation-fencing.md` 中 Metrics Contract(指标契约) 与 Audit Contract(审计契约) 所列与本功能相关的全部条目: `supervisor_child_restart_fence_total` 的 `result` 标签取值须覆盖 entered, released, abort_requested, already_pending 与 rejected; `supervisor_child_attempt_stale_report_total` 仅使用 `handled_as` 标签且不得附带高基数 `child_id` 标签; `supervisor_child_restart_pending_total` 以 `gauge`(仪表) 类型反映当前待重启请求数量. 同步将 `child control audit`(子任务控制审计) 扩展为包含 `command_id`, `generation_fence_decision`, `stale_report`, `failure` 等契约字段. `child_id` 仍不得作为高基数 `metric`(指标) 标签.

**Checkpoint(检查点)**: T024 通过, SC-003 满足.

---

## Phase 6(最终阶段): Polish & Cross-Cutting Concerns(收尾)

**Purpose(目的)**: 收口 `src/tests/supervisor_event_shape_test.rs` 中事件形状断言, 与 `manual`(手册) 中 `dashboard`(仪表盘) 相关章节, `quickstart`(快速开始) 的第 2 节至第 7 节命令矩阵, `specs/004-4-generation-fencing/plan.md` Technical Context(技术背景) 与仓库 `Cargo.toml` 依赖的一致性.

- [x] T028 [P] 更新 `src/tests/supervisor_event_shape_test.rs` 覆盖本功能新增事件必填字段形状. 若上文 Phase 5(阶段五) 中 T027 已在 `src/observe/pipeline.rs` 与 `src/observe/metrics.rs` 收口指标与审计钩子, 本节以事件载荷与序列化形状回归为主, 并与 `supervisor_generation_fencing_test` 或 `observability_smoke_test` 交叉核验.
- [x] T029 [P] 在代码事实变更前提下更新 `manual/en/dashboard.md` 与 `manual/zh/dashboard.md`, 并运行 `cargo test --test supervisor_docs_sync_test`.
- [x] T030 按 `specs/004-4-generation-fencing/quickstart.md` 第 2 节至第 7 节逐项执行列出的 `cargo test` 与人工检查条目, 含受影响监督器集成测试套件与 `naming contract`(命名契约).
- [x] T031 若 `specs/004-4-generation-fencing/plan.md` 中 Technical Context(技术背景) 与实际 Cargo(构建工具) 依赖不符, 则更新 Technical Context 与 Complexity Tracking(复杂度跟踪) 小节; 若无新增 `crate`(库)依赖则写明 无.
- [x] T032 格式化策略: `CI`(持续集成)或本地 `pre-push`(推送前)优先使用 `cargo fmt --check` 门禁. 如需消除已知格式漂移可运行 `cargo fmt` 重写工作区. T002 已执行 `cargo fmt --check` 通过后, T032 可以再次仅运行 `fmt --check` 完成本条, 不得在零差异时仅为勾掉任务而无谓改写文件.
- [x] T033 按 **`speckit.sync.proposals`** **Proposal P7** **APPLIED**, 在 **`spec.md`** 增补 **`FR-004`**, **`SC-005`**, **`Key Entities`**, **Edge Cases**, 在 **`contracts/generation-fencing.md`** **Runtime Semantics** 增补 **DelayedSpawnAttached 与正 backoff** 专节, 在 **`plan.md`** **Technical Context** 增补 **Delayed spawn mailbox** 段, 并同步漂移表 **P7** **APPLIED** 落账日期 **2026-05-15**.
- [x] T034 **Proposal P8** **APPLIED**: **`plan.md`** **`Testing`** 增补 **`Stale report test replay`**, **`Complexity Tracking`** 互指, 漂移落账 **2026-05-15**.

---

## Dependencies & Execution Order(依赖和执行顺序)

| Phase | 依赖前置 |
|-------|---------|
| 1 | 无 |
| 2 | 1 |
| 3 | 2 |
| 4 | 3 |
| 5 | 4 |
| 6 | 5 |

### User Story Dependencies(用户故事依赖)

串联顺序必须为 Phase 3(阶段三) 先于 Phase 4(阶段四) 先于 Phase 5(阶段五). US2 依赖 US1. US3 的实现依赖主干上已落地的 US2 冲突与门禁事实 (Phase 4(阶段四) T019–T023) 以及 US1 建立的 `PendingRestart` 三元身份判别与 `fence`(隔离) `lifecycle`(生命周期) (Phase 3(阶段三) T014–T018); 不得在缺少 Phase 4(阶段四) 合并事实的情形下宣称 Phase 5(阶段五) 功能完成.

### Within Each User Story(每个用户故事内部)

每个故事的 Tests(测试)先于 Implementation(实现), Phase 2(阶段二) Foundational Implementation 先于 Foundational Tests 的情形见该节 NOTE.

---

## Parallel Opportunities(并行机会)

Phase 6(阶段六) T028 与 T029 可并行. Phase 2(阶段二) T008 与 T009 可并行批次. 触及 `control_loop.rs` 的多数任务仍需串行避免冲突.

---

## Parallel Example(并行示例): Phase 6

```bash
任务 A 编辑 src/tests/supervisor_event_shape_test.rs
任务 B 编辑 manual/zh/dashboard.md 与 manual/en/dashboard.md
```

---

## Implementation Strategy(实现策略)

### MVP First(先做最小可用产品)

完成 Phase 1–3 后停顿验证 US1.

### Incremental Delivery(增量交付)

每阶段结束运行该阶段 Checkpoint 中的命令再继续.

---

## Notes(说明)

若实现时发现契约须微调, 先更新 `contracts/generation-fencing.md` 与 `spec.md` 后再改 Rust(编程语言) 代码路径.
