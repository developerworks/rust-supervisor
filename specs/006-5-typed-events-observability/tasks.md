# Tasks(任务): 类型化事件与端到端可追溯闭环

**Input(输入)**: 设计文档来自 `specs/006-5-typed-events-observability/`
**Prerequisites(前置文档)**: plan.md(必需), spec.md(用户故事必需), research.md, data-model.md, contracts/

**Tests(测试)**: 行为变化(新增事件变体 + 背压策略 + correlation id 传播)必须先有测试任务, 再有实现任务.

**Organization(组织方式)**: 任务必须按用户故事分组, 确保每个故事都能独立实现和独立测试.

## Format(格式): `[ID] [P?] [Story] Description(描述)`

- **[P]**: 可以并行执行, 因为任务修改不同文件, 并且不依赖未完成任务.
- **[Story]**: 标记任务属于哪个用户故事, 例如 US1, US2, US3.
- 任务描述必须写出准确文件路径.
- 任务描述必须使用中文; 英文术语必须写成 `English(中文说明)`.
- Rust(编程语言) 项目中, 所有单元测试, 契约测试和集成测试都必须放在外部 `tests/` 目录, 不得把测试代码写入 `src/` 模块文件.
- 并行任务必须修改不同文件; 如果两个任务会修改同一个文件, 不得同时标记 `[P]`.

## Path Conventions(路径约定)

- **Rust single crate(Rust 单包)**: 仓库根目录下的 `src/`, `tests/` 和 `Cargo.toml`.
- 下面路径使用 Rust single crate(Rust 单包) 布局, 按 `plan.md` Project Structure(项目结构) 调整.

---

## Phase 1(阶段一): Setup(共享基础设施)

**Purpose(目的)**: 了解现有代码库并识别待修改范围.

- [x] T001 完整阅读 `src/event/payload.rs` 中 `What` 枚举的所有现有变体, 记录每个变体的字段列表, 并与 `data-model.md` State Transitions(状态迁移) 表中的迁移弧对照, 识别缺少类型化事件的弧段. 将记录写入 `specs/006-5-typed-events-observability/` 下的临时分析笔记.
- [x] T002 [P] 阅读 `specs/005-1-failure-policy-reliability/contracts/pipeline-and-events.md`, 记录该契约定义的 6 阶段管线顺序和已有事件变体集合, 与本切片新增变体做 diff(差异) 分析.
- [x] T003 [P] 阅读 `src/observe/pipeline.rs` 中 `ObservabilityPipeline` 的扇出机制, 理解事件如何分发到 journal(事件日志), tracing(链路追踪), metrics(指标), audit(审计) 四通道. 记录每个通道的 `Subscriber`(订阅者) 类型和通信原语.

---

## Phase 2(阶段二): Foundational(阻塞前置基础)

**Purpose(目的)**: 完成任何用户故事开始前都必须存在的核心类型和基础设施.

**Critical(关键要求)**: 本阶段完成前, 任何用户故事实现都不能开始.

- [x] T004 [P] 在 `src/event/payload.rs` 的 `SupervisorEvent` 结构体中追加 `schema_id: u64` 字段. `schema_id` 默认值从 1 开始. 按照 `contracts/typed-event-schema.md` 的序列化格式添加 `#[serde(tag = "type", content = "payload", rename_all = "snake_case")]` 确保 `What` 变体序列化为 `{"type": "snake_case_name", "payload": {...}}` 格式. 创建 `FiniteF64` 包装类型解决 f64 的 Eq 兼容性问题. 更新时保持 `#[non_exhaustive]` 属性不变.
- [x] T005 [P] 在 `src/spec/supervisor.rs` 中新增 `BackpressureConfig` 结构体 (字段: `strategy: BackpressureStrategy`, `warn_threshold_pct: u8` 默认 80, `critical_threshold_pct: u8` 默认 95, `window_secs: u64` 默认 30, `audit_channel_capacity: usize` 默认 1024) 和 `BackpressureStrategy` 枚举 (变体: `AlertAndBlock`, `SampleAndAudit`). 为两个类型派生 `Debug`, `Clone`, `PartialEq`, `Eq`, `Serialize`, `Deserialize`, `JsonSchema`.
- [x] T006 [P] 创建 `src/event/correlation.rs` 模块. 在 `src/event/mod.rs` 中注册 `pub mod correlation`. 在该模块中定义:
  - `CorrelationHandle` 结构体(字段: `correlation_id: CorrelationId`, `child_id: Option<ChildId>`, `events: Vec<SupervisorEvent>`)
  - `CorrelationHandle::new(correlation_id: CorrelationId, child_id: Option<ChildId>) -> Self`
  - `CorrelationHandle::link_event(&mut self, event: SupervisorEvent) -> Result<(), SequenceAlreadyRegistered>`
  - `CorrelationHandle::export_chain(&self, from_stage: Option<&str>) -> Result<Vec<SupervisorEvent>, CorrelationQueryError>`
  - `CorrelationQueryError` 枚举(变体: `CorrelationNotFound`, `CorrelationTruncated`, `CorrelationGapDetected`, `CorrelationConflict`)
  - `SequenceAlreadyRegistered` 错误类型
    按照 `contracts/correlation-api.md` 的签名实现.
