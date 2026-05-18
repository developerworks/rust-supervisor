# Events Requirements Quality Checklist(事件需求质量检查清单)

**Purpose(目的)**: 验证 `006-5-typed-events-observability` 功能规格中事件类型化, correlation id 追踪和背压处理需求的质量, 完整性与可度量性.

**Created(创建日期)**: 2026-05-18
**Scope(范围)**: US1(类型化事件) + US2(Correlation ID 追踪) + US3(慢订阅者背压), 全部 3 个用户故事
**Depth(深度)**: Strict(严格 release gate)
**Gates(关口)**: 事件 schema 完备性, correlation id 全链路覆盖, 背压策略可验证

---

## Requirement Completeness(需求完整性)

- [x] CHK001 — FR-001 要求"每一次合法的状态迁移至少对应到一个稳定的 schema id 版本化事件变体". 完整的控制循环迁移弧清单是否在规格中列出? 如果没有清单, 95% 的度量基数无法确定. [Resolved, data-model.md §State Transitions 表列出所有迁移弧与其 What 变体映射]
- [x] CHK002 — FR-002 要求 journal(事件日志), tracing(链路追踪), metrics(指标) 三类出口消费同一套结构化字段释义. 该字段字典的权威来源 (文件路径/文档节) 是否在规格中写明? [Resolved, contracts/typed-event-schema.md §1 定义 SupervisorEvent 顶层字段字典; §3 定义 What 枚举变体字段]
- [x] CHK003 — FR-003 要求 correlation id(关联标识) 在多次重启之间保持稳定. correlation id 的生成策略 (如 UUID v4), 存储位置 (事件载荷? 日志上下文? 指标标签?) 和生命周期 (何时生成, 何时退役) 是否在规格中完整定义? [Resolved, research.md R001(UUID v4 生成) + R002(三通道传播); contracts/typed-event-schema.md §2(传播契约)]
- [x] CHK004 — US3 的背压策略要求"二者只能择其一写在默认配置文件中". 该默认配置项的名称, 可选值和默认值是否已在规格或配置模板中写明? [Resolved, data-model.md §BackpressureConfig: strategy(默认 AlertAndBlock), warn_threshold_pct(80), critical_threshold_pct(95); contracts/typed-event-schema.md §5 定义可选值]
- [x] CHK005 — Edge Cases 提到"审计承载的高风险改写事件必须落在单独 channel 维持全量语义". 该单独 channel 的容量, 优先级和消费者接口是否在规格中定义? [Resolved, research.md R010(独立 broadcast channel); data-model.md BackpressureConfig.audit_channel_capacity(默认 1024)]
- [x] CHK006 — Key Entities 只列出了 `SupervisorEvent` 和 `CorrelationHandle` 两个实体. 但 US3 涉及"背压阈值","采样比例","缓冲区水位"等概念 — 这些是否也需要作为 Key Entities 定义? [Resolved, data-model.md 补充了 BackpressureConfig(含阈值) 和 BackpressureStrategy(含采样比例)]

## Requirement Clarity(需求清晰度)

- [x] CHK007 — US1 验收场景中的 `event_variant(字段名示例)` 明确标注了"示例". 实际字段名约定 (snake_case? camelCase? 带前缀?) 是否在契约文档中冻结? 发布门禁不允许字段名停留在"示例"状态. [Resolved, contracts/typed-event-schema.md §4.2: What 变体序列化为 snake_case JSON(type + payload); 所有新增变体名均已冻结]
- [x] CHK008 — SC-001 要求"控制循环迁移弧清单里至少 95% 的行以 SupervisorEvent 稳定变体字段作为主要契约字段". "迁移弧清单"的具体格式 (表格? 枚举定义? 文档节?) 和存放位置是否已明确? [Resolved, data-model.md §State Transitions 表格列出了每个迁移弧与 What 变体的映射]
- [x] CHK009 — SC-002 要求"随机抽取 100 条失败复盘样本, 至少 97 条能在 5 分钟内只靠 correlation id 拼接完整段落". "5 分钟"的计时起点和终点是什么? 是从查询发出到结果返回, 还是从人工开始翻日志到段落拼接完成? [Resolved, spec.md SC-002: 计时以 CorrelationHandle 查询 API 响应时间计(不含人工操作), 5 分钟为系统响应时间上限]
- [x] CHK010 — US3 验收场景中的 `latency_inject_ms(字段名示例)` 标注了"示例". 慢订阅者的具体判定阈值 (延迟超过多少毫秒算"慢"? 缓冲区占用超过百分之多少触发告警?) 是否已在规格中量化? [Resolved, data-model.md BackpressureConfig: warn_threshold_pct(80% 软阈值), critical_threshold_pct(95% 硬阈值)]
- [x] CHK011 — "97 条"的抽样方法 (简单随机抽样? 分层抽样? 按时间段分层?) 是否在规格或测试计划中写明? 样本的代表性直接决定 SC-002 的可信度. [Resolved, spec.md SC-002: 按 child_id 分层随机抽样, CI 自动化执行]
- [x] CHK012 — US3 的"二者只能择其一" — 该选择是在编译期通过 feature flag 确定, 还是在运行时通过配置开关切换? 编译期 vs 运行时的选择影响测试覆盖策略. [Resolved, research.md R003: 运行时配置开关(BackpressureStrategy 枚举), 非编译期 feature flag]

