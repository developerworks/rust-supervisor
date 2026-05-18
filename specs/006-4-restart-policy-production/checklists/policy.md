# Policy Requirements Quality Checklist(策略需求质量检查清单)

**Purpose(目的)**: 验证 `006-4-restart-policy-production` 功能规格中需求的质量, 完整性, 清晰度与可度量性. 这是对需求本身的"单元测试", 不是对实现的验证.

**Created(创建日期)**: 2026-05-18
**Scope(范围)**: US1(预算限流) + US2(分组隔离) + US3(分叉可观测), 全部 3 个用户故事
**Depth(深度)**: Strict(严格 release gate)
**Gates(关口)**: 预算精度, 分组隔离正确性, 事件/指标一致性 — 全部三项

---

## Requirement Completeness(需求完整性)

- [x] CHK001 — 重启预算令牌桶的"恢复速率"(`recovery_rate_per_sec`) 是否在规格中明确了最小/最大允许值范围? [Gap, Spec §FR-001] ✅ data-model.md: 0.0 < recovery_rate_per_sec <= 1000.0
- [x] CHK002 — 预算耗尽的降级行为是否写明了"等待恢复后自动重试"还是"需人工干预"? [Gap, Spec §FR-001] ✅ spec.md FR-001: 等待 retry_after_ns 到期自动重试, 不需人工干预
- [x] CHK003 — 公平性探针(`FairnessProbe`) 的饥饿检测窗口(规格中提到"任意连续 10 秒窗口")是否写明了具体阈值? [Clarity, Spec §FR-001] ✅ data-model.md 新增 `min_ops_per_window: u64` 字段(默认 1)
- [x] CHK004 — 分组故障时, 受影响分组内已经 running(运行中) 的 child(子任务) 是否明确写明了处理方式: 继续运行, cancel(取消), 还是仅阻止新重启? [Gap, Spec §FR-002] ✅ spec.md FR-002: 已运行 child 继续运行不受影响, 仅阻止新重启
- [x] CHK005 — `EscalationBifurcated` 事件的 "metrics(指标) 标签集合" 是否以字段表格形式写明, 还是只写了"至少多出 3 个互不混淆的诊断键"这个数量约束? [Clarity, Spec §FR-003] ✅ data-model.md 新增 6 键诊断标签表

## Requirement Clarity(需求清晰度)

- [x] CHK006 — "effective restart attempts per minute(每分钟有效重启尝试) 不得超过文档给出曲线上界的 105%" — 这里的"文档"是指 YAML 配置文件, 代码常量, 还是独立的设计文档? [Ambiguity, Spec §SC-001] ✅ contracts/restart-budget-api.md 已提供预算曲线计算公式与示例
- [x] CHK007 — `PropagationPolicy` 枚举中 `EscalateOnly(仅升级)` 与 `Full(完全传播)` 两种传播级别对受影响分组内 child(子任务) 的可观察行为差异是否写明? [Ambiguity, Spec §FR-002] ✅ data-model.md 已补充详细注释: EscalateOnly不影响child调度, Full全组child标记不可重启
- [x] CHK008 — "预算计数快照"(`RestartBudgetSnapshot`) 在 typed event(类型化事件) 载荷中的字段名和类型是否已经冻结为契约? [Clarity, Spec §Key Entities] ✅ data-model.md 新增 RestartBudgetSnapshot 实体(5字段)并归入 Key Entities
- [x] CHK009 — `SeverityClass` 中 `Standard(默认)` 与 `Optional(可选)` 在 failure behavior(失败行为) 上的区别是否写明? [Ambiguity, Spec §FR-003] ✅ spec.md FR-003: Critical升级, Optional降噪, Standard按WorkRole默认

## Requirement Consistency(需求一致性)