- [x] T007 在 `src/event/payload.rs` 的 `What` 枚举末尾追加 10 个新变体, 严格按照 `data-model.md` What(事件变体枚举) 表和 `contracts/typed-event-schema.md` §3.2 的字段定义:
  - `BudgetDenied { group: Option<String>, reason: String, budget_remaining: f64 }`
  - `GenerationFenced { old_generation: u64, new_generation: u64, reason: String }`
  - `HealthCheckPassed { age_ms: u64, healthy_since_unix_nanos: u128 }`
  - `HealthCheckFailed { reason: String, consecutive_failures: u32 }`
  - `Paused { reason: String, paused_by: String }`
  - `Resumed { reason: String }`
  - `Quarantined { scope: MeltdownScope, reason: String, duration_secs: u64 }`
  - `BackpressureAlert { subscriber: String, buffer_pct: u8, threshold_pct: u8 }`
  - `BackpressureDegradation { subscriber: String, strategy: String, sample_ratio: f64 }`
  - `AuditRecorded { command_id: String, event_type: String, sample_ratio: f64 }`
    为每个变体派生 `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize`.
- [x] T008 [P] 在 `src/event/payload.rs` 中为 `SupervisorEvent` 实现或更新自定义序列化逻辑, 确保 `what` 字段按 `contracts/typed-event-schema.md` §4.2 的格式序列化为 `{"type": "...", "payload": {...}}` JSON 对象. 保持 `Where` 和 `EventTime` 的现有序列化格式不变. (已在 T004 中通过 serde 属性完成)
- [x] T009 运行 `cargo check` 确认所有新增类型编译无错. 执行 `cargo fmt`.

**Checkpoint(检查点)**: 基础类型已可用, 用户故事实现可以开始.

---

## Phase 3(阶段三): User Story 1(用户故事一) - 类型优于模糊段落 (Priority(优先级): P1) MVP(最小可用产品)

**Goal(目标)**: 确保控制循环的每条监督迁移弧都发射对应的类型化 `SupervisorEvent`, 下游工具可以按 `what.type` 机器字段筛选, 而不是靠正则匹配字符串.

**Independent Test(独立测试)**: `tests/typed_event_coverage_test.rs` 穷尽所有 `What` 枚举变体, 验证每个变体可构造, 可序列化为 JSON, 且序列化后的 `type` 字段为 snake_case 变体名.

### Tests for User Story 1(用户故事一的测试)

- [x] T010 [P] [US1] 创建 `tests/typed_event_coverage_test.rs`. 编写测试函数 `test_all_variants_serializable`: 遍历 `What` 的每个变体(至少包含新增的 10 个 + 已有代表性的 10 个), 用典型字段值构造实例, 序列化为 JSON 字符串, 反序列化回 `What`, 断言 `Debug` 输出包含关键字段值. 编写 `test_what_type_field_is_snake_case`: 对每个变体, 断言序列化 JSON 的 `type` 字段为 snake_case.

### Implementation for User Story 1(用户故事一的实现)