## Requirement Consistency(需求一致性)

- [x] CHK013 — FR-001 要求"每一次合法的状态迁移"都对应到事件变体, 但 US3 允许慢订阅者时采样丢弃事件. "采样丢弃"是否与"每一次"的承诺矛盾? 规格是否定义了哪些事件变体属于"可采样"类别, 哪些属于"必须全量"类别? [Resolved, research.md R010: audit 通道独立且禁止采样; BackpressureDegradation 和 AuditRecorded 是采样相关的事件变体, 不影响已有状态迁移弧的类型化事件]
- [x] CHK014 — Edge Cases 规定"审计承载的高风险改写事件必须落在单独 channel 维持全量语义", 但 FR-002 说"审计通道承载的高风险改写动作默认禁止采样". 两者表述一致但未定义"高风险"的判定标准 — 同一标准是否在两个条款中保持一致? [Resolved, data-model.md §Validation Rule 7: 定义三条高风险标准(非环回地址/生命周期影响/audit_required标记)]
- [x] CHK015 — US2 要求 correlation id 跨多次重启保持稳定, 但 006-4 的 Edge Cases 规定"跨子任务重启时 CorrelationId 不保持(每次故障链路独立)". 本切片与 006-4 的 correlation id 生命周期规则是否已在依赖说明中协调一致? [Resolved, research.md R001: CorrelationHandle 可按 child 或 supervisor 作用域限定; 每次故障链路独立的生命周期与跨阶段追踪不矛盾]
- [x] CHK016 — FR-003 要求"查询 spawn 到 shutdown 任一阶段缺失时必须产出 gap_alarm". 这里隐含了"所有五段都必须有事件"的前置条件. 如果某个 child 在 ready 之前就被 shutdown, 是否也要求 gap_alarm? 规格对"合法缺失"和"异常缺失"的区分是否一致? [Resolved, contracts/correlation-api.md §3: 五段覆盖为"若发生则必须存在"; 合法缺失不触发 gap; 异常缺失触发 CorrelationGapDetected]

## Acceptance Criteria Quality(验收标准可度量性)

- [x] CHK017 — SC-001 的 "95%" 分母 (控制循环迁移弧总数) 是否有一个权威清单可枚举? 如果没有完整清单, 95% 无法客观度量. [Resolved, data-model.md §State Transitions 表列出所有迁移弧; tests/typed_event_coverage_test.rs 将验证穷尽覆盖]
- [x] CHK018 — SC-002 的 "5 分钟内" 和 "97 条" 是否设计为自动化测试 (如 CI 中定时运行), 还是人工抽样审计? 如果是自动化, 测试脚本的输入 (失败复盘样本集) 是否有稳定版本? [Resolved, spec.md SC-002: CI 自动化复盘脚本, 从近 24h 生产记录中抽样]
- [x] CHK019 — US1 的 Independent Test "给控制循环每段迁移打上唯一的 enum 成员断言, 跑一次冒烟套件验证穷尽覆盖" — "穷尽覆盖"是指覆盖所有 enum 成员, 还是覆盖所有可能的迁移路径? 枚举成员的完整性靠什么保证? [Resolved, data-model.md §State Transitions 提供权威迁移弧列表; What 枚举定义在 src/event/payload.rs 中; 测试验证所有枚举成员可构造+序列化]
- [x] CHK020 — US3 的 Independent Test "人为限速订阅回调, 测量缓冲区水位 metrics 是否在阈值触发告警" — 告警阈值的具体数值和单位是否已在配置或规格中写明? 没有阈值就无法判断测试通过/失败. [Resolved, data-model.md BackpressureConfig: warn_threshold_pct(80%), critical_threshold_pct(95%)]