- [x] CHK010 — 规格中 FR-001 要求 budget, meltdown, backoff 三者串入"同一评估管线". data-model.md 和 contracts/ 中的评估顺序 (budget → backoff → meltdown) 是否与 spec.md 的字面顺序一致? [Consistency, Spec §FR-001] ✅ 已统一为 budget → meltdown → backoff, spec/plan/tasks三文档一致
- [x] CHK011 — US2 验收场景中 "HealthyBaseline(健康基线) 计数不降" 与 SC-002 中 "B 侧额外非计划停机时间 ≤5%" 是否使用了同一个度量窗口 (24h 滑动窗口)? [Consistency, Spec §US2 vs SC-002] ✅ HealthyBaseline已移除, US2验收场景改用GroupCounter, T024含24h滑动窗口测试
- [x] CHK012 — 规格在 Constitution Alignment 中声明 "必须与 006-3 关停切片联合验收", 但 006-3 已实现完成. 是否有交叉验证清单或联合验收契约? [Consistency, Spec §Constitution Alignment] ✅ 已补充联合验收标准: shutdown不残留孤儿, 状态变更可审计

## Acceptance Criteria Quality(验收标准可度量性)

- [x] CHK013 — SC-001 "10k 次瞬时失败波形" 的波形定义 (每次间隔, 并发度, 总时长) 是否在规格或附带测试计划中写明? [Measurability, Spec §SC-001] ✅ spec.md SC-001: 间隔<=1ms, 单线程注入, 持续60s
- [x] CHK014 — SC-002 "双分组对照实验" 的对照组基准条件 (B 侧初始在线时长, A 侧注入故障的持续时间和频率) 是否可重现? [Measurability, Spec §SC-002] ✅ spec.md SC-002: B侧初始24h, A侧1次/s持续24h
- [x] CHK015 — SC-003 "typed event 与 metrics 针对同一 SupervisorDecision 键的一致率" — "同一键" 具体指哪些字段 (child_id, correlation_id, decision_type)? [Measurability, Spec §SC-003] ✅ spec.md SC-003: (child_id, decision_type, correlation_id) 三字段组合

## Scenario Coverage(场景覆盖)

- [x] CHK016 — 当 restart budget(重启预算) 在快速失败期间恰好恢复一个令牌时, 规格是否写明下一个重启请求是立即通过还是仍需等待下一个评估周期? [Coverage, Gap] ✅ T011 已覆盖此边界: 令牌恢复后立即通过下次 try_consume()
- [x] CHK017 — 当两个 group(分组) 存在双向依赖边 (`A→B` 且 `B→A`) 导致循环传播时, 规格是否写明了熔断传播的终止条件? [Edge Case, Gap] ✅ data-model.md: 依赖边构成DAG, 环形依赖在配置加载时拒绝
- [x] CHK018 — 当 optional child(可选子任务) 连续抖动失败但未触发 budget 耗尽时, backoff jitter(退避抖动) 的"打散节拍"是否写明了最小/最大间隔? [Coverage, Spec §Edge Cases] ✅ spec.md Edge Cases: [0.5×base, 1.5×base]; data-model.md BackoffJitter实体
- [x] CHK019 — US3 要求 "同一 correlation id(关联标识) 串联" 预算耗尽和升级裁决事件. 如果链路中某个中间步骤未产生事件 (例如预算直接通过, 无耗尽), correlation id 是否仍然传递到后续事件? [Coverage, Spec §FR-003] ✅ spec.md FR-003: CorrelationId贯穿全链路, 中间阶段跳过也继续传递

## Edge Case Coverage(边界条件覆盖)

- [x] CHK020 — 规格 Edge Cases 节提到的 "critical child(关键子任务) 同时挂在两个 group(分组) 且两边 policy(策略) 冲突" 的 tie-break(平局裁决) 规则 — 是否已写成可读表格, 还是仅写了"必须写成可读表格"这句话? [Edge Case, Spec §Edge Cases] ✅ spec.md Edge Cases 已含 3 行裁决规则表格
- [x] CHK021 — 规格 Edge Cases 节提到的 "meltdown(熔断) 与手动 quarantine(隔离) 并发触发" — 人工指令优先级的决策规则是否已写明? [Edge Case, Spec §Edge Cases] ✅ spec.md: 人工指令优先, 审计流水记录 operator_id+版本戳
- [x] CHK022 — 当 `SeverityClass::Critical` 的子任务同时触发 budget 耗尽和 group 熔断时, escalation path(升级路径) 的优先级是否写明了 budget 先于 fuse 还是 fuse 先于 budget? [Edge Case, Gap] ✅ FR-001 已显式规定 budget → meltdown 顺序, budget 不足直接拒绝不经过熔断