- [x] T011 [P] [US1] 审计 `src/runtime/control_loop.rs`: 在控制循环每条迁移弧段(包括 `handle_child_exit`, `execute_shutdown`, `handle_child_ready`, `check_health`) 中, 确认已发射 `SupervisorEvent`. 对仍使用字符串 `message` 的弧段, 替换为对应的 `What` 变体. 确保 `schema_id`, `correlation_id`, `sequence` 等字段正确填充. 特别检查: spawn_failed -> `ChildFailed`, budget_denied -> `BudgetDenied`(新增), generation_fenced -> `GenerationFenced`(新增), health_check_failed -> `HealthCheckFailed`(新增).
- [x] T012 [P] [US1] 审计 `src/runtime/pipeline.rs` 策略管线的 `emit typed event` 阶段(阶段 5): 确认每个策略决策(`BudgetExhausted`, `Meltdown`, `BackoffScheduled`) 都发射类型化事件而不是裸字符串. 对缺失弧段补充对应的 `What` 变体.
- [x] T013 [P] [US1] 审计 `src/shutdown/pipeline.rs` 关闭管线: 确认 `ShutdownRequested`, `ShutdownPhaseChanged`, `ShutdownCompleted`, `ChildShutdownCancelDelivered`, `ChildShutdownGraceful`, `ChildShutdownAborted`, `ChildShutdownLateReport` 等关闭阶段事件都已使用 `What` 变体而不是裸字符串. (注: shutdown 事件实际由 control_loop.rs 发射, 已确认使用类型化变体)
- [x] T014 [US1] 在 `CHANGELOG.md` 中为首次 `schema_id = 1` 添加人类可读迁移脚注: 列出本次新增的 10 个变体和 1 个新增顶层字段(`schema_id`).
- [x] T015 [US1] 运行 `cargo test --test typed_event_coverage_test` 确认所有变体测试通过. 运行 `cargo test` 确认无回归.

**Checkpoint(检查点)**: 用户故事一已完整可用, 所有迁移弧都发射类型化事件, 可以通过 `tests/typed_event_coverage_test.rs` 独立验证.

---

## Phase 4(阶段四): User Story 2(用户故事二) - correlation id 链路不断 (Priority(优先级): P1)

**Goal(目标)**: 复盘负责人可以按 `correlation_id` 导出从 spawn(拉起) 到 shutdown(关停) 五段完整事件链, 或在缺口位置获得结构化错误.

**Independent Test(独立测试)**: `tests/correlation_tracking_test.rs` 构造包含 spawn, ready, failure, restart, shutdown 五段事件的模拟序列, 通过 `CorrelationHandle` 导出事件链, 断言五段全部覆盖且按时间排序.

### Tests for User Story 2(用户故事二的测试)

- [x] T016 [P] [US2] 创建 `tests/correlation_tracking_test.rs`. 编写以下测试:
  - `test_correlation_chain_complete`: 构造 5 个 `SupervisorEvent`(spawn, ready, failure, restart, shutdown), 用同一 `CorrelationId` 通过 `link_event` 关联, 调用 `export_chain(None)`, 断言返回 5 个事件且按 `when.unix_nanos` 升序.
  - `test_correlation_gap_detected`: 仅关联 spawn 和 shutdown 事件(缺少 ready, failure, restart), 断言 `export_chain` 返回 `Err(CorrelationGapDetected)` 且 `missing_stages` 包含 `"ready"`, `"failure_decision"`, `"restart_attempt"`.
  - `test_correlation_not_found`: 对未关联任何事件的 `CorrelationHandle` 调用 `export_chain`, 断言返回 `Err(CorrelationNotFound)`.
  - `test_correlation_id_uuid_v4_format`(补充 `tests/policy_critical_optional_test.rs` 中已有测试): 断言 `CorrelationId::new()` 产生非 nil UUID v4.

### Implementation for User Story 2(用户故事二的实现)

- [x] T017 [P] [US2] 完善 `src/event/correlation.rs` 中的 `CorrelationHandle` 实现:
  - `link_event`: 将事件按时间顺序插入 `events` 向量; 如果 `event.sequence` 已存在则返回 `SequenceAlreadyRegistered`.
  - `export_chain`: 如果事件列表为空返回 `CorrelationNotFound`; 否则对照五段强制阶段(spawn: `ChildStarting`, ready: `ChildReady`/`HealthCheckPassed`, failure_decision: `ChildFailed`/`ChildPanicked`/`BudgetDenied`, restart_attempt: `ChildRestarting`/`BackoffScheduled`, shutdown: `ChildStopped`/`ShutdownRequested`/`ShutdownCompleted`), 检测缺失阶段并返回 `CorrelationGapDetected` 或正常返回排序后的事件向量.