## Scenario Coverage(场景覆盖)

- [x] CHK021 — US2 要求 correlation id 覆盖 spawn, ready, failure decision, restart attempt, shutdown 五段. 是否还有其他生命周期阶段 (如 paused, quarantined, health check passed/failed) 也需要纳入 correlation id 追踪范围? [Resolved, data-model.md §State Transitions 扩展迁移弧涵盖 health_check/ pause/resume/quarantine; CorrelationHandle 通过 events 向量存储所有关联事件, 不限于五段]
- [x] CHK022 — US3 只覆盖了"慢订阅者"一种背压场景. command channel 被塞满、IPC connection 风暴、event bus 内部缓冲区溢出等是否也需要定义行为? 或者这些由其他切片覆盖? [Accepted, data-model.md §Scope & Boundaries: 本切片仅覆盖 slow subscriber 背压; 其他场景由后续切片或基础设施层处理]
- [x] CHK023 — correlation id 查询场景: 如果传入的 correlation id 不存在 (如拼写错误或已过期), 查询 API 返回什么? 空结果? 结构化错误? [Resolved, contracts/correlation-api.md §2: CorrelationNotFound 错误变体]
- [x] CHK024 — 当多个 child 在同一次 control loop 迭代中产生事件时, 事件的排序保证 (per-child 有序? 全局有序? 按 child_id 字典序?) 是否定义? [Resolved, contracts/typed-event-schema.md §2.2: 按 when.unix_nanos 时间升序排列]

## Edge Case Coverage(边界条件覆盖)

- [x] CHK025 — SupervisorEvent 的 schema id 版本化策略: schema 升级时 (新增字段/废弃字段/重命名字段), 旧版本事件是否仍能被下游消费? 向后兼容的承诺期限是否定义? [Resolved, research.md R005: 单调递增 u64 schema_id; 向后兼容期: 当前版本 + 上一版本]
- [x] CHK026 — 事件的序列化失败处理: 如果某个 SupervisorEvent 因字段不合法或序列化器异常而无法产出 JSON, 控制循环是重试、跳过还是 panic? [Resolved, data-model.md §Validation Rule 6: 不得 panic, 记录结构化错误到 stderr 并继续执行]
- [x] CHK027 — correlation id 的冲突处理: 如果两个不同的故障链路在极低概率下生成了相同的 correlation id (如 UUID 碰撞), 查询 API 会返回合并结果还是检测到冲突? [Resolved, contracts/correlation-api.md §2: CorrelationConflict 错误变体]
- [x] CHK028 — 背压策略中的"保护性降级停机分支" — 触发降级停机后, 正在运行的 child 是全部 abort 还是等待 graceful timeout? 降级停机的审计记录是否包含触发原因和采样比例? [Resolved, data-model.md §Backpressure Behavior: 降级仅影响事件发射路径(buffer backlog), 不影响正在运行的 child; BackpressureDegradation 包含 buffer_peak_pct/strategy/recovered; AuditRecorded 包含 trigger_reason/events_discarded]
- [x] CHK029 — 当 `audit_enabled: false` 时, "审计通道禁止采样"的约束是否自动失效? 规格是否定义了 audit 禁用时的替代防护措施? [Resolved, data-model.md §Validation Rule 9: audit 禁用时约束不适用; 非替代防护措施, 因禁用是管理员有意选择]

## Non-Functional Requirements(非功能需求)

- [x] CHK030 — 事件序列化和发射的性能预算是否定义? 控制循环主路径上每个事件的处理延迟 p99 上限是微秒级还是毫秒级? 没有量化阈值则无法判断序列化开销是否可接受. [Resolved, research.md R008(plan.md 引用): 单次 emit p99 < 10µs; 完整四通道扇出 p99 < 100µs]
- [x] CHK031 — Edge Cases 提到"tracing 与 metrics 的标签基数必须有文档硬上限". 该硬上限的具体数值 (如每个 span 不超过 10 个标签, 每个标签不超过 100 个唯一值) 和超限时的处理策略 (拒绝写入? 截断? 告警?) 是否已定义? [Resolved, research.md R006: 每个 span ≤ 10 标签, 每个标签键 ≤ 100 唯一值; 超限时拒绝写入并告警(不 panic)]
- [x] CHK032 — Event stream (broadcast channel) 的容量上限是否已定义? 当 subscriber 消费速度持续低于生产速度时, channel 是阻塞发送方、丢弃最旧事件、还是返回错误? [Resolved, data-model.md §Backpressure Behavior: 默认 256; AlertAndBlock 阻塞生产者; SampleAndAudit 按采样率丢弃]
- [x] CHK033 — 事件 journal 的固定容量已由 002 切片配置 (`event_journal_capacity`). 当 journal 满时是覆盖最旧事件还是停止写入? 行为是否与背压策略一致? [Resolved, data-model.md §Validation Rule 10 + src/journal/ring.rs: 覆盖最旧事件(ring buffer), dropped_count 跟踪丢弃数; 与背压策略独立]

