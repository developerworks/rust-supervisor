# Events Requirements Quality Checklist(事件需求质量检查清单)

**Purpose(目的)**: 验证 `006-5-typed-events-observability` 功能规格中事件类型化, correlation id 追踪和背压处理需求的质量, 完整性与可度量性.

**Created(创建日期)**: 2026-05-18
**Scope(范围)**: US1(类型化事件) + US2(Correlation ID 追踪) + US3(慢订阅者背压), 全部 3 个用户故事
**Depth(深度)**: Strict(严格 release gate)
**Gates(关口)**: 事件 schema 完备性, correlation id 全链路覆盖, 背压策略可验证

---

## Requirement Completeness(需求完整性)

- [ ] CHK001 — FR-001 要求"每一次合法的状态迁移至少对应到一个稳定的 schema id 版本化事件变体". 完整的控制循环迁移弧清单是否在规格中列出? 如果没有清单, 95% 的度量基数无法确定. [Gap, Spec §FR-001]
- [ ] CHK002 — FR-002 要求 journal(事件日志), tracing(链路追踪), metrics(指标) 三类出口消费同一套结构化字段释义. 该字段字典的权威来源 (文件路径/文档节) 是否在规格中写明? [Gap, Spec §FR-002]
- [ ] CHK003 — FR-003 要求 correlation id(关联标识) 在多次重启之间保持稳定. correlation id 的生成策略 (如 UUID v4), 存储位置 (事件载荷? 日志上下文? 指标标签?) 和生命周期 (何时生成, 何时退役) 是否在规格中完整定义? [Gap, Spec §FR-003]
- [ ] CHK004 — US3 的背压策略要求"二者只能择其一写在默认配置文件中". 该默认配置项的名称, 可选值和默认值是否已在规格或配置模板中写明? [Gap, Spec §US3]
- [ ] CHK005 — Edge Cases 提到"审计承载的高风险改写事件必须落在单独 channel 维持全量语义". 该单独 channel 的容量, 优先级和消费者接口是否在规格中定义? [Gap, Spec §Edge Cases]
- [ ] CHK006 — Key Entities 只列出了 `SupervisorEvent` 和 `CorrelationHandle` 两个实体. 但 US3 涉及"背压阈值","采样比例","缓冲区水位"等概念 — 这些是否也需要作为 Key Entities 定义? [Completeness, Spec §Key Entities]

## Requirement Clarity(需求清晰度)

- [ ] CHK007 — US1 验收场景中的 `event_variant(字段名示例)` 明确标注了"示例". 实际字段名约定 (snake_case? camelCase? 带前缀?) 是否在契约文档中冻结? 发布门禁不允许字段名停留在"示例"状态. [Clarity, Spec §US1]
- [ ] CHK008 — SC-001 要求"控制循环迁移弧清单里至少 95% 的行以 SupervisorEvent 稳定变体字段作为主要契约字段". "迁移弧清单"的具体格式 (表格? 枚举定义? 文档节?) 和存放位置是否已明确? [Clarity, Spec §SC-001]
- [ ] CHK009 — SC-002 要求"随机抽取 100 条失败复盘样本, 至少 97 条能在 5 分钟内只靠 correlation id 拼接完整段落". "5 分钟"的计时起点和终点是什么? 是从查询发出到结果返回, 还是从人工开始翻日志到段落拼接完成? [Clarity, Spec §SC-002]
- [ ] CHK010 — US3 验收场景中的 `latency_inject_ms(字段名示例)` 标注了"示例". 慢订阅者的具体判定阈值 (延迟超过多少毫秒算"慢"? 缓冲区占用超过百分之多少触发告警?) 是否已在规格中量化? [Clarity, Spec §US3]
- [ ] CHK011 — "97 条"的抽样方法 (简单随机抽样? 分层抽样? 按时间段分层?) 是否在规格或测试计划中写明? 样本的代表性直接决定 SC-002 的可信度. [Clarity, Spec §SC-002]
- [ ] CHK012 — US3 的"二者只能择其一" — 该选择是在编译期通过 feature flag 确定, 还是在运行时通过配置开关切换? 编译期 vs 运行时的选择影响测试覆盖策略. [Clarity, Spec §US3]

## Requirement Consistency(需求一致性)

