# Tasks(任务): 生产级重启策略与分组隔离观测

**Input(输入)**: 设计文档来自 `specs/006-4-restart-policy-production/`
**Prerequisites(前置文档)**: plan.md(必需), spec.md(用户故事必需), data-model.md(实体定义), contracts/restart-budget-api.md(预算 API 契约), contracts/group-isolation-api.md(分组隔离 API 契约), research.md(技术决策), quickstart.md(阅读顺序)

**Tests(测试)**: 行为变化必须先有测试任务, 再有实现任务. 所有测试放在外部 `tests/` 目录.

**Organization(组织方式)**: 任务按用户故事分组: US1(fast failure doesn't cause storm), US2(group fault stays within boundary), US3(critical/optional bifurcation). 每个故事可独立实现和独立测试.

**Dependencies(依赖)**: 强依赖 `specs/005-1-failure-policy-reliability/`, `specs/005-2-work-role-defaults/`, `specs/006-3-lifecycle-shutdown-realism/`(ChildSlot 基础设施).

## Format(格式): `[ID] [P?] [Story] Description(描述)`

- **[P]**: 可以并行执行, 因为任务修改不同文件, 并且不依赖未完成任务.
- **[Story]**: 标记任务属于哪个用户故事, 例如 US1, US2, US3.
- 任务描述必须写出准确文件路径.
- 任务描述使用中文; 英文术语写成 `English(中文说明)`.
- 所有测试放在外部 `tests/` 目录, 不在 `src/` 模块文件中写测试.

---

## Phase 1(阶段一): Setup(共享基础设施)

**Purpose(目的)**: 创建新模块骨架和类型声明.

- [x] T001 在 `src/policy/` 下创建 `budget.rs`, `group.rs` 两个空模块骨架, 并在 `src/policy/mod.rs` 中追加 `pub mod budget; pub mod group;`.
- [x] T002 [P] 在 `src/observe/` 下创建 `fairness.rs` 空模块骨架, 并在 `src/observe/mod.rs` 中追加 `pub mod fairness;`.
- [x] T003 [P] 在外部 `tests/` 目录创建 `policy_budget_waveform_test.rs`, `policy_group_isolation_test.rs`, `policy_critical_optional_test.rs`, `policy_fairness_probe_test.rs` 四个空测试文件.
- [x] T004 运行 `cargo check` 确认零编译错误. 运行 `cargo fmt`.

---

## Phase 2(阶段二): Foundational(阻塞前置基础)

**Purpose(目的)**: 完成所有用户故事都需要的核心类型定义. 本阶段完成前, 任何用户故事实现都不能开始.

**Critical(关键要求)**: 类型定义必须与 `data-model.md` 冻结的一致.

- [x] T005 在 `src/policy/budget.rs` 中定义 `RestartBudgetConfig` 结构体和 `BudgetVerdict` 枚举, 按 `data-model.md` 和 `contracts/restart-budget-api.md`. 字段: `window: Duration`, `max_burst: u32`, `recovery_rate_per_sec: f64`. 所有字段带英文文档注释.
- [x] T006 [P] 在 `src/policy/group.rs` 中定义 `GroupDependencyEdge` 结构体, `PropagationPolicy` 枚举, `GroupIsolationPolicy` 结构体, 按 `data-model.md` 和 `contracts/group-isolation-api.md`. `GroupIsolationPolicy` 实现 `affected_by()` 方法.
- [x] T007 [P] 在 `src/policy/role_defaults.rs` 中定义 `SeverityClass` 枚举: `Critical`, `Optional`, `Standard`. 在 `EffectivePolicy` 结构体中新增 `severity: SeverityClass` 和 `group_name: Option<String>` 字段.
- [x] T008 [P] 在 `src/observe/fairness.rs` 中定义 `FairnessProbe` 结构体和 `StarvationAlert` 结构体, 按 `data-model.md`. `FairnessProbe` 实现 `record_opportunity()` 和 `check()` 方法.
- [x] T009 在 `src/event/payload.rs` 的 `What` 枚举中新增 `BudgetExhausted`, `GroupFuseTriggered`, `EscalationBifurcated` 三个事件变体, 按 `data-model.md` 字段定义.
- [x] T010 运行 `cargo check` 确认 Foundational(基础) 阶段所有新增类型编译无错. 执行 `cargo fmt`.
- [x] T047 [P] 在 `src/config/loader.rs` 或 `src/policy/budget.rs` 中添加 `RestartBudgetConfig` 字段约束校验: `max_burst` 超过 10_000 时产生配置告警, 接近 `u32::MAX` 时拒绝; `recovery_rate_per_sec` 低于 0.001 时产生配置告警. 按 `data-model.md` 补充的业务上下限.

**Checkpoint(检查点)**: `RestartBudgetConfig`, `BudgetVerdict`, `SeverityClass`, `GroupDependencyEdge`, `PropagationPolicy`, `GroupIsolationPolicy`, `FairnessProbe`, `StarvationAlert` 类型已就绪. 用户故事实现可以开始.

---

## Phase 3(阶段三): User Story 1(用户故事一) - 快速失败不致风暴 (Priority(优先级): P1) MVP(最小可用产品)

**Goal(目标)**: 子任务快速失败时, RestartBudgetTracker(重启预算跟踪器) 限流有效重启速率, BackoffJitter(退避抖动) 打散重启节拍, FairnessProbe(公平性探针) 检测调度饥饿.

**Independent Test(独立测试)**: 输入固定失败波形脚本, 统计每分钟有效重启尝试, 与配置预算曲线对照.

### Tests for User Story 1(用户故事一的测试)

> **NOTE(说明): 必须先写这些测试, 并确认它们在实现前失败.**

- [x] T011 [P] [US1] 在 `tests/policy_budget_waveform_test.rs` 中添加 `test_budget_limits_effective_restart_rate`: 模拟 10_000(一万次) 快速失败, 断言 `RestartBudgetTracker` 令牌耗尽后返回 `Exhausted`, 有效重启速率不超过配置上界 105%. 同时覆盖边界: 令牌刚好恢复 1 个时立即通过下次 `try_consume()`, 不等待额外评估周期.
- [x] T012 [P] [US1] 在 `tests/policy_budget_waveform_test.rs` 中添加 `test_budget_recovers_tokens_over_time`: 令牌耗尽后等待超过 `recovery_rate_per_sec` 的恢复时间, 断言令牌逐步恢复到上限.
- [x] T013 [P] [US1] 在 `tests/policy_fairness_probe_test.rs` 中添加 `test_fairness_probe_detects_starvation`: 连续 10 秒仅调度 1 个 child, 断言 `FairnessProbe::check()` 检测到其余 child 饥饿并返回 `StarvationAlert`.
- [x] T014 [P] [US1] 在 `tests/policy_fairness_probe_test.rs` 中添加 `test_fairness_probe_ok_when_all_scheduled`: 所有 child 均获得调度机会时, 断言 `check()` 返回 `None`.

### Implementation for User Story 1(用户故事一的实现)

- [x] T015 [US1] 在 `src/policy/budget.rs` 中实现 `RestartBudgetTracker` 结构体, 按 `contracts/restart-budget-api.md` 契约: `new()`, `try_consume()`, `current_tokens()`, `window_failures()`. 内部使用 `VecDeque<u128>` 滑动窗口 + `f64` 令牌桶.
- [x] T016 [US1] 在 `src/observe/fairness.rs` 中完整实现 `FairnessProbe::record_opportunity()` 和 `FairnessProbe::check()`. `check()` 返回 `Option<StarvationAlert>`.
- [x] T017 [US1] 在 `src/runtime/pipeline.rs` 的 `SupervisionPipeline` 的 `evaluate_budget` 阶段中注入 `RestartBudgetTracker` 引用. 在决定重启前调用 `try_consume()`. 若返回 `Exhausted`, 跳过重启并发射 `What::BudgetExhausted` 事件.
- [x] T018 [US1] 在 `src/runtime/control_loop.rs` 的 `RuntimeControlState` 结构体中新增 `budget_tracker: RestartBudgetTracker` 和 `fairness_probe: FairnessProbe` 字段. 在 `new()` 中正确初始化.
- [x] T019 [US1] 在 `src/runtime/control_loop.rs` 的控制循环主路径中: 每次成功处理一个事件后调用 `fairness_probe.record_opportunity(child_id)`. 每 `probe_interval_ns` 调用 `fairness_probe.check()`, 若检测到饥饿, 发射 `What::FairnessProbeStarvation` 事件.
- [x] T020 [US1] 在 `src/runtime/pipeline.rs` 中确保 `evaluate_budget` 阶段按 `budget → meltdown(熔断) → backoff(退避)` 顺序执行. 预算不足直接拒绝(不经过熔断与退避), 熔断后不计算退避.
- [x] T021 [US1] 运行 `cargo test --test policy_budget_waveform_test --test policy_fairness_probe_test` 确认 US1 全部 4 个测试通过. 运行 `cargo test` 确认无回归. ✅ 全量测试 0 失败
- [x] T048 [P] [US1] 在 `src/runtime/pipeline.rs` 的 `evaluate_budget` 阶段中: 当 `BudgetExhausted` 事件率超过 10 次/分钟时, 通过 `ObservabilityPipeline` 发射告警信号. 按 `spec.md` FR-001 补充的告警阈值.

**Checkpoint(检查点)**: 快速失败波形下, 重启速率被预算限流, 公平性探针检测调度饥饿. US1 可独立演示.

---

## Phase 4(阶段四): User Story 2(用户故事二) - 分组故障止步于组边界 (Priority(优先级): P1)

**Goal(目标)**: 任一 group(分组) 触发熔断后, 只有声明了 `GroupDependencyEdge(分组依赖边)` 的其他分组受影响. 未声明依赖的分组维持正常运行.

**Independent Test(独立测试)**: 双分组对照实验: group A 注入熔断条件, group B 的 uptime(在线时间) 比例不降.

### Tests for User Story 2(用户故事二的测试)

> **NOTE(说明): 必须先写这些测试, 并确认它们在实现前失败.**

- [x] T022 [P] [US2] 在 `tests/policy_group_isolation_test.rs` 中添加 `test_group_fuse_does_not_affect_unrelated_group`: group A 触发熔断, 断言 `GroupIsolationPolicy::affected_by("group_b", "group_a")` 在无依赖边声明时返回 `false`.
- [x] T023 [P] [US2] 在 `tests/policy_group_isolation_test.rs` 中添加 `test_dependency_edge_propagates_fuse`: 声明 `GroupDependencyEdge { from_group: "B", to_group: "A", propagation: Full }`, 断言 `affected_by("group_b", "group_a")` 返回 `true`.
- [x] T024 [P] [US2] 在 `tests/policy_group_isolation_test.rs` 中添加 `test_meltdown_tracker_group_counter_isolation` 和 `test_group_isolation_24h_sliding_window`: 前者断言 group A 熔断后 group B 计数器不受影响; 后者模拟 group A 持续熔断 24h 滑动窗口, 统计 group B 额外非计划停机时间, 断言不超过对照实验基线的 5%(对应 SC-002).

### Implementation for User Story 2(用户故事二的实现)

- [x] T025 [US2] 在 `src/policy/meltdown.rs` 的 `MeltdownTracker` 中新增 `group_counters: HashMap<String, GroupCounter>` 字段. `GroupCounter` 包含 `failures: VecDeque<Instant>`, `fuse_active: bool`.
- [x] T026 [US2] 在 `src/policy/meltdown.rs` 中实现 `track_group_failure()`, `group_fuse_active()`, `propagate_fuse()` 方法, 按 `contracts/group-isolation-api.md`.
- [x] T027 [US2] 在 `src/policy/group.rs` 的 `GroupIsolationPolicy` 中完整实现 `affected_by(&self, my_group: &str, failed_group: &str) -> bool`. 同一分组自身返回 `true`.
- [x] T028 [US2] 在 `src/runtime/pipeline.rs` 的 `evaluate_budget` 阶段中: 熔断触发时调用 `MeltdownTracker::propagate_fuse()`, 对每个受影响的分组发射 `What::GroupFuseTriggered` 事件, 并对该分组内所有 child 标记为不可重启.
- [x] T029 [US2] 在 `src/runtime/control_loop.rs` 中: 当 `RestartDecision::Quarantine` 由 group 级熔断触发时, 确保 `child_runtime_records` 中正确反映 `group_fuse_active` 状态.
- [x] T030 [US2] 运行 `cargo test --test policy_group_isolation_test` 确认 US2 全部 4 个测试通过(含 24h 滑动窗口). 运行 `cargo test` 确认 US1 测试无回归.
- [x] T049 [P] [US2] 在 `src/spec/supervisor.rs` 或 `src/config/loader.rs` 的配置加载阶段添加校验: `ChildSpec.group` 引用的分组名必须在 `SupervisorSpec.group_configs` 中存在; 不存在时拒绝启动并返回结构化错误(指出未找到的分组名). 按 `spec.md` FR-002 和 `data-model.md` Relationships.

**Checkpoint(检查点)**: 分组故障隔离生效, 依赖边传播正确. US1 和 US2 均可独立测试.

---

## Phase 5(阶段五): User Story 3(用户故事三) - critical 与 optional 分叉可观测 (Priority(优先级): P2)

**Goal(目标)**: Critical(关键) 与 Optional(可选) 子任务的失败处置路径在 typed event(类型化事件) 与 metrics(指标) 双通道完全区分.

**Independent Test(独立测试)**: 对两条路径分别抓取最新 100 条事件记录与 metrics 标签集合, 核对字段基数差异.

### Tests for User Story 3(用户故事三的测试)

> **NOTE(说明): 必须先写这些测试, 并确认它们在实现前失败.**

- [x] T031 [P] [US3] 在 `tests/policy_critical_optional_test.rs` 中添加 `test_critical_failure_escalation_produces_bifurcated_event`: 注入 `SeverityClass::Critical` 子任务失败, 断言 `What::EscalationBifurcated` 事件中 `severity == Critical`.
- [x] T032 [P] [US3] 在 `tests/policy_critical_optional_test.rs` 中添加 `test_optional_failure_no_escalation`: 注入 `SeverityClass::Optional` 子任务失败, 断言 `What::EscalationBifurcated` 事件中 `severity == Optional`, 且不触发升级路径.
- [x] T033 [P] [US3] 在 `tests/policy_critical_optional_test.rs` 中添加 `test_correlation_id_links_budget_and_escalation_events` 和 `test_event_metrics_consistency_rate`: 前者断言同一次故障链路的 `BudgetExhausted` 和 `EscalationBifurcated` 事件共享同一个 `CorrelationId(关联标识)`; 后者抓取 100 条 typed event 与对应 metrics 标签, 逐条比对 `SupervisorDecision` 键字段一致率, 断言不低于 98%(对应 SC-003).

### Implementation for User Story 3(用户故事三的实现)

- [x] T034 [US3] 在 `src/policy/role_defaults.rs` 中: `WorkRole` 添加默认 `SeverityClass` 映射: `Service → Critical`, `Supervisor → Critical`, `Worker → Standard`, `Job → Optional`, `Sidecar → Standard`. `ChildSpec` 中的显式 `severity` 字段覆盖角色默认值.
- [x] T035 [US3] 在 `src/runtime/pipeline.rs` 中: evaluate_budget 阶段完成后, 根据 `EffectivePolicy.severity` 决定进一步动作. `Critical` → 发射 `EscalationBifurcated` 并升级. `Optional` → 发射 `EscalationBifurcated` 并降噪.
- [x] T036 [US3] 在 `src/event/payload.rs` 中: 确保 `EscalationBifurcated` 事件变体包含 `severity: SeverityClass`, `budget_verdict: Option<BudgetVerdict>`, `fuse_outcome: Option<MeltdownOutcome>`. `Option` 字段在跳过时使用 `None`, 不引入 `NotEvaluated` 变体. 按 `data-model.md` EscalationBifurcated 诊断键表.
- [x] T037 [US3] 在 `src/runtime/control_loop.rs` 中: 生成 `CorrelationId(关联标识)` 使用 UUID v4 算法, 确保多个 child 同时触发故障时各自获得独立 ID; 在 budget → meltdown → escalation 事件链路中传递, 确保同一故障链路的所有事件共享同一 CorrelationId. 按 `spec.md` Edge Cases.
- [x] T038 [US3] 运行 `cargo test --test policy_critical_optional_test` 确认 US3 全部 4 个测试通过(含事件/指标一致率验证). 运行 `cargo test` 确认 US1, US2 测试无回归.
- [x] T050 [P] [US3] 在 `tests/policy_critical_optional_test.rs` 中添加 `test_correlation_id_uuid_v4_uniqueness`: 模拟 1000 个 child 同时触发故障, 断言所有 CorrelationId 唯一.

**Checkpoint(检查点)**: critical/optional 分叉路径在事件和指标中完全可区分. 所有 3 个用户故事可独立测试.

---

## Phase 6(最终阶段): Polish & Cross-Cutting Concerns(收尾和横向事项)

**Purpose(目的)**: 完成影响多个用户故事的改进和代码清理.

- [x] T039 [P] 在 `src/spec/supervisor.rs` 中新增 `GroupConfig` 结构体: `name: String`, `children: Vec<ChildId>`, `budget: Option<RestartBudgetConfig>`. 新增 `group_dependencies: Vec<GroupDependencyEdge>`, `severity_defaults: HashMap<WorkRole, SeverityClass>`.
- [x] T040 [P] 在 `src/spec/child.rs` 的 `ChildSpec` 中新增 `severity: Option<SeverityClass>` 和 `group: Option<String>` 字段(可选, 覆盖角色默认).
- [x] T041 [P] 为 `src/policy/budget.rs`, `src/policy/group.rs`, `src/observe/fairness.rs` 补齐模块文档注释(符合 Rust 源码英文注释规范 `//!`).
- [x] T042 [P] 在 `src/observe/pipeline.rs` 中: 为 `BudgetExhausted`, `GroupFuseTriggered`, `EscalationBifurcated` 事件类型添加观测流水线处理, 转为可审计的 `PipelineStageDiagnostic`.
- [x] T043 运行 `cargo test` 全量测试套件, 确保所有现有测试无回归.
- [x] T044 运行 `cargo clippy --all-targets -- -D warnings` 确认零 clippy 警告.
- [x] T045 运行 `cargo fmt --all` 确保代码格式一致.
- [x] T046 运行 `cargo doc --no-deps --document-private-items` 确认新模块无文档警告.
- [x] T051 [P] 在 `src/spec/supervisor.rs` 的 `GroupConfig` 中: `budget` 为 `None` 时继承 `SupervisorSpec` 级默认 `RestartBudgetConfig`, supervisor 级也未配置时使用内置安全默认值 (`window=60s, max_burst=10, recovery_rate_per_sec=0.5, max_tokens=10`). 按 `data-model.md` Relationships.
- [x] T052 [P] 在 `src/policy/budget.rs` 的 `RestartBudgetTracker::new()` 中: 当 `RestartBudgetConfig` 字段缺失时使用内置安全默认值填充, 确保旧版配置升级时无需手动添加 budget 字段. 按 `spec.md` Edge Cases.
- [x] T053 [P] 在 `src/observe/pipeline.rs` 中: 扩展 `PipelineStageDiagnostic` 新增 `evaluated: bool` 字段(区分"阶段已执行"与"阶段被跳过")和 `skip_reason: Option<String>`(跳过时注明原因). 按 `spec.md` Diagnostics.
- [x] T054 在 CI 配置 (`.github/workflows/nightly-gates.yml`) 中添加 SC-003 持续验证步骤: 定时抽检最近 24 小时内 event/metrics 样本, typed event 与 metrics 一致率低于 98% 时阻塞发布.

---

## Dependencies & Execution Order(依赖和执行顺序)

### Phase Dependencies(阶段依赖)

```text
Phase 1 (Setup)
  └─ Phase 2 (Foundational)
       ├─ Phase 3 (US1) ──┐
       ├─ Phase 4 (US2) ──┤  可并行(不同模块)
       ├─ Phase 5 (US3) ──┘
       └─ Phase 6 (Polish)  依赖所有 US 完成
```

### User Story Dependencies(用户故事依赖)

- **US1(快速失败不致风暴, P1)**: Foundational 完成后可开始. 不依赖 US2 或 US3. **是 MVP(最小可用产品)**.
- **US2(分组故障止步于边界, P1)**: Foundational 完成后可开始. 不依赖 US1 或 US3. 可与 US1 并行.
- **US3(关键可选分叉可观测, P2)**: Foundational 完成后可开始. 依赖 US1 的 `BudgetExhausted` 事件和 US2 的 `GroupFuseTriggered` 事件存在, 但不需要它们的完整集成.

### Parallel Opportunities(并行机会)

- Phase 2 中 T005, T006, T007, T008, T009 可以并行(不同文件).
- Phase 3(US1), Phase 4(US2), Phase 5(US3) 的核心实现可并行推进: US1 改 `budget.rs` + `fairness.rs`, US2 改 `meltdown.rs` + `group.rs`, US3 改 `role_defaults.rs` + `payload.rs`.
- 同一用户故事中标记 [P] 的测试可以并行(不同测试函数, 同文件).

---

## Parallel Example(并行示例): US1, US2, US3 同时推进

```bash
# 开发者 A: US1 实现
Task: "T015 实现 RestartBudgetTracker in src/policy/budget.rs"
Task: "T016 实现 FairnessProbe in src/observe/fairness.rs"

# 开发者 B: US2 实现(同时进行)
Task: "T025 扩展 MeltdownTracker group_counters"
Task: "T026 实现 track_group_failure, propagate_fuse"

# 开发者 C: US3 实现(同时进行)
Task: "T034 添加 SeverityClass 默认映射 in role_defaults.rs"
Task: "T036 新增 EscalationBifurcated 事件变体"
```

---

## Implementation Strategy(实现策略)

### MVP First(先做最小可用产品)

1. Phase 1(Setup) + Phase 2(Foundational) → 类型基础就绪.
2. Phase 3(US1) → 重启预算限流 + 公平性探测 → MVP.
3. 停止并验证: `cargo test --test policy_budget_waveform_test`.
4. 此时即可演示: 快速失败波形被预算压住.

### Incremental Delivery(增量交付)

1. Setup + Foundational → 类型基础就绪.
2. US1 → 预算限流 + 公平性 (MVP).
3. US2 → 分组隔离.
4. US3 → 分叉可观测.
5. Polish → 配置集成, 清理, 回归测试.

---

## Notes(说明)

- [P] 表示任务修改不同文件, 并且没有依赖.
- [Story] 标签把任务映射到具体用户故事, 方便追踪.
- 每个用户故事都必须能独立完成和独立测试.
- 实现前必须确认测试失败.
- 每个任务或逻辑组完成后可以提交.
- 可以在任何检查点停止并验证该故事.
- 所有 Rust 源码注释(`//`, `///`, `//!`) 必须使用英文.
- 所有规格, 计划, 任务文档正文必须使用中文, 英文术语写为 `English(中文说明)`.