## Dependencies & Assumptions(依赖与假设)

- [x] CHK034 — 规格声明强依赖 `specs/005-1-failure-policy-reliability/contracts/pipeline-and-events.md`. 该契约的 `SupervisorEvent` 事件变体集合是否已冻结? 如果 005-1 仍在迭代, 本切片的事件 schema 是否与其保持同步? [Resolved, research.md R009: 本切片扩展 005-1 契约; tasks.md T002 读取 005-1 契约进行 diff 分析]
- [x] CHK035 — 假设"调用方负责装配具体 OpenTelemetry 导出栈". 监督器是否提供默认的 no-op 实现, 使得未配置导出栈时系统不会 panic 或报错? [Resolved, Cargo.toml 依赖 tracing-subscriber; tracing crate 默认行为是 no-op(无 subscriber 时静默丢弃); 系统不会 panic]
- [x] CHK036 — 假设"correlation id 的生成策略由计划阶段 data-model.md 冻结". 但 006-5 目前没有 plan.md 和 data-model.md. correlation id 的生成算法是否需要在当前 spec 中先行固定, 以免后续切片依赖断裂? [Resolved, research.md R001(UUID v4) + data-model.md §CorrelationHandle + contracts/correlation-api.md 已完全冻结]
- [x] CHK037 — 规格提到"schema id 抬版本时必须附带人类可读迁移脚注段落". schema 版本的管理责任 (谁批准版本晋升? 版本号格式? 迁移脚注存放位置?) 是否已明确? [Resolved, data-model.md §Validation Rule 8: 版本晋升由 tech lead 在 PR 中批准; 脚注存放于 CHANGELOG.md]

## Ambiguities & Conflicts(歧义与冲突)

- [x] CHK038 — 规格全文多处使用 `sample_ratio(字段名示例)`, `gap_alarm(字段名示例)`, `latency_inject_ms(字段名示例)`, `event_variant(字段名示例)`, `investigation_blocked(字段名示例)`. 这些字段名标注了"示例", 但发布门禁要求字段名稳定. 是否需要在当前规格中为这些字段提供"建议字段名"作为占位, 还是全部推给 plan 阶段? [Resolved, contracts/typed-event-schema.md §3.2 和 data-model.md 已冻结实际字段名 (snake_case); spec 中的示例标注不影响契约]
- [x] CHK039 — US2 的验收场景要求"返回数组必须按时间顺序把上述五个阶段都盖住, 或在缺口位置抛出可读 structured error". "按时间顺序"是指 wall clock 顺序还是 monotonic clock 顺序? 如果系统时钟在记录期间发生跳变 (NTP 校时), 排序是否仍然可靠? [Resolved, contracts/typed-event-schema.md §4.1: 排序优先使用 monotonic_nanos 避免 NTP 跳变干扰]
- [x] CHK040 — FR-002 要求"journal, tracing, metrics 三类出口必须消费同一套结构化字段释义". 但 journal 是固定容量缓冲区, tracing 是 span/event, metrics 是时序数据点. 三类出口的持久化语义和查询能力完全不同 — "同一套字段释义"是指字段名相同, 还是也包括数据类型和序列化格式相同? [Resolved, contracts/typed-event-schema.md §1 定义统一字段字典; research.md R002: 同一套字段名和类型在三通道中保持一致的语义和序列化格式]

## Constitution Compliance(宪章合规)

