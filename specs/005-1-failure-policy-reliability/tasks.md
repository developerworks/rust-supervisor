# Tasks(任务): 失败策略流水线与生产级退避

**Input(输入)**: 设计文档来自 `specs/005-1-failure-policy-reliability/`
**Prerequisites(前置文档)**: plan.md, spec.md, research.md, data-model.md, contracts/pipeline-and-events.md, quickstart.md

**Tests(测试)**: 行为变化必须先有测试任务,再有实现任务.纯文档或纯模板变更必须说明运行时测试为什么不适用.

**Organization(组织方式)**: 任务按三个用户故事分组,确保每个故事都能独立实现和独立测试.

## Format(格式): `[ID] [P?] [Story] Description(描述)`

- **[P]**: 可以并行执行,因为任务修改不同文件,并且不依赖未完成任务.
- **[Story]**: 标记任务属于哪个用户故事,例如 US1,US2,US3.
- 任务描述必须写出准确文件路径.
- 任务描述必须使用中文;英文术语必须写成 `English(中文说明)`.
- Rust(编程语言) 项目中, 所有单元测试,契约测试和集成测试都必须放在外部 `tests/` 目录, 不得把测试代码写入 `src/` 模块文件.
- 并行任务必须修改不同文件;如果两个任务会修改同一个文件, 不得同时标记 `[P]`.

---

## Phase 1(阶段一): Setup(共享基础设施)

**Purpose(目的)**: 确认项目结构和验证命令可用.

- [X] T001 确认 `cargo test` 和 `cargo fmt` 在仓库根目录可正常执行.
- [X] T002 阅读 `src/runtime/control_loop.rs`, `src/policy/meltdown.rs`, `src/tree/order.rs`, `src/event/payload.rs` 了解现有结构.

---

## Phase 2(阶段二): Foundational(阻塞前置基础)

**Purpose(目的)**: 完成任何用户故事开始前都必须存在的核心基础设施.

**Critical(关键要求)**: 本阶段完成前, 任何用户故事实现都不能开始.

- [X] T003 [P] 在 `src/event/payload.rs` 中新增 `TypedSupervisionEvent`(类型化监督事件) 增量字段: `scopes_triggered`, `lead_scope`, `effective_protective_action`, `cold_start_reason`, `hot_loop_reason`, `throttle_gate_owner`.
- [X] T003b [P] **必须新建** `src/policy/failure_window.rs` 模块实现 `FailureWindow`(失败窗口) 的滑动累计逻辑, 支持按时间滑动或按次数滑动两种模式, 并将窗口内失败样本累计结果写入 `MeltdownScopeState.quota_counters` 供 `evaluate budget`(评估预算) 阶段读取. **不得修改 `src/policy/meltdown.rs`**, 仅在 `src/policy/mod.rs` 或等价入口中导出新模块.
- [X] T004 在 `src/policy/meltdown.rs` 中扩展 `MeltdownTracker`(熔断跟踪器), 增加 `group`(分组) 作用域的计数桶和阈值判定逻辑. **本任务依赖 T003b 完成后执行**, 因为需在 `meltdown.rs` 中导入 T003b 创建的 `failure_window` 模块.
- [X] T005 在 `src/observe/pipeline.rs` 或新建模块中定义六阶段流水线的诊断转发接口, 确保每阶段都能输出可对账的结构化事件.
- [X] T006 在 `src/tree/order.rs` 中仅确认 `restart_execution_plan`(重启执行计划) 的 `restart_limit`(重启次数限制) 和 `escalation_policy`(升级策略) 字段定义存在且可访问, 不涉及业务逻辑消费.
- [X] T007 在 `src/policy/decision.rs` 或相关模块中定义 `protection restrictiveness ladder`(保护从严档位序) 枚举, 包含六个档位: `restart_allowed`, `restart_queued`, `restart_denied`, `supervision_paused`, `escalated`, `supervised_stop`.

**Checkpoint(检查点)**: 基础类型和数据结构已就绪, 用户故事实现可以开始.

---

## Phase 3(阶段三): User Story 1(用户故事一) - 失败路径进入单一可查流水线 (Priority(优先级): P1) MVP(最小可用产品)