- [ ] CHK013 — FR-001 要求"每一次合法的状态迁移"都对应到事件变体, 但 US3 允许慢订阅者时采样丢弃事件. "采样丢弃"是否与"每一次"的承诺矛盾? 规格是否定义了哪些事件变体属于"可采样"类别, 哪些属于"必须全量"类别? [Conflict, Spec §FR-001 vs US3]
- [ ] CHK014 — Edge Cases 规定"审计承载的高风险改写事件必须落在单独 channel 维持全量语义", 但 FR-002 说"审计通道承载的高风险改写动作默认禁止采样". 两者表述一致但未定义"高风险"的判定标准 — 同一标准是否在两个条款中保持一致? [Consistency, Spec §FR-002 vs Edge Cases]
- [ ] CHK015 — US2 要求 correlation id 跨多次重启保持稳定, 但 006-4 的 Edge Cases 规定"跨子任务重启时 CorrelationId 不保持(每次故障链路独立)". 本切片与 006-4 的 correlation id 生命周期规则是否已在依赖说明中协调一致? [Consistency, Spec §FR-003 → specs/006-4/]
- [ ] CHK016 — FR-003 要求"查询 spawn 到 shutdown 任一阶段缺失时必须产出 gap_alarm". 这里隐含了"所有五段都必须有事件"的前置条件. 如果某个 child 在 ready 之前就被 shutdown, 是否也要求 gap_alarm? 规格对"合法缺失"和"异常缺失"的区分是否一致? [Consistency, Spec §FR-003 vs US2]

## Acceptance Criteria Quality(验收标准可度量性)

- [ ] CHK017 — SC-001 的 "95%" 分母 (控制循环迁移弧总数) 是否有一个权威清单可枚举? 如果没有完整清单, 95% 无法客观度量. [Measurability, Spec §SC-001]
- [ ] CHK018 — SC-002 的 "5 分钟内" 和 "97 条" 是否设计为自动化测试 (如 CI 中定时运行), 还是人工抽样审计? 如果是自动化, 测试脚本的输入 (失败复盘样本集) 是否有稳定版本? [Measurability, Spec §SC-002]
- [ ] CHK019 — US1 的 Independent Test "给控制循环每段迁移打上唯一的 enum 成员断言, 跑一次冒烟套件验证穷尽覆盖" — "穷尽覆盖"是指覆盖所有 enum 成员, 还是覆盖所有可能的迁移路径? 枚举成员的完整性靠什么保证? [Measurability, Spec §US1]
- [ ] CHK020 — US3 的 Independent Test "人为限速订阅回调, 测量缓冲区水位 metrics 是否在阈值触发告警" — 告警阈值的具体数值和单位是否已在配置或规格中写明? 没有阈值就无法判断测试通过/失败. [Measurability, Spec §US3]

## Scenario Coverage(场景覆盖)

- [ ] CHK021 — US2 要求 correlation id 覆盖 spawn, ready, failure decision, restart attempt, shutdown 五段. 是否还有其他生命周期阶段 (如 paused, quarantined, health check passed/failed) 也需要纳入 correlation id 追踪范围? [Coverage, Spec §US2]
- [ ] CHK022 — US3 只覆盖了"慢订阅者"一种背压场景. command channel 被塞满、IPC connection 风暴、event bus 内部缓冲区溢出等是否也需要定义行为? 或者这些由其他切片覆盖? [Coverage, Spec §US3]
- [ ] CHK023 — correlation id 查询场景: 如果传入的 correlation id 不存在 (如拼写错误或已过期), 查询 API 返回什么? 空结果? 结构化错误? [Coverage, Spec §US2]
- [ ] CHK024 — 当多个 child 在同一次 control loop 迭代中产生事件时, 事件的排序保证 (per-child 有序? 全局有序? 按 child_id 字典序?) 是否定义? [Coverage, Spec §FR-001]

## Edge Case Coverage(边界条件覆盖)

- [ ] CHK025 — SupervisorEvent 的 schema id 版本化策略: schema 升级时 (新增字段/废弃字段/重命名字段), 旧版本事件是否仍能被下游消费? 向后兼容的承诺期限是否定义? [Edge Case, Spec §FR-001]
- [ ] CHK026 — 事件的序列化失败处理: 如果某个 SupervisorEvent 因字段不合法或序列化器异常而无法产出 JSON, 控制循环是重试、跳过还是 panic? [Edge Case, Spec §FR-001]
- [ ] CHK027 — correlation id 的冲突处理: 如果两个不同的故障链路在极低概率下生成了相同的 correlation id (如 UUID 碰撞), 查询 API 会返回合并结果还是检测到冲突? [Edge Case, Spec §FR-003]
- [ ] CHK028 — 背压策略中的"保护性降级停机分支" — 触发降级停机后, 正在运行的 child 是全部 abort 还是等待 graceful timeout? 降级停机的审计记录是否包含触发原因和采样比例? [Edge Case, Spec §US3]
- [ ] CHK029 — 当 `audit_enabled: false` 时, "审计通道禁止采样"的约束是否自动失效? 规格是否定义了 audit 禁用时的替代防护措施? [Edge Case, Spec §FR-002]

## Non-Functional Requirements(非功能需求)