- [x] CHK041 — Constitution Alignment 要求"事件模型要把监督状态迁移的每一条弧都盖上". 该要求与 FR-001 的"每一次合法状态迁移"一致. 但控制循环的状态迁移图是否以机器可读格式 (如 DOT 图或枚举定义) 存在于 specs/ 或 src/ 中? [Resolved, data-model.md §Scope & Boundaries: What 枚举在 src/event/payload.rs 中, 是 Rust 类型系统第一等成员, 可被 cargo doc 和 IDE 解析]
- [x] CHK042 — Module ownership 要求"SupervisorEvent 的 schema 定义集中在 observe 模块名下维护". 当前是否已经有一个集中的事件 schema 定义文件 (如 `src/event/payload.rs` 或 `src/observe/schema.rs`) 被明确指定为 schema 权威来源? [Resolved, spec.md §Module ownership 已修正: schema 集中在 src/event/ 模块; src/observe/ 负责扇出和背压, 不持有 schema]
- [x] CHK043 — 术语格式:"英文术语必须写成 `English(中文说明)`". 规格中使用了 `gap_alarm(字段名示例)` 和 `investigation_blocked(字段名示例)` 等标注"示例"的字段名. 这些"示例标注"是否符合术语格式要求, 还是需要在冻结字段名后更新? [Resolved, contracts/typed-event-schema.md 使用实际字段名; spec 中的示例标注因标注了"示例"字样而符合"禁止把示例字段名写成最终实现承诺"的规则]

## Schema & Versioning(事件方案与版本化)

- [x] CHK044 — SupervisorEvent 的 schema 版本号格式 (SemVer? 单调递增整数? 日期戳?) 是否已在规格或契约中冻结? 不同版本之间如何区分和路由? [Resolved, research.md R005: 单调递增 u64; 不同版本通过 schema_id 字段区分; 向后兼容期: 当前版本 + 上一版本]
- [x] CHK045 — 规格要求 "schema id 抬版本时必须附带人类可读迁移脚注段落". 该脚注的存放位置 (事件定义旁? 独立 MIGRATION.md? CHANGELOG 条目?) 和必需内容 (变更多少字段? 兼容性类别? 迁移指南?) 是否已规定? [Resolved, data-model.md §Validation Rule 8: CHANGELOG.md, 含变更摘要/字段列表/兼容性类型]
- [x] CHK046 — 当引入新 schema 版本时, 旧版本事件是否需要继续在 journal 中保留? 保留期限或容量策略是否与 002 切片的 `event_journal_capacity` 协调一致? [Resolved, research.md R005: 当前版本 + 上一版本同时保留]
- [x] CHK047 — 事件 schema 的权威注册中心在哪里? 是 Rust enum 定义 + serde 派生, 还是独立的 JSON Schema / protobuf / flatbuffers IDL? 不同的序列化框架影响字段字典的稳定策略. [Resolved, contracts/typed-event-schema.md §Alias mapping: Rust enum(What) + serde JSON; 权威来源是 src/event/payload.rs]
- [x] CHK048 — 如果采用 Rust enum 作为 schema 权威来源, 如何处理跨 crate 的事件类型引用? 是否需要在 `rust-tokio-supervisor` 中定义公共 event crate? [Resolved, 单 crate 项目, 不需要公共 event crate; 所有事件类型在同一 crate 内可见]

## Serialization & Error Handling(序列化与错误处理)

- [x] CHK049 — SupervisorEvent 的序列化格式 (JSON? MessagePack? 自描述二进制?) 是否在规格中冻结? 不同出口 (journal/tracing/metrics/audit) 是否需要使用统一格式? [Resolved, contracts/typed-event-schema.md §4: 默认 JSON(serde); 所有出口使用同一格式]
- [x] CHK050 — 当事件序列化失败 (如字段格式非法、嵌套深度超限、序列化器 OOM) 时, 控制循环的默认行为 (panic? 跳过并告警? 重试?) 是否已明确定义? [Resolved, data-model.md §Validation Rule 6: 不得 panic, 记录结构化错误到 stderr 并继续执行]
- [x] CHK051 — 序列化失败的审计记录是否包含原始事件的关键标识 (至少 child_id + event_variant), 使得追溯不因序列化失败而完全断裂? [Resolved, data-model.md §Validation Rule 6: 审计记录必须包含 child_id 和 what 变体名]
- [x] CHK052 — 反序列化旧版本事件的兼容性策略: 当 journal 回放时遇到未知字段是静默忽略还是严格报错? 该行为是否在整个事件路径上一致? [Resolved, contracts/typed-event-schema.md §4.3: 默认静默忽略; 测试场景可使用 deny_unknown_fields 严格模式]

## Backpressure Quantification(背压量化)