**Goal(目标)**: 确保所有受监督单元的运行结束情形都经过统一的六阶段流水线处理, 且 `restart_execution_plan` 中的限额和升级策略在评估预算阶段生效并写入事件.

**Independent Test(独立测试)**: 使用固定失败样本集, 从外部验证每条样本的监督结论都能对应到六阶段的正确位置, 且每阶段至少有一种可订阅或导出的诊断信息.

### Tests for User Story 1(用户故事一的测试)

> **NOTE(说明): 必须先写这些测试,并确认它们在实现前失败.**

- [X] T008 [P] [US1] 在 `tests/supervisor_pipeline_order.rs` 中添加验收测试, 验证非零退出码触发的失败能按顺序走完六阶段流水线, 且每阶段都有结构化事件输出; 同时验证 `success`(成功) 退出码也走完六阶段并在事件流中留下可对账记录点.
- [X] T009 [P] [US1] 在 `tests/supervisor_restart_limit_usage.rs` 中添加验收测试, 验证 `restart_execution_plan` 携带 `restart_limit` 时, `evaluate budget`(评估预算) 阶段能读取该字段并影响最终处置结论.
- [X] T009b [P] [US1] 在 `tests/supervisor_cancel_stop_priority.rs` 中添加验收测试, 验证当 `external_cancel`(外部取消) 或 `manual_stop`(人工停止) 与自动重启竞争执行权时, `execute action`(执行动作) 不得将已标明必须结束的任务再次自动拉起.

### Implementation for User Story 1(用户故事一的实现)

- [X] T010 [US1] 在 `src/runtime/control_loop.rs` 中重构进程结束处理逻辑, 确保所有退出情形(成功, 非零退出, 崩溃, 超时, 外部取消, 人工停止)都进入 `classify exit`(分类退出) 阶段.
- [X] T011 [US1] 在 `src/runtime/control_loop.rs` 或抽取的新模块中实现六阶段流水线的编排逻辑: `classify exit` → `record failure window` → `evaluate budget` → `decide action` → `emit typed event` → `execute action`.
- [X] T012 [US1] 在 `src/policy/decision.rs` 或相关模块中实现 `evaluate budget`(评估预算) 阶段的业务逻辑: 消费 `restart_execution_plan` 的 `restart_limit` 和 `escalation_policy` 字段(字段存在性由 T006 确认), 结合熔断判定结果产出决策输出并写入事件载荷.
- [X] T013 [US1] 在 `src/observe/pipeline.rs` 中实现每阶段的结构化事件输出, 确保 `TypedSupervisionEvent` 包含 `pipeline_stage` 标识和该阶段的诊断字段.
- [X] T014 [US1] 在 `src/runtime/control_loop.rs` 中确保 `execute action`(执行动作) 阶段不会与前几阶段的禁止重启或固定处置结论冲突.

**Checkpoint(检查点)**: 用户故事一已经完整可用, 并且可以独立测试. 审查者能从事件流中核对六阶段顺序和限额字段的生效情况.

---

## Phase 4(阶段四): User Story 2(用户故事二) - 熔断压力按作用域隔离 (Priority(优先级): P2)

**Goal(目标)**: 确保 `MeltdownTracker`(熔断跟踪器) 按 `child`(子任务), `group`(分组), `supervisor`(监督器) 三层独立计数, 多层判定合并时取最严档位, 并在事件中写明主导归因作用域.

**Independent Test(独立测试)**: 向特定 `group`(分组) 或 `child`(子任务) 注入高频失败, 验证其他分组和子任务仍能独立完成熔断判断并正常重启.

### Tests for User Story 2(用户故事二的测试)

- [X] T015 [P] [US2] 在 `tests/supervisor_meltdown_group_isolation.rs` 中添加验收测试, 验证仅某分组内连续失败时, 其他分组不受影响.
- [X] T016 [P] [US2] 在 `tests/supervisor_meltdown_lead_scope.rs` 中添加验收测试, 验证三层同时触发熔断时, 事件中的 `lead_scope` 按 `child` → `group` → `supervisor` 次序取值.

### Implementation for User Story 2(用户故事二的实现)