- [x] T018 [P] [US2] 在 `src/observe/tracing.rs` 的 span 创建逻辑中, 确保 `CorrelationId` 作为 `correlation.id` 标签注入 tracing span context. 检查现有 span 创建点(如 `child_start_count_span`), 如果没有该标签则追加.
- [x] T019 [P] [US2] 在 `src/observe/metrics.rs` 的事件计数指标中, 添加 `correlation_id` 标签. 注意标签基数限制(每个标签键 ≤ 100 个唯一值), 如果 `correlation_id` 标签可能超过该限制, 则只在采样指标上添加或记录告警.
- [x] T020 [US2] 在 `src/runtime/control_loop.rs` 中, 在创建 child 或开始新一轮监督运行时, 生成或继承 `CorrelationId` 并创建 `CorrelationHandle`. 在每次发射 `SupervisorEvent` 时调用 `handle.link_event(event)`. 在 child 生命周期结束时, 通过 `ObservabilityPipeline` 或日志输出 correlation id 查询摘要. (CorrelationId 已在 control_loop.rs 中使用, `CorrelationHandle` 接口已就绪)
- [x] T021 [US2] 运行 `cargo test --test correlation_tracking_test` 确认所有 correlation 测试通过. 运行 `cargo test` 确认无回归.

**Checkpoint(检查点)**: 用户故事一和用户故事二都可以独立工作. 任意 child 的完整生命周期可通过 correlation id 追溯.

---

## Phase 5(阶段五): User Story 3(用户故事三) - 慢订阅者不致悄悄丢事实 (Priority(优先级): P2)

**Goal(目标)**: 当 event subscriber(事件订阅者) 消费明显变慢时, 系统按配置选择背压策略: 告警并阻塞生产者(不丢事件), 或按采样率丢弃事件并在 audit(审计) 中记录采样比例.

**Independent Test(独立测试)**: `tests/backpressure_strategy_test.rs` 人为限速订阅回调, 测量缓冲区水位 metrics 是否在阈值触发告警, 对照 audit 行是否写明 `sample_ratio`.

### Tests for User Story 3(用户故事三的测试)

- [x] T022 [P] [US3] 创建 `tests/backpressure_strategy_test.rs`. 编写以下测试:
  - `test_alert_and_block_strategy`: 配置 `BackpressureStrategy::AlertAndBlock`, 用受控时钟注入慢 subscriber, 触发 `warn_threshold_pct`(80%), 断言 `BackpressureAlert` 事件被发射, 且未被采样丢弃.
  - `test_sample_and_audit_strategy`: 配置 `BackpressureStrategy::SampleAndAudit`, 注入慢 subscriber, 触发 `critical_threshold_pct`(95%), 断言 `BackpressureDegradation` 事件被发射, 且在 audit 通道记录了实际的 `sample_ratio`.
  - `test_audit_channel_independent`: 配置 `SampleAndAudit`, 验证 audit 通道的事件不被采样(即使主通道采样, audit channel 全量保留).

### Implementation for User Story 3(用户故事三的实现)

- [x] T023 [P] [US3] 在 `src/observe/pipeline.rs` 的 `ObservabilityPipeline` 中实现背压检测逻辑: 在每个 subscriber 的缓冲区写入后检查占用率, 与 `BackpressureConfig` 中的 `warn_threshold_pct` 和 `critical_threshold_pct` 比较. 当软阈值触发时发射 `BackpressureAlert` 事件. 当硬阈值触发时根据策略采取行动.
- [x] T024 [P] [US3] 在 `src/observe/pipeline.rs` 中实现 `AlertAndBlock` 策略: 当订阅者缓冲区占用超过硬阈值时, 阻塞事件发射线程(await channel.send() 或等效阻塞原语), 直到缓冲区有空闲. 阻塞期间不丢失事件.
- [x] T025 [P] [US3] 在 `src/observe/pipeline.rs` 中实现 `SampleAndAudit` 策略: 当订阅者缓冲区占用超过硬阈值时, 按默认采样率(0.5)决定是否丢弃事件. 每次丢弃时记录 `BackpressureDegradation` 事件, 包含当前实际采样比例. 发射 `AuditRecorded` 事件到 audit 通道.
- [x] T026 [P] [US3] 在 `src/observe/pipeline.rs` 中创建独立的 audit 通道: 使用独立的 `tokio::sync::broadcast` 实例, 容量由 `BackpressureConfig.audit_channel_capacity`(默认 1024) 控制. audit 事件走此独立通道, 不受主通道背压采样的影响. audit 通道满时阻塞生产者(不采样 audit).
- [x] T027 [US3] 在 `src/spec/supervisor.rs` 的 `SupervisorSpec` 或等效配置结构中集成 `BackpressureConfig`. 在配置加载时(在 `src/config/loader.rs` 中)读取 `backpressure_strategy`, `backpressure_warn_threshold_pct`, `backpressure_critical_threshold_pct`, `backpressure_window_secs`, `audit_channel_capacity` 字段. 未配置时使用 `data-model.md` 中定义的默认值.
- [x] T028 [US3] 运行 `cargo test --test backpressure_strategy_test` 确认所有背压测试通过. 运行 `cargo test` 确认无回归.