- [x] CHK053 — US3 的 "二者只能择其一" 选择是否已在配置 schema 中体现? 配置键名、可选枚举值、默认值是否已在 `config/default.toml` 或等效位置写明? [Resolved, data-model.md §BackpressureConfig: strategy(AlertAndBlock/SampleAndAudit), 默认 AlertAndBlock; 配置项在 spec 层面冻结, config/default.toml 待实现阶段写入]
- [x] CHK054 — 背压触发阈值是否已量化? (缓冲区占用百分比? 事件排队延迟毫秒数? 订阅者回调执行耗时?) 如果没有量化, "明显变慢" 无法客观判定. [Resolved, data-model.md §BackpressureConfig: warn_threshold_pct(80%), critical_threshold_pct(95%)]
- [x] CHK055 — 当选择 "告警 + 顶住背压" 策略时, 告警的严重级别 (warn/error/critical) 和告警通道 (tracing event? metrics counter? health check degradation?) 是否已指定? [Resolved, data-model.md §Backpressure Behavior: 软阈值 -> warn tracing event; 硬阈值 -> error tracing event + BackpressureDegradation]
- [x] CHK056 — 当选择 "采样 + audit 记录" 策略时, 采样率的配置范围 (0.0–1.0? 固定步长?) 和动态调整策略 (是否支持自适应采样?) 是否已定义? [Resolved, data-model.md §Backpressure Behavior + contracts/typed-event-schema.md §5: 范围 [0.01, 1.0], 步长 0.01, 默认 0.5]
- [x] CHK057 — 背压状态下的 "保护性降级停机分支" 是否明确写明了降级条件 (连续 N 次背压告警? 缓冲区溢出?), 降级范围 (单个 subscriber? 整个 event bus?), 以及恢复机制 (自动恢复还是需人工介入)? [Resolved, data-model.md §Backpressure Behavior: 触发硬阈值时降级; 范围仅单个 subscriber; 连续 3 窗口低于软阈值自动恢复]

## Correlation Tracking Completeness(关联追踪完备性)

- [x] CHK058 — US2 要求 correlation id 覆盖 spawn → ready → failure decision → restart attempt → shutdown 五段. 但控制循环的实际迁移弧可能多于五类 (如 health_check_passed, health_check_failed, paused, quarantined, budget_exhausted). 这些弧段是否也需纳入 correlation id 追踪? [Resolved, data-model.md §State Transitions 扩展迁移弧包含 health/pause/resume/quarantine; CorrelationHandle 通过 events 向量存储所有关联事件]
- [x] CHK059 — correlation id 在跨事件出口时的传播机制: 是显式嵌入每个事件的载荷字段, 还是通过 tracing span context / 指标标签隐式传递? 两种做法的语义一致性是否已评估? [Resolved, research.md R002: 三通道同时携带(嵌入 event payload + tracing span label + metrics label)]
- [x] CHK060 — 当查询 API 收到不存在的 correlation id 时, 返回值类型是否已定义? 返回空数组 vs 抛出结构化错误 vs 返回带 `not_found` 标识的结果 — 哪种行为与 US2 的 "gap_alarm 级别可观测条目" 一致? [Resolved, contracts/correlation-api.md §2: CorrelationNotFound 结构化错误]
- [x] CHK061 — 时间戳在跨阶段排序时的可靠性: 如果系统时钟在事件记录期间发生 NTP 跳变, 排序是否改用 monotonic clock? 规格是否显式要求使用单调时钟以避免排序混乱? [Resolved, contracts/typed-event-schema.md §4.1: 排序优先使用 monotonic_nanos, 仅当无法比较时回退 unix_nanos]
- [x] CHK062 — 当多次重启产生的 correlation id 链条因日志轮转或 journal 容量限制而截断时, 查询 API 的返回是标记 "truncated" 还是静默返回部分结果? [Resolved, contracts/correlation-api.md §2: CorrelationTruncated 结构化错误]

## Event Bus & Channel Architecture(事件总线与通道架构)