- [X] T017 [US2] 在 `src/policy/meltdown.rs` 中实现三层独立的计数桶: `child` 级绑定 `ChildId`, `group` 级绑定 `restart_execution_plan` 的 `group` 字段, `supervisor` 级绑定监督实例边界. **本任务与 T004 协同**: T004 负责 `group` 作用域的扩展入口,T017 负责三层完整桶结构的最终实现.
- [X] T018 [US2] 在 `src/policy/meltdown.rs` 中实现每层的 `local verdict`(局部判定) 计算, 映射到 `protection restrictiveness ladder`(保护从严档位序).
- [X] T019 [US2] 在 `src/policy/meltdown.rs` 中抽取独立函数 `merge_meltdown_verdicts`, 实现多层判定合并逻辑: 接收三层 `local verdict`(局部判定), 在 `protection restrictiveness ladder`(保护从严档位序) 上取最严一档作为 `effective meltdown verdict`(有效熔断判定), 并返回平局判定后的 `lead_scope`. 该函数须有独立单元测试.
- [X] T020 [US2] 在 `src/event/payload.rs` 的事件输出中填充 `scopes_triggered`(已触发作用域列表) 和 `lead_scope`(主导归因作用域) 字段, 符合平局判定规则.
- [X] T021 [US2] 在 `src/runtime/control_loop.rs` 的 `evaluate budget` 阶段调用三层熔断判定, 并将结果传递给 `decide action` 阶段.

**Checkpoint(检查点)**: 用户故事二已经完整可用, 局部故障不会耗尽全局重启配额, 事件中能追溯熔断归因.

---

## Phase 5(阶段五): User Story 3(用户故事三) - 生产级退避与并发重启闸门 (Priority(优先级): P3)

**Goal(目标)**: 将 `BackoffPolicy`(退避策略) 升级为支持全抖动, 去相关抖动, 并发重启闸门, 冷启动预算和热循环检测的生产级策略, 并确保测试结果可重复.

**Independent Test(独立测试)**: 在可控时钟和随机种子下, 对比全抖动和去相关抖动相对于固定抖动的等待时长分散程度. 使用并发失败样本验证热循环检测和冷启动预算的限速行为.

### Tests for User Story 3(用户故事三的测试)

- [X] T022 [P] [US3] 在 `tests/supervisor_backoff_jitter_distribution.rs` 中添加验收测试, 固定 RNG seed, 验证全抖动或去相关抖动的等待间隔比固定抖动更分散.
- [X] T023 [P] [US3] 在 `tests/supervisor_concurrent_restart_throttle.rs` 中添加验收测试, 验证同一时段超出并发闸门上限的失败进入排队或拒绝档位, 且事件注明闸门归属; **必须包含原子性测试**: 使用至少 10 个同时触发的并发失败样本, 确认超出上限的任务全部进入保护档位, 无漏网之鱼.
- [X] T024 [P] [US3] 在 `tests/supervisor_cold_start_and_hot_loop.rs` 中添加验收测试, 验证冷启动预算耗尽和热循环检测触发时的保护处置符合从严档位序.

### Implementation for User Story 3(用户故事三的实现)

- [X] T025 [US3] 在 `src/policy/backoff.rs` 中实现 `full jitter`(全抖动) 算法: 若该文件不存在则先创建模块, 然后在零到策略上限间均匀随机抽样.
- [X] T026 [US3] 在 `src/policy/backoff.rs` 中实现 `decorrelated jitter`(去相关抖动) 算法: 在依赖初始基数和上一轮等待长度的区间内随机取值.
- [X] T027 [US3] 在 `src/runtime/concurrent_gate.rs` 中实现实例全局并发重启闸门计数, 确保不与进程内其他监督器实例共享. **闸门计数器必须在重启启动时递减**(即获得闸门许可后立即释放配额), 而非等待重启完成; 若重启启动前监督器崩溃, 闸门配额由超时机制或垃圾回收释放.
- [X] T028 [US3] 在 `src/runtime/concurrent_gate.rs` 中实现可选的分组级并发闸门计数, 未启用时回落到实例全局闸门. **当分组闸门与全局闸门冲突时, 取更严档位**(即任一闸门超限即触发保护).
- [X] T029 [US3] 在 `src/policy/backoff.rs` 中实现 `cold start budget`(冷启动预算) 逻辑: 绑定监督实例启动后的时间窗或重启次数配额, 耗尽时收紧保护档位.
- [X] T030 [US3] 在 `src/policy/backoff.rs` 中实现 `hot loop detection`(热循环检测) 逻辑: 在滑动时间窗内检测崩溃后短时间再次拉起, 触发时给出区别于重启次数超限的保护处置.
- [X] T031 [US3] 在 `src/runtime/pipeline.rs` 的事件输出中填充 `cold_start_reason`, `hot_loop_reason`, `throttle_gate_owner` 字段, 写明触发原因和闸门归属.
- [X] T032 [US3] 在 `src/test_support/factory.rs` 中注入可控时钟(`tokio` `pause`/`advance`) 和固定 RNG seed, 确保退避策略的测试结果可重复.