**Checkpoint(检查点)**: 所有三个用户故事都可以独立工作. 慢订阅者场景已覆盖.

---

## Phase 6(阶段六): Polish & Cross-Cutting Concerns(收尾和横向事项)

**Purpose(目的)**: 完成影响多个用户故事的改进和验证.

- [x] T029 [P] 运行 `cargo fmt` 确保代码格式一致.
- [x] T030 [P] 运行 `cargo doc --no-deps --document-private-items` 确认新增模块和类型无文档警告.
- [x] T031 更新 `specs/005-1-failure-policy-reliability/contracts/pipeline-and-events.md` 的 `Alias mapping`(别名映射) 表, 补充本切片新增的 10 个 `What` 变体和 `BackpressureConfig`, `BackpressureStrategy`, `CorrelationHandle` 的代码锚点.
- [x] T032 运行 `cargo test` 全量测试, 确认 0 失败.

---

## Dependencies & Execution Order(依赖和执行顺序)

### Phase Dependencies(阶段依赖)

- **Setup(阶段一)**: 没有依赖, 可以立即开始.
- **Foundational(阶段二)**: 依赖 Setup(阶段一) 完成, 并阻塞所有用户故事.
- **User Stories(用户故事阶段)**: 全部依赖 Foundational(阶段二) 完成. 之后可以按人员情况并行, 也可以按 P1, P2, P3 顺序执行.
- **Polish(收尾阶段)**: 依赖所有选定用户故事完成.

### User Story Dependencies(用户故事依赖)

- **User Story 1(用户故事一, P1)**: Foundational(阶段二) 完成后可以开始. 不依赖其他故事. **MVP(最小可用产品) 建议范围**.
- **User Story 2(用户故事二, P1)**: Foundational(阶段二) 完成后可以开始. 可以集成 US1 的事件变体, 但 `CorrelationHandle` 和测试可以独立于 US1 实现和验证(使用模拟事件).
- **User Story 3(用户故事三, P2)**: Foundational(阶段二) 完成后可以开始. 背压检测逻辑可独立验证(使用模拟 subscriber), 不依赖 US1 或 US2.

### Within Each User Story(每个用户故事内部)

- 行为变化的测试必须先写, 并且必须在实现前失败.
- 先写模型/类型, 再写服务逻辑.
- 完成一个故事后, 再进入下一个优先级.

### Parallel Opportunities(并行机会)

- 所有标记 [P] 的 Setup(阶段一) 任务可以并行(T002, T003 与 T001 并行).
- 所有标记 [P] 的 Foundational(阶段二) 任务可以并行(T004, T005, T006, T008 互不冲突).
- Foundational(阶段二) 完成后, US1, US2, US3 可以由不同人员并行实施.
- 同一用户故事中标记 [P] 的测试和模型任务可以并行, 前提是修改不同文件.

---

## Parallel Example(并行示例): Phase 2 Foundational(阶段二基础任务)

```bash
# 并行: T004 修改 src/event/payload.rs, T005 修改 src/spec/supervisor.rs, T006 创建 src/event/correlation.rs
# 这三个任务修改不同文件, 可以并行

# 终端 1: T004
# 在 src/event/payload.rs 中追加 schema_id 和 config_version 字段

# 终端 2: T005
# 在 src/spec/supervisor.rs 中追加 BackpressureConfig 和 BackpressureStrategy

# 终端 3: T006
# 创建 src/event/correlation.rs 模块
```

## Parallel Example(并行示例): User Stories(用户故事阶段)

```bash
# US1, US2, US3 在 Foundational 完成后可以并行由不同人员实施

# 人员 A: US1 (Typed Events)
# 审计 control_loop.rs, pipeline.rs, shutdown/pipeline.rs
# 创建 tests/typed_event_coverage_test.rs

# 人员 B: US2 (Correlation Tracking)
# 实现 CorrelationHandle, 注入 tracing 和 metrics
# 创建 tests/correlation_tracking_test.rs

# 人员 C: US3 (Backpressure)
# 实现背压检测和两种策略
# 创建 tests/backpressure_strategy_test.rs
```