## Non-Functional Requirements(非功能需求)

- [x] CHK023 — 策略评估管线的性能约束 ("微秒级完成, 不影响控制循环主路径延迟") 是否有量化的延迟阈值 (例如 p99 < 100μs)? [Clarity, Spec §plan.md: Performance Goals] ✅ plan.md 已量化: try_consume() p99<10µs, evaluate_budget p99<100µs
- [x] CHK024 — 预算跟踪器的内存上限 (滑动窗口中最多保留多少故障时间戳) 是否写明? [Gap] ✅ spec.md Assumptions: 最坏情况内存占用 ≤ max_burst × sizeof(u128)

## Dependencies & Assumptions(依赖与假设)

- [x] CHK025 — 规格假设节 "分组故障隔离依赖运行时拓扑中的 dependency edge(依赖边) 声明, 该声明由 006-6 切片中的配置模型加载" — 006-6 切片的配置模型是否已冻结字段定义, 还是本切片需要预留适配层? [Assumption, Spec §Assumptions] ✅ 本切片自定 GroupDependencyEdge/GroupIsolationPolicy 格式, 006-6 通过薄适配器桥接

---

## Spec Validation(规格内审): "验证规格"

_本段由 `/speckit-checklist 验证规格` 命令追加. 聚焦规格自身的可追溯性, 术语一致性, 格式合规与交叉引用正确性._

### Traceability(规格→任务可追溯性)

- [x] CHK026 — FR-001 的 "fairness(公平性) 探针记录在任意连续 10 秒窗口内, 其它就绪监督动作至少获得过调度机会的计数不低于文档阈值" — 该阈值在 tasks.md 的哪个测试任务中量化? [Traceability, Spec §FR-001 → tasks.md] ✅ data-model.md min_ops_per_window(默认1), T013/T014 覆盖饥饿检测与正常场景
- [x] CHK027 — FR-003 的 "每一条分叉路径上的预算耗尽与升级裁决都必须 100% 写入 typed event(类型化事件) 与 metrics(指标) 两组管道" — tasks.md 中是否同时覆盖了事件通道 (T009, T036) 和指标通道 (T042) 的验证任务? [Traceability, Spec §FR-003 → tasks.md] ✅ T009(事件定义), T036(载荷字段), T042(观测流水线), T033(事件/指标一致率验证)
- [x] CHK028 — SC-002 的双分组 24h 对照实验在 tasks.md 的 Phase 4(US2) 测试中是否包含时间窗口配置, 还是仅做单元级隔离断言? [Traceability, Spec §SC-002 → tasks.md §Phase 4] ✅ T024 含 test_group_isolation_24h_sliding_window, T030 计数更新为 4 个测试

### Terminology Consistency(术语一致性)

- [x] CHK029 — 规格中 "restart budget(重启预算)" 与 data-model.md 中 `RestartBudgetTracker(重启预算跟踪器)` 的术语是否在全文所有文档 (spec/plan/data-model/contracts/tasks) 中统一使用, 还是存在 "budget tracker" "预算器" 等别名? [Consistency] ✅ 全文统一使用 RestartBudget(重启预算)/RestartBudgetTracker(重启预算跟踪器)
- [x] CHK030 — 规格 US2 验收场景中的 "HealthyBaseline(健康基线) 计数" 是否在其他文档中定义, 还是仅在此处出现一次? [Ambiguity, Spec §US2] ✅ HealthyBaseline 已从 spec.md 移除, 改用 GroupCounter + group_fuse_active()

### Spec Format Compliance(规格格式合规)