**Checkpoint(检查点)**: 用户故事三已经完整可用, 生产级退避策略能减轻雷群效应, 并发闸门防止瞬时资源竞争.

---

## Phase 6(最终阶段): Polish & Cross-Cutting Concerns(收尾和横向事项)

**Purpose(目的)**: 完成影响多个用户故事的改进.

- [X] T033 [P] 在 `tests/supervisor_pipeline_full_integration.rs` 中添加端到端集成测试, 覆盖六阶段流水线, 三层熔断和退避策略的完整路径; **必须包含交叉场景**: (1) 多层熔断同时触发且并列最严时 `lead_scope` 平局判定, (2) 并发闸门超限与冷启动预算耗尽同时发生时的保护档位合并.
- [X] T034 更新 `specs/005-1-failure-policy-reliability/quickstart.md` 中的代码阅读顺序和验收步骤, 反映实际实现的文件路径.
- [X] T035 清理 `src/runtime/control_loop.rs` 中的临时诊断代码, 确保所有分支都有结构化事件输出而非仅字符串广播.
- [X] T036 运行 `cargo fmt` 格式化全部源码.
- [X] T037 运行 `cargo test` 确认所有测试通过, 包括新增的验收测试和现有回归测试.
- [X] T038 确认 `src/` 中的 Rust 注释均为英文, 规格文档保持中文且术语格式为 `English(中文说明)`.

---

## Dependencies & Execution Order(依赖和执行顺序)

### Phase Dependencies(阶段依赖)

- **Setup(阶段一)**: 没有依赖,可以立即开始.
- **Foundational(阶段二)**: 依赖 Setup(阶段一) 完成,并阻塞所有用户故事.
- **User Stories(用户故事阶段)**: 全部依赖 Foundational(阶段二) 完成.之后可以按人员情况并行,也可以按 P1,P2,P3 顺序执行.
- **Polish(收尾阶段)**: 依赖所有选定用户故事完成.

### User Story Dependencies(用户故事依赖)

- **User Story 1(用户故事一,P1)**: Foundational(阶段二) 完成后可以开始,不依赖其他故事.
- **User Story 2(用户故事二,P2)**: Foundational(阶段二) 完成后可以开始,依赖 US1 的流水线框架,但熔断逻辑独立.
- **User Story 3(用户故事三,P3)**: Foundational(阶段二) 完成后可以开始,依赖 US1 的流水线框架,但退避策略独立.

### Within Each User Story(每个用户故事内部)

- 行为变化的测试必须先写,并且必须在实现前失败.
- 先写模型,再写服务.
- 先写服务,再写端点.
- 先写核心实现,再写集成.
- 完成一个故事后,再进入下一个优先级.

### Parallel Opportunities(并行机会)

- 所有标记 [P] 的 Setup(阶段一) 任务可以并行.
- 所有标记 [P] 的 Foundational(阶段二) 任务可以在阶段内部并行.
- Foundational(阶段二) 完成后,不同用户故事可以由不同人员并行.
- 同一用户故事中标记 [P] 的测试可以并行, 因为它们必须写入不同 `tests/` 文件.
- 同一用户故事中标记 [P] 的模型任务可以并行, 前提是它们修改不同 `src/` 文件.

---

## Parallel Example(并行示例): User Story 1(用户故事一)

```bash
# 同时启动用户故事一的测试任务:
Task(任务): "在 tests/supervisor_pipeline_order.rs 中添加验收测试"
Task(任务): "在 tests/supervisor_restart_limit_usage.rs 中添加验收测试"

# 用户故事一的实现任务需按顺序执行,因为都修改 control_loop.rs
```

---