- [x] CHK063 — 事件通道 (broadcast channel) 的容量上限是否已在配置或规格中定义? 容量耗尽时的策略 (阻塞生产者? 丢弃最旧事件? 溢出 panic?) 是否与 US3 的背压策略一致? [Resolved, data-model.md §Backpressure Behavior: 默认 256; AlertAndBlock 时阻塞, SampleAndAudit 时按采样率丢弃]
- [x] CHK064 — 多个事件订阅者之间的故障隔离: 一个慢订阅者导致背压触发时, 其他正常订阅者是否也受影响? 规格是否要求提供 per-subscriber 独立缓冲区? [Accepted, data-model.md §Scope & Boundaries: 本切片不实现 per-subscriber 隔离; 可后续切片解除]
- [x] CHK065 — audit 通道的 "单独 channel" 在事件总线架构中如何实现? 是独立的 tokio::broadcast 实例, 还是同一 channel 的优先级队列? 该 channel 的容量和消费者线程模型是否已设计? [Resolved, research.md R010: 独立 tokio::sync::broadcast; 容量由 audit_channel_capacity 控制(默认 1024)]
- [x] CHK066 — 当 audit channel 本身成为瓶颈时, "禁止采样" 的承诺是否还能维持? 规格是否定义了 audit channel 的保护机制 (如独立线程、有界背压、健康检查)? [Resolved, data-model.md §Scope & Boundaries: audit channel 满时阻塞生产者(不采样); 建议生产环境配置 ≥ 1024 容量]

## Performance & Resource Budgets(性能与资源预算)

- [x] CHK067 — 控制循环主路径上每个事件的生产-发射延迟的 p99 上限是否已量化? 如果没有预算, 无法判断引入结构化序列化是否引入不可接受的延迟. [Resolved, research.md R008/plan.md: 单次 emit p99 < 10µs; 四通道扇出 p99 < 100µs]
- [x] CHK068 — tracing 与 metrics 的标签基数硬上限是否已定义? 具体数值 (如每个 span ≤ 10 个标签, 每个标签键 ≤ 100 个唯一值) 和超限处理策略 (拒绝? 截断? 告警?) 是否已写入规格? [Resolved, research.md R006: 每个 span ≤ 10 标签, 每个标签键 ≤ 100 唯一值; 超限时拒绝写入并告警]
- [x] CHK069 — 事件通道的内存预算: 在最大背压场景下, 未消费事件占用的内存上限是否已估算并在文档中声明? [Resolved, data-model.md §Backpressure Behavior: 单事件 ~512 字节, 256 容量 × 4 通道 ≈ 512KB]
- [x] CHK070 — SC-002 的 "5 分钟" 是否已拆分为系统响应时间 (查询 API 延迟) 和人工操作时间 (检索和拼接)? 如果该指标用于 SLO, 需要明确哪一部分计入计时. [Resolved, spec.md SC-002: 5 分钟为 API 响应时间, 不含人工操作]

## Integration & Dependency Alignment(集成与依赖对齐)

- [x] CHK071 — 规格依赖 `specs/005-1-failure-policy-reliability/contracts/pipeline-and-events.md`. 该契约中 SupervisorEvent 的事件变体集合是否与本切片所需变体一一对应? 如果 005-1 缺少本切片需要的变体 (如 `budget_denied`, `generation_fenced`), 是本切片扩展契约还是需要 005-1 先补充? [Resolved, research.md R009: 本切片扩展 005-1 契约; tasks.md T002 要求做 diff 分析]
- [x] CHK072 — 002 切片的 `event_journal_capacity` 配置项是否与本切片的事件生产速率兼容? 在最大负载下 journal 满时的行为 (覆盖最旧 vs 停止写入) 是否与 US3 背压策略协调? [Resolved, src/journal/ring.rs: 覆盖最旧(ring buffer), 与背压策略独立; 生产速率兼容性需性能测试验证]
- [x] CHK073 — 004-4 (generation fencing) 产生的 `generation_fenced` 事件是否应纳入 Supervisorevent 类型家族? 该事件的字段字典是否需要在本切片中统一定义? [Resolved, data-model.md §What 枚举: GenerationFenced 已纳入新增变体]
- [x] CHK074 — 假设 "调用方负责装配 OpenTelemetry 导出栈" 是否已通过默认 no-op 实现验证? 当未配置导出栈时, 监督器是否仍能正常启动并输出结构化日志而不 panic? [Resolved, 同 CHK035: tracing 默认 no-op; 系统不会 panic]

## Testability & Release Gate(可测试性与发布门禁)

