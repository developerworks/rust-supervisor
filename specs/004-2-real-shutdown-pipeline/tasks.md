# Tasks(任务): 真实关闭流水线

**Input(输入)**: 设计文档来自 `specs/004-2-real-shutdown-pipeline/`
**Prerequisites(前置文档)**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/shutdown-pipeline.md`, `quickstart.md`

**Tests(测试)**: 本功能改变 supervision(监督) 关闭行为, 所以每个用户故事必须先写测试, 再写实现.

**Organization(组织方式)**: 任务按用户故事分组. `US1` 交付取消送达. `US2` 交付按序等待和优雅结果. `US3` 交付强制中止和对账报告.

## Format(格式): `[ID] [P?] [Story] Description(描述)`

- **[P]**: 可以并行执行, 因为任务修改不同文件, 并且不依赖未完成任务.
- **[Story]**: 标记任务属于哪个用户故事, 例如 `US1`, `US2`, `US3`.
- 任务描述必须写出准确文件路径.

## Phase 1(阶段一): Setup(共享基础设施)

**Purpose(目的)**: 建立真实关闭流水线需要的测试入口和模块入口.

**Gate(门禁)**: 进入 Phase 2 前须确认 `004-1-runtime-lifecycle-guard` 规格所定义的运行时控制循环存活, 健康, `join(等待结束)` 与幂等语义已在当前合并基线中可用; 否则应先完成零四杠一相关交付或变基, 再执行本清单中依赖控制循环句柄与生命周期的任务.

- [X] T001 在 `Cargo.toml` 中新增 `supervisor_real_shutdown_pipeline_test` 测试目标, 路径指向 `src/tests/supervisor_real_shutdown_pipeline_test.rs`.
- [X] T002 在 `src/runtime/mod.rs` 中声明 `shutdown_pipeline` 模块, 在 `src/shutdown/mod.rs` 中声明 `report` 模块, 并确认不添加 `pub use` 兼容导出.
- [X] T003 在 `src/tests/supervisor_real_shutdown_pipeline_test.rs` 中创建测试文件和共享测试工厂骨架.

---

## Phase 2(阶段二): Foundational(阻塞前置基础)

**Purpose(目的)**: 先建立所有用户故事都会依赖的运行时类型, 句柄边界和公开结果边界.

**Critical(关键要求)**: 本阶段完成前, 不得开始任何用户故事实现.

- [X] T004 在 `src/shutdown/report.rs` 中定义 `ShutdownPipelineReport`, `ChildShutdownOutcome`, `ChildShutdownStatus`, `ShutdownReconcileReport` 和 `ResourceReconcileStatus` 的初始类型.
- [X] T005 在 `src/shutdown/coordinator.rs` 中为 `ShutdownResult` 增加 `report: Option<ShutdownPipelineReport>`, 并保持原有 `phase`, `cause` 和 `idempotent` 语义, 同时避免 `src/shutdown/` 依赖 `src/runtime/`.
- [X] T006 [P] 在 `src/task/context.rs` 中新增使用外部 `CancellationToken(取消令牌)` 创建 `TaskContext(任务上下文)` 的构造函数.
- [X] T007 在 `src/child_runner/runner.rs` 中新增可持有 `CancellationToken(取消令牌)` 和真实 child future(子任务 future) `AbortHandle(强制中止句柄)` 的 `ChildRunHandle(子任务运行句柄)` 或等价类型.
- [X] T008 在 `src/runtime/control_loop.rs` 中新增 active attempt(活动尝试) 集合, 用来保存 `child_id(子任务标识)`, `generation(代次)`, `attempt(尝试)`, token(令牌), abort handle(强制中止句柄) 和完成接收端.
- [X] T009 在 `src/control/command.rs` 中确认 `CommandResult::Shutdown(关闭命令结果)` 继续返回扩展后的 `ShutdownResult(关闭结果)`, 并且不新增控制命令变体.

**Checkpoint(检查点)**: 运行时已经能表达真实关闭流水线所需的句柄, token(令牌) 和报告类型.

---

## Phase 3(阶段三): User Story 1(用户故事一) - 请求所有任务协作关闭 (Priority(优先级): P1) MVP(最小可用产品)

**Goal(目标)**: `ShutdownTree(关闭监督树)` 请求必须向所有运行中的 child task(子任务) 发送取消信号, 并且不得重复取消已经结束的任务.

**Independent Test(独立测试)**: 启动多个长运行任务后请求关闭, 测试必须证明每个运行中任务都观察到 token(令牌) 已取消, 已退出任务不会再次收到取消.

### Tests for User Story 1(用户故事一的测试)

- [X] T010 [US1] 在 `src/tests/supervisor_real_shutdown_pipeline_test.rs` 中添加多个运行中 child task(子任务) 收到 `CancellationToken(取消令牌)` 的失败优先测试.
- [X] T011 [US1] 在 `src/tests/supervisor_real_shutdown_pipeline_test.rs` 中添加关闭前已经退出或没有运行中 child task(子任务) 时输出 `AlreadyExited(已经退出)` 且不重复取消的失败优先测试.
- [X] T012 [P] [US1] 在 `src/tests/observability_smoke_test.rs` 中添加取消送达事件可观察的失败优先测试.

### Implementation for User Story 1(用户故事一的实现)

- [X] T013 [US1] 在 `src/runtime/shutdown_pipeline.rs` 中实现 `RequestStop(请求停止)` 阶段的取消送达逻辑, 并记录已送达 child(子任务) 集合和无活动尝试 child(子任务) 的 `AlreadyExited(已经退出)` 结果.
- [X] T014 [US1] 在 `src/runtime/control_loop.rs` 中把 `ShutdownTree(关闭监督树)` 路由到真实 `ShutdownPipeline(关闭流水线)`, 并停止立即推进到 `Completed(已完成)` 的旧逻辑.
- [X] T015 [US1] 在 `src/child_runner/runner.rs` 中让运行器把 runtime(运行时) 保存的 token(令牌) 传入 `TaskContext(任务上下文)`.
- [X] T016 [P] [US1] 在 `src/event/payload.rs` 中新增或扩展 child cancellation delivered(子任务取消已送达) 事件载荷和事件名.
- [X] T017 [US1] 在 `src/runtime/control_loop.rs` 中确保关闭期间自动 restart policy(重启策略) 不会重新启动被取消的 child(子任务).

**Checkpoint(检查点)**: `US1` 可以独立验证. 此时关闭可以真实取消运行中任务, 但还不要求完整等待顺序和强制中止.

---

## Phase 4(阶段四): User Story 2(用户故事二) - 按关闭顺序等待任务结束 (Priority(优先级): P2)

**Goal(目标)**: 运行时必须按 `shutdown_order(关闭顺序)` 等待 child task(子任务) 正常返回, 并把优雅完成结果写入关闭摘要.

**Independent Test(独立测试)**: 构造有依赖关系的 supervisor tree(监督树), 请求关闭后验证等待顺序和每个 child(子任务) 的 `Graceful(优雅完成)` 结果.

### Tests for User Story 2(用户故事二的测试)

- [X] T018 [US2] 在 `src/tests/supervisor_real_shutdown_pipeline_test.rs` 中添加关闭等待顺序必须符合 `shutdown_order(关闭顺序)` 的失败优先测试.
- [X] T019 [US2] 在 `src/tests/supervisor_real_shutdown_pipeline_test.rs` 中添加优雅完成 child(子任务) 进入 `ChildShutdownStatus::Graceful` 的失败优先测试.
- [X] T020 [P] [US2] 在 `src/tests/supervisor_control_test.rs` 中添加 `ShutdownTree(关闭监督树)` 返回完成报告的控制命令回归测试.

### Implementation for User Story 2(用户故事二的实现)

- [X] T021 [US2] 在 `src/runtime/shutdown_pipeline.rs` 中使用 `src/tree/order.rs` 的 `shutdown_order(关闭顺序)` 生成稳定等待顺序.
- [X] T022 [US2] 在 `src/runtime/shutdown_pipeline.rs` 中实现 `GracefulDrain(优雅排空)` 阶段, 并按 `ShutdownPolicy.graceful_timeout` 限制总等待预算.
- [X] T023 [US2] 在 `src/runtime/shutdown_pipeline.rs` 中把正常返回的 `ChildRunReport(子任务运行报告)` 转换为 `ChildShutdownOutcome(子任务关闭结果)`.
- [X] T024 [US2] 在 `src/runtime/control_loop.rs` 中把关闭等待期间到达的 `ChildAttemptMessage(子任务尝试消息)` 归并到正在执行的关闭流水线.
- [X] T025 [P] [US2] 在 `src/observe/metrics.rs` 中记录 `shutdown_duration_seconds(关闭耗时秒数)` 的完整流水线耗时.

**Checkpoint(检查点)**: `US2` 可以独立验证. 此时合作任务可以按关闭顺序完成, 并返回 per-child(逐子任务) 结果.

---

## Phase 5(阶段五): User Story 3(用户故事三) - 强制中止滞留任务并完成对账 (Priority(优先级): P3)

**Goal(目标)**: 运行时必须在优雅排空超时后中止滞留任务, 再输出 registry(注册表), socket(套接字), journal(日志), metrics(指标) 和 runtime handles(运行时句柄) 对账报告.

**Independent Test(独立测试)**: 构造一个忽略取消的 child task(子任务), 请求关闭后验证它被记录为 `Aborted(已强制中止)` 或 `AbortFailed(强制中止失败)`, 并且最终报告覆盖全部 child(子任务).

### Tests for User Story 3(用户故事三的测试)

- [X] T026 [US3] 在 `src/tests/supervisor_real_shutdown_pipeline_test.rs` 中添加忽略取消的 child task(子任务) 超时后被强制中止的失败优先测试.
- [X] T027 [US3] 在 `src/tests/supervisor_real_shutdown_pipeline_test.rs` 中添加重复 `ShutdownTree(关闭监督树)` 返回缓存报告和 `idempotent(幂等)` 标记的失败优先测试.
- [X] T028 [US3] 在 `src/tests/supervisor_real_shutdown_pipeline_test.rs` 中添加迟到 child exit(子任务退出) 被归并或标记为 `LateReport(迟到报告)` 的失败优先测试.
- [X] T029 [P] [US3] 在 `src/tests/observability_smoke_test.rs` 中添加强制中止, 迟到报告和对账摘要的观测测试.
- [X] T030 [P] [US3] 在 `tests/dashboard_protocol_shape_test.rs` 中添加 dashboard protocol(仪表盘协议) 未改变 `ShutdownTree(关闭监督树)` 请求形状的回归断言.

### Implementation for User Story 3(用户故事三的实现)

- [X] T031 [US3] 在 `src/runtime/shutdown_pipeline.rs` 中实现 `AbortStragglers(强制中止滞留任务)` 阶段, 并按 `ShutdownPolicy.abort_wait` 等待中止结果.
- [X] T032 [US3] 在 `src/runtime/shutdown_pipeline.rs` 中实现 `Reconcile(对账)` 阶段, 生成 `ShutdownReconcileReport(关闭对账报告)`, 并把核心 runtime(运行时) 不拥有的 socket(套接字) 记录为 `NotOwned(非运行时拥有)`.
- [X] T033 [US3] 在 `src/runtime/shutdown_pipeline.rs` 中实现重复关闭请求的缓存报告返回和迟到报告归并逻辑.
- [X] T034 [US3] 在 `src/runtime/control_loop.rs` 中清理 active attempt(活动尝试) 集合, 更新 registry(注册表), 并保证关闭完成后不再接收新的 child restart(子任务重启).
- [X] T035 [P] [US3] 在 `src/observe/metrics.rs` 中记录 `shutdown_child_outcomes_total(子任务关闭结果总数)`, `shutdown_abort_total(关闭强制中止总数)` 和 `shutdown_late_reports_total(关闭迟到报告总数)`.
- [X] T036 [P] [US3] 在 `src/observe/pipeline.rs` 中记录关闭请求, 取消送达, 等待顺序, 强制中止集合, socket(套接字) `NotOwned(非运行时拥有)` 和对账状态的 audit(审计) 事实.
- [X] T037 [US3] 在 `src/event/payload.rs` 中补齐 `ChildShutdownGraceful(子任务优雅完成)`, `ChildShutdownAborted(子任务已强制中止)` 和 `ChildShutdownLateReport(子任务迟到报告)` 事件载荷.

**Checkpoint(检查点)**: 所有用户故事都可以独立工作. 真实关闭流水线可以覆盖合作关闭, 非合作关闭, 重复请求和资源对账.

---

## Phase 6(阶段六): Polish & Cross-Cutting Concerns(收尾和横向事项)

**Purpose(目的)**: 同步文档, 收敛命名契约, 并运行验收命令.

- [X] T038 [P] 在 `manual/zh/observability.md` 中补充真实关闭流水线的 event(事件), metrics(指标) 和 audit(审计) 观测说明.
- [X] T039 [P] 在 `README.zh.md` 中把 `ShutdownTree(关闭监督树)` 描述更新为真实取消, 等待, 强制中止和对账.
- [X] T040 在 `src/tests/supervisor_docs_sync_test.rs` 中更新文档同步断言, 确保文档提到真实关闭流水线.
- [X] T041 按 `specs/004-2-real-shutdown-pipeline/quickstart.md` 运行 `cargo fmt --check`.
- [X] T042 按 `specs/004-2-real-shutdown-pipeline/quickstart.md` 运行 `cargo test --test supervisor_real_shutdown_pipeline_test`.
- [X] T043 按 `specs/004-2-real-shutdown-pipeline/quickstart.md` 运行 `cargo test --test supervisor_control_test --test supervisor_shutdown_test --test observability_smoke_test`.
- [X] T044 按 `specs/004-2-real-shutdown-pipeline/quickstart.md` 运行 `cargo test --test dashboard_protocol_shape_test`.
- [X] T045 按 `specs/004-2-real-shutdown-pipeline/quickstart.md` 运行 `cargo test --test naming_contract_test source_code_uses_approved_state_names`.
- [X] T046 按 `specs/004-2-real-shutdown-pipeline/quickstart.md` 运行 `cargo test -- --skip checked_artifacts_avoid_forbidden_state_terms`.

---

## Dependencies & Execution Order(依赖和执行顺序)

### Phase Dependencies(阶段依赖)

- **Setup(阶段一)**: 没有依赖, 可以立即开始.
- **Foundational(阶段二)**: 依赖 Setup(阶段一) 完成, 并阻塞所有用户故事.
- **User Story 1(用户故事一)**: 依赖 Foundational(阶段二), 是 MVP(最小可用产品).
- **User Story 2(用户故事二)**: 依赖 Foundational(阶段二), 可以在 `US1` 测试骨架存在后并行设计, 但实现需要集成 `US1` 的 active attempt(活动尝试) 句柄.
- **User Story 3(用户故事三)**: 依赖 `US1` 和 `US2` 的流水线基础, 因为强制中止需要知道哪些任务没有优雅完成.
- **Polish(收尾阶段)**: 依赖所有选定用户故事完成.

### User Story Dependencies(用户故事依赖)

- **US1(P1)**: 完成后可以证明关闭请求真实取消运行中任务.
- **US2(P2)**: 在 `US1` 的取消基础上增加按序等待和优雅结果.
- **US3(P3)**: 在 `US2` 的等待基础上增加强制中止, 缓存报告, 迟到报告和对账.

### Within Each User Story(每个用户故事内部)

- 测试任务必须先写, 并且必须在实现前失败.
- 先更新模型和句柄边界, 再连接控制循环.
- 先实现运行时结果, 再补齐 event(事件), metrics(指标) 和 audit(审计).
- 完成一个故事后, 必须运行该故事对应的最小测试.

## Parallel Opportunities(并行机会)

- `T006` 可以与 `T004` 和 `T005` 并行, 因为它只修改 `src/task/context.rs`.
- `T012` 可以与 `T010` 和 `T011` 并行, 因为它修改 `src/tests/observability_smoke_test.rs`, 而主行为测试在 `src/tests/supervisor_real_shutdown_pipeline_test.rs`.
- `T016` 可以与 `T013` 到 `T015` 并行, 因为事件载荷文件独立.
- `T020` 可以与 `T018` 和 `T019` 并行, 因为控制命令回归测试文件独立.
- `T025` 可以与 `T021` 到 `T024` 并行, 因为指标文件独立.
- `T029` 和 `T030` 可以与 `T026` 到 `T028` 并行, 因为观测测试和 dashboard protocol(仪表盘协议) 测试文件独立.
- `T035` 和 `T036` 可以并行, 因为它们分别修改 metrics(指标) 和 audit(审计) 文件.
- `T038` 和 `T039` 可以并行, 因为它们修改不同文档.

## Parallel Example(并行示例): User Story 1(用户故事一)

```bash
Task: "T010 在 src/tests/supervisor_real_shutdown_pipeline_test.rs 中添加取消送达测试"
Task: "T012 在 src/tests/observability_smoke_test.rs 中添加取消送达事件测试"
Task: "T016 在 src/event/payload.rs 中新增取消送达事件载荷"
```

## Parallel Example(并行示例): User Story 3(用户故事三)

```bash
Task: "T029 在 src/tests/observability_smoke_test.rs 中添加强制中止观测测试"
Task: "T030 在 tests/dashboard_protocol_shape_test.rs 中添加协议形状回归断言"
Task: "T035 在 src/observe/metrics.rs 中添加关闭结果指标"
Task: "T036 在 src/observe/pipeline.rs 中添加关闭审计事实"
```

## Implementation Strategy(实现策略)

### MVP First(先做最小可用产品)

1. 完成 Phase 1(阶段一) 和 Phase 2(阶段二).
2. 完成 `US1(用户故事一)`, 先证明所有运行中任务都收到取消.
3. 运行 `cargo test --test supervisor_real_shutdown_pipeline_test` 中与取消相关的测试.
4. 在 `US1` 通过后再推进 `US2` 和 `US3`.

### Incremental Delivery(增量交付)

1. 交付 `US1(用户故事一)`: 真实取消送达.
2. 交付 `US2(用户故事二)`: 按关闭顺序等待和优雅结果摘要.
3. 交付 `US3(用户故事三)`: 强制中止, 重复请求, 迟到报告和资源对账.
4. 每个故事完成后运行对应测试, 并且不得破坏之前故事.

### Final Validation(最终验证)

最终实现必须运行 `T041` 到 `T046` 中列出的命令. 如果完整 `cargo test` 被 sibling UI(同级用户界面) 命名契约阻塞, 必须在最终汇报中说明阻塞项, 并保留 `--skip checked_artifacts_avoid_forbidden_state_terms` 的近似全量验收结果.