## Parallel Example(并行示例): User Story 2(用户故事二)

```bash
# 同时启动用户故事二的测试任务:
Task(任务): "在 tests/supervisor_meltdown_group_isolation.rs 中添加验收测试"
Task(任务): "在 tests/supervisor_meltdown_lead_scope.rs 中添加验收测试"

# 同时启动用户故事二的部分实现任务(修改不同文件):
Task(任务): "在 src/policy/meltdown.rs 中实现三层计数桶"
Task(任务): "在 src/event/payload.rs 中填充 scopes_triggered 和 lead_scope 字段"
```

---

## Parallel Example(并行示例): User Story 3(用户故事三)

```bash
# 同时启动用户故事三的测试任务:
Task(任务): "在 tests/supervisor_backoff_jitter_distribution.rs 中添加验收测试"
Task(任务): "在 tests/supervisor_concurrent_restart_throttle.rs 中添加验收测试"
Task(任务): "在 tests/supervisor_cold_start_and_hot_loop.rs 中添加验收测试"

# 同时启动用户故事三的部分实现任务(修改不同文件):
Task(任务): "在 src/policy/backoff.rs 中实现全抖动算法"
Task(任务): "在 src/policy/backoff.rs 中实现去相关抖动算法"
Task(任务): "在 src/runtime/control_loop.rs 中实现并发闸门计数"
```

---

## Implementation Strategy(实现策略)

### MVP First(先做最小可用产品)

1. 完成 Phase 1(阶段一): Setup(初始化).
2. 完成 Phase 2(阶段二): Foundational(基础),该阶段会阻塞所有故事.
3. 完成 Phase 3(阶段三): User Story 1(用户故事一).
4. 停止并验证 User Story 1(用户故事一).
5. 在可用时进行演示或交付.

### Incremental Delivery(增量交付)

1. 完成 Setup(初始化) 和 Foundational(基础).
2. 增加 User Story 1(用户故事一),独立测试后交付 MVP(最小可用产品).
3. 增加 User Story 2(用户故事二),独立测试后交付.
4. 增加 User Story 3(用户故事三),独立测试后交付.
5. 每个故事都必须增加价值,并且不得破坏已经完成的故事.

### Parallel Team Strategy(并行团队策略)

1. 团队先一起完成 Setup(初始化) 和 Foundational(基础).
2. Foundational(基础) 完成后,开发者可以按故事分工.
3. 每个故事必须独立完成并集成.

---

## Notes(说明)

- [P] 表示任务修改不同文件,并且没有依赖.
- [Story] 标签把任务映射到具体用户故事,方便追踪.
- 每个用户故事都必须能独立完成和独立测试.
- 实现前必须确认测试失败.
- 每个任务或逻辑组完成后可以提交.
- 可以在任何检查点停止并验证该故事.
- 避免模糊任务,同文件冲突,以及破坏故事独立性的跨故事依赖.

## Task Summary(任务汇总)

- **Total tasks(总任务数)**: 41
- **Phase 1(阶段一)**: 2 tasks
- **Phase 2(阶段二)**: 6 tasks
- **Phase 3(阶段三, US1)**: 9 tasks (3 tests + 6 implementation)
- **Phase 4(阶段四, US2)**: 7 tasks (2 tests + 5 implementation)
- **Phase 5(阶段五, US3)**: 11 tasks (3 tests + 8 implementation)
- **Phase 6(最终阶段)**: 6 tasks

### Independent Test Criteria(独立测试标准)

- **US1**: 固定失败样本集触发六阶段流水线, 每阶段有结构化事件输出, `restart_limit` 和 `escalation_policy` 在评估预算阶段生效.
- **US2**: 向特定分组或子任务注入高频失败, 其他分组和子任务仍能独立重启, 事件中 `lead_scope` 符合平局判定规则.
- **US3**: 固定 RNG seed 下全抖动或去相关抖动的等待间隔比固定抖动分散至少三成, 并发闸门超限部分进入排队或拒绝档位, 冷启动和热循环触发时保护处置符合从严档位序.

### Suggested MVP Scope(建议的 MVP 范围)

User Story 1(用户故事一) 构成 MVP: 统一六阶段流水线, `restart_execution_plan` 字段生效, 结构化事件可追溯. 这是后续熔断隔离和退避策略的基础.