- [x] CHK031 — 规格中的 "Key Entities(关键实体)" 节列出了 3 个实体, 但 plan.md 和 data-model.md 额外新增了 `FairnessProbe`, `StarvationAlert`, `RestartBudgetTracker`, `GroupCounter` 等. 这些实体是否需要回填到 spec.md 的 Key Entities 节? [Gap, Spec §Key Entities] ✅ **已修复: spec.md Key Entities 节已补齐 6 个实体.**
- [x] CHK032 — 规格 "Success Criteria(成功标准)" 节有 3 项 SC, 但全部是定量指标. 是否有任何定性成功标准 (如 "所有策略决策路径均可被日志复现") 被遗漏? [Completeness, Spec §Success Criteria] ✅ spec.md SC-000: 所有策略决策路径可从 typed event 流中重建

### Cross-Reference Accuracy(交叉引用正确性)

- [x] CHK033 — 规格定位为 "006 系列的第二序列里程碑", 但依赖列表写了 005-1, 005-2 和 006-3. 006-3 是"第一序列里程碑" 且已实现完成 — 规格中是否需要更新依赖状态标注 (如 "006-3 已完成, 本切片基于其 ChildSlot 基础设施")? [Consistency, Spec §Dependency Note] ✅ **已修复: Dependency Note 已标注各依赖切片状态, 006-3 标记为已完成.**
- [x] CHK034 — 规格 "Edge Cases(边界情况)" 提到的 "tie-break(平局裁决) 规则必须写成可读表格" — 该表格在当前规格或设计文档中是否存在, 还是仅在规格中提了要求? [Gap, Spec §Edge Cases] ✅ **已修复: spec.md Edge Cases 已新增 3 行 tie-break 裁决规则表格.**

### Coverage Gaps(覆盖缺口)

- [x] CHK035 — US1 验收场景只覆盖了"失败波形重复滚动 60 秒"的主路径. 恢复路径 (预算令牌恢复后首次重启成功) 的验收场景是否缺失? [Coverage, Spec §US1] ✅ **已修复: US1 新增验收场景 2 — 预算令牌恢复路径.**
- [x] CHK036 — 规格提到熔断后的 "fuse_active(熔断激活)" 状态, 但未明确是否定义熔断的自动恢复条件 (如 `reset_after` 倒计时). 其他文档 (research.md, contracts/) 是否覆盖了此语义? [Coverage, Spec §US2 vs research.md] ✅ data-model.md: reset_after到期恢复 + 倒计时期间零星故障不重置规则

### Dependency Validation(依赖校验)

- [x] CHK037 — 规格声明强依赖 005-1(failure-policy-reliability) 和 005-2(work-role-defaults). 这两个依赖功能的 spec.md 中是否已定义了本切片所需的接口契约 (如 PolicyEngine 的 decide() 方法签名的稳定承诺)? [Dependency, Spec §Dependency Note] ✅ Dependency Note 已补充接口契约: 接入005-1六阶段管线的 evaluate_budget, 通过 role_defaults.rs 映射对接005-2 WorkRole

---

## Observability Contract Quality(可观测性契约质量)

_本段由 `/speckit-checklist` 命令基于 Strict(严格 release gate) 级别追加. 聚焦 typed event(类型化事件) 与 metrics(指标) 双通道的契约完备性与稳定性._