- [ ] CHK030 — 事件序列化和发射的性能预算是否定义? 控制循环主路径上每个事件的处理延迟 p99 上限是微秒级还是毫秒级? 没有量化阈值则无法判断序列化开销是否可接受. [NFR, Gap]
- [ ] CHK031 — Edge Cases 提到"tracing 与 metrics 的标签基数必须有文档硬上限". 该硬上限的具体数值 (如每个 span 不超过 10 个标签, 每个标签不超过 100 个唯一值) 和超限时的处理策略 (拒绝写入? 截断? 告警?) 是否已定义? [NFR, Gap, Spec §Edge Cases]
- [ ] CHK032 — Event stream (broadcast channel) 的容量上限是否已定义? 当 subscriber 消费速度持续低于生产速度时, channel 是阻塞发送方、丢弃最旧事件、还是返回错误? [NFR, Gap]
- [ ] CHK033 — 事件 journal 的固定容量已由 002 切片配置 (`event_journal_capacity`). 当 journal 满时是覆盖最旧事件还是停止写入? 行为是否与背压策略一致? [NFR, Spec §FR-002]

## Dependencies & Assumptions(依赖与假设)

- [ ] CHK034 — 规格声明强依赖 `specs/005-1-failure-policy-reliability/contracts/pipeline-and-events.md`. 该契约的 `SupervisorEvent` 事件变体集合是否已冻结? 如果 005-1 仍在迭代, 本切片的事件 schema 是否与其保持同步? [Dependency, Spec §Dependency Note]
- [ ] CHK035 — 假设"调用方负责装配具体 OpenTelemetry 导出栈". 监督器是否提供默认的 no-op 实现, 使得未配置导出栈时系统不会 panic 或报错? [Assumption, Spec §Assumptions]
- [ ] CHK036 — 假设"correlation id 的生成策略由计划阶段 data-model.md 冻结". 但 006-5 目前没有 plan.md 和 data-model.md. correlation id 的生成算法是否需要在当前 spec 中先行固定, 以免后续切片依赖断裂? [Assumption, Spec §Assumptions]
- [ ] CHK037 — 规格提到"schema id 抬版本时必须附带人类可读迁移脚注段落". schema 版本的管理责任 (谁批准版本晋升? 版本号格式? 迁移脚注存放位置?) 是否已明确? [Dependency, Gap]

## Ambiguities & Conflicts(歧义与冲突)

- [ ] CHK038 — 规格全文多处使用 `sample_ratio(字段名示例)`, `gap_alarm(字段名示例)`, `latency_inject_ms(字段名示例)`, `event_variant(字段名示例)`, `investigation_blocked(字段名示例)`. 这些字段名标注了"示例", 但发布门禁要求字段名稳定. 是否需要在当前规格中为这些字段提供"建议字段名"作为占位, 还是全部推给 plan 阶段? [Ambiguity, Spec §US1/US2/US3]
- [ ] CHK039 — US2 的验收场景要求"返回数组必须按时间顺序把上述五个阶段都盖住, 或在缺口位置抛出可读 structured error". "按时间顺序"是指 wall clock 顺序还是 monotonic clock 顺序? 如果系统时钟在记录期间发生跳变 (NTP 校时), 排序是否仍然可靠? [Ambiguity, Spec §US2]
- [ ] CHK040 — FR-002 要求"journal, tracing, metrics 三类出口必须消费同一套结构化字段释义". 但 journal 是固定容量缓冲区, tracing 是 span/event, metrics 是时序数据点. 三类出口的持久化语义和查询能力完全不同 — "同一套字段释义"是指字段名相同, 还是也包括数据类型和序列化格式相同? [Ambiguity, Spec §FR-002]

## Constitution Compliance(宪章合规)

- [ ] CHK041 — Constitution Alignment 要求"事件模型要把监督状态迁移的每一条弧都盖上". 该要求与 FR-001 的"每一次合法状态迁移"一致. 但控制循环的状态迁移图是否以机器可读格式 (如 DOT 图或枚举定义) 存在于 specs/ 或 src/ 中? [Completeness, Spec §Constitution Alignment]
- [ ] CHK042 — Module ownership 要求"SupervisorEvent 的 schema 定义集中在 observe 模块名下维护". 当前是否已经有一个集中的事件 schema 定义文件 (如 `src/event/payload.rs` 或 `src/observe/schema.rs`) 被明确指定为 schema 权威来源? [Compliance, Spec §Module ownership]
- [ ] CHK043 — 术语格式:"英文术语必须写成 `English(中文说明)`". 规格中使用了 `gap_alarm(字段名示例)` 和 `investigation_blocked(字段名示例)` 等标注"示例"的字段名. 这些"示例标注"是否符合术语格式要求, 还是需要在冻结字段名后更新? [Compliance, Spec §Chinese Writing]

## Notes(说明)

- 完成检查项后使用 `[x]` 标记.
- 评论或发现可以直接写在相关检查项下.
- 需要时链接相关资源或文档.
- 检查项必须按顺序编号, 方便引用.