- [x] CHK075 — US1 的 "穷尽覆盖" 测试: 是否需要一个权威的枚举清单列出所有 SupervisorEvent 变体, 并在 CI 中验证该清单未被遗漏? 如果枚举定义在 Rust 源码中, 测试如何保证 `#[non_exhaustive]` 声明不会导致漏测? [Resolved, plan.md §Constraints: #[non_exhaustive] 下测试改为遍历辅助函数或序列化 roundtrip, 而非直接 match]
- [x] CHK076 — SC-001 的 "95%" 度量: 分母 (迁移弧总数) 的权威来源是哪里? 是 Rust 枚举成员数量, 还是 spec 中的状态转移图? 不同来源可能导致不同的度量结果. [Resolved, spec.md SC-001: 权威来源为 src/event/payload.rs 中 What 枚举定义]
- [x] CHK077 — SC-002 的 "97 条" 抽样方法: 是 CI 中自动随机抽样, 还是人工定期审计? 样本集是否来自生产环境的失败复盘记录? 如果 100 条样本中某类失败场景占比过高, 是否需要按类型分层以确保代表性? [Resolved, spec.md SC-002: CI 自动化按 child_id 分层随机抽样, 样本来自近 24h 生产失败记录]
- [x] CHK078 — 发布门禁要求 "字段名不能停留在示例状态". 规格中标记为 `(字段名示例)` 的字段名是否有一份明确的冻结时间表或冻结条件? 发布前是否需要一个验收步骤检查所有示例字段名已被替换? [Resolved, contracts/typed-event-schema.md §3.3: 字段名从 Draft 状态起冻结; 变更需更新契约 + 递增 schema_id + CHANGELOG 记录]
- [x] CHK079 — 事件 schema 的向后兼容性验证是否纳入 CI? 是否存在契约测试在 PR 合并前检测字段名更改或类型变更? [Resolved, 已纳入 tasks.md T032(全量测试) 和 CHK078 的字段冻结规则; CI 契约测试的具体配置在实现阶段补充]
- [x] CHK080 — 背压策略的配置是否纳入发布门禁的配置验证步骤? 是否要求每次发布时确认默认配置文件中背压策略字段存在且值合法? [Resolved, 配置验证在 tasks.md T027(配置加载)中覆盖; 发布门禁的具体 CI 步骤在实施阶段补充]

## Configuration Management(配置管理)

- [x] CHK081 — 事件相关的全部配置项 (背压策略选择、阈值、采样率、audit channel 容量、标签基数上限) 是否已在配置 schema 中定义并有默认值? [Resolved, data-model.md §BackpressureConfig + §Backpressure Behavior: 背压配置已定义; 标签基数上限(10 span/100 值)在 research.md R006 中定义, 未纳入运行时配置 schema 按设计决策接受]
- [x] CHK082 — 当配置变更 (如从 "告警" 切换到 "采样") 在运行时生效还是需要重启? 如果是运行时生效, 配置热加载的安全性验证是否已指定? [Resolved, data-model.md §配置热加载: 本切片不支持运行时热加载; 配置更改需重启 supervisor; 可后续切片解除]
- [x] CHK083 — 不同环境 (开发/预发/生产) 的事件配置基线是否在规格或部署文档中给出推荐值? 背压阈值在不同环境是否需要差异化? [Resolved, data-model.md §Deployment Recommendations: 三套基线配置表(开发/预发/生产)]

## Observability Audit Trail(可观测性审计轨迹)

- [x] CHK084 — 采样事件时 audit 记录是否至少包含: 被采样事件数量、采样比例、采样触发原因、触发时间窗口? 这些字段是否已在规格中定义? [Resolved, data-model.md AuditRecorded 变体扩展: 含 correlation_id, trigger_reason, events_discarded, sample_ratio]
- [x] CHK085 — 当背压导致保护性降级停机时, 停机原因的 audit 记录是否足够重建现场 (含触发订阅者标识、背压度量峰值、所选策略)? [Resolved, data-model.md AuditRecorded 变体已包含 trigger_reason/correlation_id; BackpressureDegradation 包含 subscriber/buffer_peak_pct/strategy; 组合两者可重建现场]
- [x] CHK086 — 非采样事件 (高风险改写事件) 的 audit 记录是否包含 correlation id, 使得 audit 行可直接与 US2 的查询 API 结果关联? [Resolved, data-model.md AuditRecorded 变体已包含 correlation_id: Uuid 字段]

## Notes(说明)

- 完成检查项后使用 `[x]` 标记.
- 评论或发现可以直接写在相关检查项下.
- 需要时链接相关资源或文档.
- 检查项必须按顺序编号, 方便引用.
- 追加部分从 CHK044 到 CHK086, 覆盖 Schema & Versioning, Serialization & Error Handling, Backpressure Quantification, Correlation Tracking Completeness, Event Bus & Channel Architecture, Performance & Resource Budgets, Integration & Dependency Alignment, Testability & Release Gate, Configuration Management, Observability Audit Trail 等维度.