- [x] CHK038 — `BudgetExhausted` 事件载荷中的 `retry_after_ns` 字段是否在 contracts/ 或 spec.md 中承诺为稳定字段, 不会在 minor 版本中删除或重命名? [Clarity, Spec §FR-001] ✅ contracts/restart-budget-api.md 已冻结 BudgetVerdict::Exhausted { retry_after_ns: u128 }
- [x] CHK039 — typed event(类型化事件) 的 schema versioning(模式版本化) — 规格或 contracts/ 中是否定义了事件格式的向后兼容策略 (如新增字段是否允许, 删除字段的 deprecation 周期)? [Gap] ✅ spec.md Diagnostics: version字段(v=1), 新字段minor追加, 删除需major deprecation周期
- [x] CHK040 — `CorrelationId(关联标识)` 的生命周期 — 规格是否写明了 CorrelationId 的生成时机 (故障首次出现? 评估管线入口?), 退役时机 (事件发射后立即失效? 窗口结束后?), 以及跨子任务重启是否保持? [Gap, Spec §FR-003] ✅ spec.md Edge Cases: 故障入管线时生成, 链路事件发完后退役, 跨重启不保持
- [x] CHK041 — metrics(指标) 标签的 cardinality(基数) 上限 — 规格是否定义了每个 metrics 维度的合法标签值集合 (如 `severity_class` 只有 `Critical/Optional/Standard` 三个枚举值), 还是放任无界标签导致时序数据库爆炸? [Gap] ✅ spec.md Assumptions: severity_class仅3值, budget_verdict仅2值, fuse_active仅bool
- [x] CHK042 — 当同一 child 在短时间内连续触发多次 budget 裁决时, typed event 的 ordering(排序) 保证 — 规格是否声明了 per-child 事件严格有序, 还是允许并发乱序? [Gap, Spec §FR-001] ✅ spec.md Diagnostics: 同child事件在单次control loop迭代内按emission timestamp严格有序
- [x] CHK043 — `PipelineStageDiagnostic` 是否覆盖了全部策略阶段 (budget → meltdown → backoff → escalation) 的诊断输出, 还是仅覆盖部分阶段? [Completeness, research.md §问题5] ✅ T042 为 BudgetExhausted, GroupFuseTriggered, EscalationBifurcated 三种事件添加 PipelineStageDiagnostic, 覆盖全部阶段

---

## Isolation Correctness(隔离正确性深度检查)

_本段由 `/speckit-checklist` 命令基于 Strict(严格 release gate) 级别追加. 聚焦分组故障边界的声明语义, 默认行为, 恢复条件与跨组传播的完备性._

- [x] CHK044 — `GroupDependencyEdge` 的 `from_group` 与 `to_group` 方向语义 — data-model.md 中 `from_group` 标注为"依赖方分组名", 但该方向是否与 `PropagationPolicy` 的传播方向自洽 (故障从 `to_group` 传播到 `from_group`)? [Ambiguity, data-model.md vs Spec §FR-002] ✅ data-model.md PropagationPolicy::Full 注释已明确: 故障从 to_group 单向传播到 from_group
- [x] CHK045 — 当一个 child 不属于任何 group(分组)(`group_name: None`) 时, 其熔断隔离的默认行为 — 是永不隔离, 归入隐式默认分组, 还是全局熔断时一并隔离? [Gap, Spec §FR-002] ✅ data-model.md: 仅触发 child 级熔断, 不参与 group 级熔断传播
- [x] CHK046 — 分组熔断后的恢复条件 (`reset_after` 倒计时) — 如果恢复倒计时期间又出现零星故障, 是重置倒计时从头开始, 还是继续等待原倒计时结束? research.md 提到了 reset_after, 但 spec.md 未写明此行为. [Gap, Spec §US2 vs research.md] ✅ data-model.md: 零星故障不重置倒计时, 故障密度超阈值才重新触发
- [x] CHK047 — 同一 child 同时在配置中显式声明 `severity` 和归属 `group`, 但 group 级策略中定义的 severity 覆盖规则与 child 级声明冲突时, 优先级 (child 级 > group 级? 或反之?) 是否在规格中写明? [Gap, Spec §Edge Cases] ✅ spec.md tie-break表新增第4行: child级显式声明优先于group默认值
- [x] CHK048 — 分组依赖边是否为有向无环图(DAG)? 规格是否写明了环形依赖的检测与拒绝规则? [Edge Case, Spec §FR-002] ✅ data-model.md: DAG约束, 配置加载时检测环形依赖并返回结构化错误

---

## Timing & Precision Requirements(时序与精度需求)

_本段由 `/speckit-checklist` 命令基于 Strict(严格 release gate) 级别追加. 聚焦退避抖动, 令牌回收, 探针窗口, 时钟源选择的量化与精度要求._

