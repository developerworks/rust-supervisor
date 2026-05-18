# Tasks(任务): 运行时生命周期守卫

**Input(输入)**: 设计文档来自 `/specs/004-1-runtime-lifecycle-guard/`
**Prerequisites(前置文档)**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/runtime-control-plane.md`, `quickstart.md`

**Tests(测试)**: 本功能改变 runtime control loop(运行时控制循环) 生命周期语义, 所以必须先写测试任务, 再写实现任务.

**Organization(组织方式)**: 任务按用户故事分组, 每个故事都可以独立验证.

## Phase 1(阶段一): Setup(共享基础设施)

**Purpose(目的)**: 为运行时生命周期测试和新模块准备最小结构.

- [X] T001 在 `Cargo.toml` 中注册 `supervisor_runtime_lifecycle_test` 测试目标, 路径为 `src/tests/supervisor_runtime_lifecycle_test.rs`.
- [X] T002 [P] 在 `src/runtime/mod.rs` 中声明 `lifecycle` 和 `watchdog` 模块, 不添加 `pub use`.
- [X] T003 [P] 在 `src/tests/supervisor_docs_sync_test.rs` 中加入运行时生命周期公共方法文档同步检查项.

---

## Phase 2(阶段二): Foundational(测试骨架)

**Purpose(目的)**: 建立所有用户故事都会复用的外部测试骨架.

**Critical(关键要求)**: 本阶段只准备测试文件和断言辅助, 不实现生产行为.

- [X] T004 在 `src/tests/supervisor_runtime_lifecycle_test.rs` 中创建测试文件, 写入模块注释, 公共 imports(导入) 和共享断言函数.
- [X] T005 [P] 在 `src/tests/observability_smoke_test.rs` 中创建运行时控制面可观察性测试分组, 不修改生产代码.

**Checkpoint(检查点)**: 测试骨架已经存在, 用户故事测试可以开始编写.

---

## Phase 3(阶段三): User Story 1(用户故事一) - 查询运行时健康状态 (Priority(优先级): P1) MVP(最小可用产品)

**Goal(目标)**: 操作者在 Supervisor(监督器) 启动后可以立即读取 alive(存活) 健康状态和控制循环启动事件.

**Independent Test(独立测试)**: 启动一个 Supervisor(监督器), 立即调用 `is_alive` 和 `health`, 并订阅运行时事件验证启动信号.

### Tests for User Story 1(用户故事一的测试)

- [X] T006 [US1] 在 `src/tests/supervisor_runtime_lifecycle_test.rs` 中添加启动后 `is_alive` 返回 true(真) 的测试.
- [X] T007 [US1] 在 `src/tests/supervisor_runtime_lifecycle_test.rs` 中添加启动后 `health` 返回 alive(存活), 启动时间和最近观测时间的测试.
- [X] T008 [US1] 在 `src/tests/supervisor_runtime_lifecycle_test.rs` 中添加订阅运行时控制循环启动事件的测试.

### Implementation for User Story 1(用户故事一的实现)

- [X] T009 [US1] 在 `src/runtime/lifecycle.rs` 中定义并实现 `RuntimeControlPlaneState`, `RuntimeHealthReport`, `RuntimeExitReport`, `RuntimeFailureReason` 和 alive(存活) 健康状态查询, 并补齐模块, 结构体, 字段, 函数和 doctest(文档测试) 注释.
- [X] T010 [US1] 在 `src/runtime/watchdog.rs` 中实现控制循环启动时的状态标记和启动事件发布.
- [X] T011 [US1] 在 `src/runtime/supervisor.rs` 中让 `Supervisor::start_with_policy` 创建并保存 `RuntimeControlPlane(运行时控制面)` 和 watchdog(看门狗).
- [X] T012 [US1] 在 `src/control/handle.rs` 中为 `SupervisorHandle` 增加 `is_alive` 和 `health` 方法, 并保持现有控制命令行为不变.

**Checkpoint(检查点)**: 用户故事一已经完整可用, 并且可以独立测试.

---

## Phase 4(阶段四): User Story 2(用户故事二) - 提前发现控制循环异常退出 (Priority(优先级): P2)

**Goal(目标)**: 控制循环异常退出后, 操作者在发送下一条控制命令前就能看到结构化故障信号.

**Independent Test(独立测试)**: 通过测试夹具让控制循环异常退出, 验证健康状态, typed event(类型化事件), metrics(指标) 和 audit log(审计日志) 都包含阶段和原因.

### Tests for User Story 2(用户故事二的测试)

- [X] T013 [US2] 在 `src/tests/supervisor_runtime_lifecycle_test.rs` 中添加控制循环异常退出后 `health` 返回 not alive(非存活) 和结构化失败原因的测试.
- [X] T014 [US2] 在 `src/tests/supervisor_runtime_lifecycle_test.rs` 中添加异常退出后下一条普通控制命令返回已知退出原因的测试.
- [X] T015 [P] [US2] 在 `src/tests/observability_smoke_test.rs` 中添加运行时控制面 failed(失败) 事件映射到 metrics(指标) 和 audit log(审计日志) 的测试.

### Implementation for User Story 2(用户故事二的实现)

- [X] T016 [US2] 在 `src/test_support/factory.rs` 中新增运行时控制面异常退出测试夹具, 让外部测试可以触发控制循环异常退出而不暴露兼容导出.
- [X] T017 [US2] 在 `src/runtime/control_loop.rs` 中让 `run_control_loop` 返回 `RuntimeExitReport(运行时退出报告)`, 并区分正常返回和内部错误阶段.
- [X] T018 [US2] 在 `src/runtime/watchdog.rs` 中等待控制循环 `JoinHandle(任务句柄)`, 并把 panic(恐慌), 取消和异常结果转换为 failed(失败) 健康状态.
- [X] T019 [US2] 在 `src/control/handle.rs` 中让控制循环结束后的普通控制命令错误包含已知退出阶段和原因.
- [X] T020 [US2] 在 `src/event/payload.rs` 中新增运行时控制面 typed event(类型化事件) payload(载荷), 并完成 `RuntimeControlLoopFailed` 字段映射.
- [X] T021 [US2] 在 `src/observe/metrics.rs` 中新增 `supervisor_runtime_control_loop_exit_total` 和 `supervisor_runtime_control_plane_alive` 指标名称, 并把 failed(失败) 和 completed(已完成) 事件映射到运行时控制循环退出指标.
- [X] T022 [US2] 在 `src/observe/pipeline.rs` 中把控制面失败事件写入 structured log(结构化日志), metrics(指标), tracing(结构化追踪) 和 audit log(审计日志) 记录.

**Checkpoint(检查点)**: 用户故事二已经完整可用, 并且可以在没有新控制命令的情况下观察到控制面失败.

---

## Phase 5(阶段五): User Story 3(用户故事三) - 等待和关闭运行时控制面 (Priority(优先级): P3)

**Goal(目标)**: 操作者可以主动关闭控制面, 等待控制循环结束, 并重复读取同一个最终结果.

**Independent Test(独立测试)**: 调用 `shutdown` 后再调用 `join` 10 次, 验证每次都在 1 秒内返回相同 `RuntimeExitReport(运行时退出报告)`.

### Tests for User Story 3(用户故事三的测试)

- [X] T023 [US3] 在 `src/tests/supervisor_runtime_lifecycle_test.rs` 中添加 `shutdown` 正常结束控制面并返回 completed(已完成) 结果的测试.
- [X] T024 [US3] 在 `src/tests/supervisor_runtime_lifecycle_test.rs` 中添加重复 `join` 10 次都返回相同最终结果且不挂起的测试.
- [X] T025 [US3] 在 `src/tests/supervisor_runtime_lifecycle_test.rs` 中添加控制面已结束后重复 `shutdown` 幂等返回最终结果的测试.

### Implementation for User Story 3(用户故事三的实现)

- [X] T026 [US3] 在 `src/runtime/message.rs` 中新增内部 `RuntimeLoopMessage::ControlPlane(ControlPlaneMessage::Shutdown)` 消息, 并让控制循环收到后返回 completed(已完成) 退出报告.
- [X] T027 [US3] 在 `src/runtime/lifecycle.rs` 中实现最终退出报告缓存和重复 `join` 等待逻辑.
- [X] T028 [US3] 在 `src/control/handle.rs` 中实现 `join` 和 `shutdown` 方法, 并校验 `requested_by` 和 `reason` 非空.
- [X] T029 [US3] 在 `src/runtime/watchdog.rs` 中发布 shutdown requested(已请求关闭), completed(已完成) 和 join completed(等待结束已完成) 诊断事件.
- [X] T030 [US3] 在 `src/event/payload.rs` 中补齐 shutdown requested(已请求关闭), completed(已完成) 和 join completed(等待结束已完成) 事件字段.

**Checkpoint(检查点)**: 所有用户故事都可以独立工作.

---

## Phase 6(阶段六): Polish & Cross-Cutting Concerns(收尾和横向事项)

**Purpose(目的)**: 同步文档并完成验证.

- [X] T031 [P] 在 `manual/zh/runtime-control.md` 中说明 `is_alive`, `health`, `join` 和 `shutdown` 的调用者语义.
- [X] T032 [P] 在 `README.zh.md` 中补充运行时控制面健康查询和等待能力说明.
- [X] T033 在 `specs/004-1-runtime-lifecycle-guard/quickstart.md` 中对照执行 `cargo test --test supervisor_runtime_lifecycle_test`.
- [X] T034 在 `specs/004-1-runtime-lifecycle-guard/quickstart.md` 中对照执行 `cargo test --test supervisor_control_test --test observability_smoke_test`.
- [ ] T035 在 `specs/004-1-runtime-lifecycle-guard/quickstart.md` 中对照执行 `cargo test`.
- [X] T036 在仓库根目录对照 `specs/004-1-runtime-lifecycle-guard/quickstart.md` 执行 `cargo fmt --check`.
- [X] T037 按同步决议 **speckit.sync.proposals** 中 **Proposal P6** **APPLIED**, 把 **`specs/004-1-runtime-lifecycle-guard/spec.md`** 头部 **`Status(状态)`** 设为 **`Accepted(已接受)`**, 并写入 **`Updated(更新日期)`** **2026-05-15** 与漂移同步批次落账一致.

---

## Dependencies & Execution Order(依赖和执行顺序)

### Phase Dependencies(阶段依赖)

- **Setup(阶段一)**: 没有依赖, 可以立即开始.
- **Foundational(阶段二)**: 依赖 Setup(阶段一) 完成, 并阻塞所有用户故事.
- **User Story 1(用户故事一)**: 依赖 Foundational(阶段二) 完成, 是 MVP(最小可用产品).
- **User Story 2(用户故事二)**: 依赖 Foundational(阶段二) 完成, 可以在 US1(用户故事一) 类型存在后独立验证.
- **User Story 3(用户故事三)**: 依赖 Foundational(阶段二) 完成, 可以在 US1(用户故事一) 健康状态模型存在后独立验证.
- **Polish(收尾阶段)**: 依赖选定用户故事完成.

### User Story Dependencies(用户故事依赖)

- **US1(用户故事一)**: 提供最小健康查询能力, 是建议 MVP(最小可用产品).
- **US2(用户故事二)**: 复用 US1(用户故事一) 的健康报告类型, 但是异常退出测试可以独立运行.
- **US3(用户故事三)**: 复用 US1(用户故事一) 的最终状态模型, 但是 shutdown(关闭) 和 join(等待结束) 语义可以独立测试.

### Within Each User Story(每个用户故事内部)

- 先写该故事的测试任务, 并确认实现前失败.
- 再写生产实现任务.
- 最后运行该故事对应测试命令.

### Parallel Opportunities(并行机会)

- T002 和 T003 可以并行, 因为它们修改不同文件.
- T004 和 T005 可以并行, 因为它们修改不同测试文件.
- T015 可以和 T013, T014 并行编写, 因为它修改 `src/tests/observability_smoke_test.rs`, 其他两个任务修改 `src/tests/supervisor_runtime_lifecycle_test.rs`.
- T031 和 T032 可以并行, 因为它们修改不同文档文件.

---

## Parallel Example(并行示例): User Story 2(用户故事二)

```bash
# 同时启动用户故事二的测试任务:
Task(任务): "在 src/tests/supervisor_runtime_lifecycle_test.rs 中添加异常退出健康状态测试"
Task(任务): "在 src/tests/observability_smoke_test.rs 中添加 failed(失败) 事件 metrics(指标) 和 audit log(审计日志) 映射测试"
```

---

## Implementation Strategy(实现策略)

### MVP First(先做最小可用产品)

1. 完成 Phase 1(阶段一) 和 Phase 2(阶段二).
2. 完成 Phase 3(阶段三) User Story 1(用户故事一).
3. 运行 `cargo test --test supervisor_runtime_lifecycle_test`.
4. 只交付 `is_alive` 和 `health` 时, 调用者已经可以判断控制循环是否存活.

### Incremental Delivery(增量交付)

1. 增加 US1(用户故事一), 交付启动后健康查询.
2. 增加 US2(用户故事二), 交付异常退出主动诊断.
3. 增加 US3(用户故事三), 交付 shutdown(关闭) 和 join(等待结束) 幂等.
4. 每个故事都必须独立测试, 不得破坏已经完成的故事.

### Parallel Team Strategy(并行团队策略)

1. 团队先完成 Setup(阶段一) 和 Foundational(阶段二).
2. Foundational(阶段二) 完成后, 一个开发者实现 US1(用户故事一), 另一个开发者可以先写 US2(用户故事二) 的观察性测试.
3. 所有标记 `[P]` 的任务必须由不同子代理或不同开发者并行执行, 并且不得修改同一文件.