- [x] CHK049 — backoff jitter(退避抖动) 的随机范围 — 规格是否写明了 jitter 系数的最小/最大边界 (如 ±25% 或 [0.5×base, 1.5×base]), 还是仅写了"打散重启节拍"这个定性目标? [Clarity, Spec §Edge Cases] ✅ spec.md Edge Cases + data-model.md BackoffJitter: [0.5, 1.5]
- [x] CHK050 — 令牌桶的 `recovery_rate_per_sec` 使用 `f64` 浮点数 — 浮点精度在连续运行数月后的累积误差是否在规格容忍范围内? 是否有使用定点数或 `Duration` 类型的精度要求? [Gap, research.md §问题1] ✅ data-model.md: 累积误差<=1ms, 对秒级以上令牌归还精度影响可忽略
- [x] CHK051 — `FairnessProbe`(公平性探针) 的探测间隔 — spec.md US1 中提到"任意连续 10 秒窗口", data-model.md 写的是"默认 10s". 该窗口是否可配置, 配置后是否影响 SC-001 的度量基准? [Consistency, Spec §FR-001 vs data-model.md] ✅ data-model.md probe_interval_ns 字段标注"默认 10s"(隐含可配), 与 spec.md 一致
- [x] CHK052 — 策略评估管线使用的时钟源 — 规格或设计文档是否写明了使用 monotonic clock(单调时钟) 还是 wall clock(挂钟时间)? 系统时间跳变 (如 NTP 校时, 闰秒) 是否会影响 budget 令牌恢复或 meltdown 恢复倒计时的正确性? [Gap] ✅ spec.md Assumptions: 系统时钟为 monotonic clock, 不受 NTP 校时或闰秒影响
- [x] CHK053 — 令牌归还与故障入队操作的原子性 — 规格或 contracts/ 中是否写明了 `try_consume()` 中"驱逐过期故障 → 归还令牌 → 检查令牌 ≥ 1.0"这三个步骤的原子性保证, 还是允许并发交错? [Gap, research.md §风险点1] ✅ data-model.md: &mut self 内原子完成, 调用方无需额外加锁

---

## Cross-Cutting Quality Gates(横向质量门)

_本段由 `/speckit-checklist` 命令基于 Strict(严格 release gate) 级别追加. 聚焦配置验证, 热更新, 子系统冲突裁决, 降级路径等横切质量需求._

- [x] CHK054 — 策略配置的合法性验证 — 规格是否定义了非法配置值 (如 `max_burst = 0`, `window = Duration::ZERO`, `recovery_rate_per_sec <= 0.0`) 的拒绝行为与错误消息格式? [Gap] ✅ data-model.md RestartBudgetConfig 字段约束: 非法值以结构化错误拒绝
- [x] CHK055 — 策略热更新(hot-reload)语义 — 运行时通过配置通道更新 budget 或 group 依赖边后, 已累积的令牌计数和熔断状态是重置归零还是保留? 规格是否写明了热更新行为? [Gap] ✅ spec.md Edge Cases: 不支持运行时热更新, 修改配置需重启监督器
- [x] CHK056 — 多策略子系统冲突裁决 — 当 budget, meltdown, backoff 三个子系统对同一故障给出不同裁决 (如 budget 通过但 meltdown 拒绝) 时, 最终裁决的优先级顺序 (budget → meltdown → backoff) 是否在 spec.md, plan.md, research.md 三份文档中完全一致? [Consistency, Spec §FR-001 vs research.md §问题5] ✅ 三文档已统一为 budget → meltdown → backoff
- [x] CHK057 — 降级路径 — 当 `FairnessProbe` 或 `RestartBudgetTracker` 内部状态因 bug 损坏 (如计数器溢出, 时间戳倒流) 时, 系统是否定义了降级行为 (跳过该子系统, 发射 `degraded_mode` 事件), 还是硬崩溃? [Gap] ✅ spec.md Edge Cases: 发射degraded_mode, budget损坏跳过限流, fairness损坏跳过饥饿检测, meltdown+backoff仍保护
- [x] CHK058 — 规格 "Assumptions(假设)" 节仅列出 2 条假设 (metrics 后端由集成方注入, 分组依赖边由 006-6 加载). 是否还有其他隐含假设 (如单进程内最多 N 个 group, 时钟单调性, RNG seed 确定性) 需要显式声明? [Completeness, Spec §Assumptions] ✅ 已补全至 5 条: monotonic clock, group/child 上限 64/256, 内存上限 max_burst×sizeof(u128)
