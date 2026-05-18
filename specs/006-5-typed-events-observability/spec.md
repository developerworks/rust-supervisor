# Feature Specification (功能规格): 类型化事件与端到端可追溯闭环

**Feature Branch (功能分支)**: `[006-5-typed-events-observability]`
**Created (创建日期)**: 2026-05-17
**Updated (更新日期)**: 2026-05-19
**Status (状态)**: Accepted (已接受)
**Input (输入)**: 本规格对应第三序列里程碑: 把控制循环里的字符串事件全部替换成 typed SupervisorEvent(类型化监督器事件), 并接入 journal(事件日志), tracing(链路追踪), metrics(指标), audit(审计). 任意一次任务失败都能通过 correlation id(关联标识) 追踪到启动, 就绪, 失败, 策略决策, 重启, 关闭全过程.

## Dependency Note (依赖说明)

与本切片耦合的失败流水线类型化条目见 specs/005-1-failure-policy-reliability/contracts/pipeline-and-events.md. 本节补齐控制循环剩余弧段, 并把 event subscriber(事件订阅者) 消费太慢时背压的处理写成明示条款.

## User Scenarios & Testing (用户场景和测试) _(mandatory (必填))_

### User Story 1 (用户故事一) - 类型优于模糊段落 (Priority (优先级): P1)

on-call(待命工程师) 需要检索告警时按 SupervisorEvent(监督器事件) 变体字段聚合, 而不是靠正则匹配几句英文摘要.

**Why this priority (为什么是这个优先级)**: 字符串检索换个版本就很容易悄悄漏报.

**Independent Test (独立测试)**: 给控制循环每段迁移打上唯一的 enum(枚举) 成员断言. 跑一次冒烟套件验证穷尽覆盖.

**Acceptance Scenarios (验收场景)**:

1. **Given (假设)** 控制循环发出了 spawn_failed(拉起失败) 与 budget_denied(预算拒绝) 两类事件, **When (当)** 下游工具消费 journal(事件日志) JSON 流, **Then (则)** 必须能只靠 event_variant(字段名示例, 实际字段名交给计划书附录约束) 这类机器字段完成 filter(筛选), 严禁依赖自然语言 message(消息) 段落.

### User Story 2 (用户故事二) - correlation id 链路不断 (Priority (优先级): P1)

复盘负责人需要导出一张按时间排序的同 ID 事件表, 覆盖 spawn(拉起), ready(就绪), 失败判决, 重启尝试与 shutdown(关停) 五段.

**Why this priority (为什么是这个优先级)**: 片段缺失会把线上结论带进沟里.

**Independent Test (独立测试)**: 构造多次重启脚本后导出 CorrelationHandle(关联句柄) 查询 API. 比对缺失弧计数.

**Acceptance Scenarios (验收场景)**:

1. **Given (假设)** 某 child id(子任务标识) 在 24 小时内多次重启, **When (当)** 复盘脚本提交根 correlation id(关联标识), **Then (则)** 返回数组必须按时间顺序把上述五个阶段都盖住, 或在缺口位置抛出可读 structured error(结构化错误).

### User Story 3 (用户故事三) - 慢订阅者不致悄悄丢事实 (Priority (优先级): P2)

平台提供者必须写明 event subscriber(事件订阅者) 明显变慢时, 背压要么顶住控制循环并能告警, 要么按策略采样并在 audit(审计) 记下采样比例. 二者只能择其一写在默认配置文件中.

**Why this priority (为什么是这个优先级)**: 默认悄悄丢掉事件会把合规审计架空.

**Independent Test (独立测试)**: 人为限速订阅回调, 测量缓冲区水位 metrics(指标) 是否在阈值触发告警, 对照 audit(审计) 行是否写明 sample_ratio(字段名示例).

**Acceptance Scenarios (验收场景)**:

1. **Given (假设)** 订阅回调延迟被人为拉到 latency_inject_ms(字段名示例) 上限, **When (当)** 缓冲区占用越过契约阈值, **Then (则)** metrics(指标) 必须抛出可告警背压计数, 或者控制循环进入文档写明的保护性降级停机分支. 二者只能实现其一并与默认规格一致.

### Edge Cases (边界情况)

- 当结构化序列化开销超过 SLO(服务等级目标) 上限时可以提供采样开关, 但 audit(审计) 承载的高风险改写事件必须落在单独 channel(通道) 维持全量语义.
- tracing(链路追踪) 与 metrics(指标) 的标签基数必须有文档硬上限, 并在超限策略触发时写入告警事件.

## Requirements (需求) _(mandatory (必填))_

### Functional Requirements (功能需求)

- **FR-001**: 控制平面必须把原先主要靠自由字符串当契约的监督口径改成 SupervisorEvent(监督器事件) 类型家族当家. 每一次合法的状态迁移至少对应到一个稳定的 schema id(方案标识) 版本化事件变体.
- **FR-002**: journal(事件日志), tracing(链路追踪), metrics(指标) 三类出口必须消费同一套结构化字段释义. audit(审计) 通道承载的高风险改写动作默认禁止采样, 除非规格声明例外清单.
- **FR-003**: 系统必须为每一次被承认的监督运行下发或继承 correlation id(关联标识), 使其在多次重启之间保持稳定. 查询 spawn(拉起) 到 shutdown(关停) 任一阶段缺失时必须产出 gap_alarm(字段名示例) 级别的可观测条目或可解析缺口错误.

### Key Entities (关键实体) _(涉及数据时填写)_

- **SupervisorEvent(监督器事件)**: 在给定 schema id(方案标识) 版本号之下自成一体的结构化事件字段集合, 字段字典对外保持稳定.
- **CorrelationHandle(关联句柄)**: 串联跨阶段记录的 correlation id(关联标识) 包装类型, 对人 API 暴露.
- **BackpressureConfig(背压配置)**: 包含背压策略选择、告警/降级阈值、采样率、audit 通道容量等字段的配置结构.
- **BackpressureStrategy(背压策略)**: 枚举 `AlertAndBlock`(告警并阻塞, 不丢事件) 和 `SampleAndAudit`(采样并记录审计).

## Constitution Alignment (宪章对齐) _(mandatory (必填))_

### Supervision Contract (监督契约)

- **Lifecycle impact (生命周期影响)**: 事件模型要把监督状态迁移的每一条弧都盖上. 不许偷偷删掉某一类迁移.

### Rust Boundary and Observability Requirements (Rust 边界和可观察性需求)

- **Module ownership (模块所有权)**: SupervisorEvent(监督器事件) 的 schema(方案) 定义集中在 `src/event/` 模块名下维护. `src/observe/` 下的 pipeline 和 fairness 模块负责扇出和背压, 不持有事件 schema 定义.
- **Compatibility exports (兼容导出)**: None (无).
- **Diagnostics (诊断)**: schema id(方案标识) 抬版本时必须附带人类可读迁移脚注段落.
- **Dependency impact (依赖影响)**: 不适用, 除非 OpenTelemetry(开放遥测) 绑定被接受写进默认二进制.

### Chinese Writing (中文写作)

- **Writing language (写作语言)**: 本文档必须使用中文.
- **Term format (术语格式)**: 英文术语必须写成 English(中文说明).
- **Forbidden style (禁止风格)**: 禁止全角标点, 禁止把示例字段名写成最终实现承诺却不标注示例字样.

## Success Criteria (成功标准) _(mandatory (必填))_

### Measurable Outcomes (可衡量结果)

- **SC-001**: 控制循环迁移弧清单(权威来源为 `src/event/payload.rs` 中 `What` 枚举定义)里至少 95% 的行以 SupervisorEvent(监督器事件) 稳定变体字段作为主要契约字段, 而不是只靠自由文本 message(消息). 并行附带的人类可读 message(消息) 段落不计入违背比例.
- **SC-002**: 通过 CI 中的自动化复盘脚本, 从生产环境最近 24 小时的失败记录中按 child_id 分层随机抽取 100 条样本. 至少 97 条能在 5 分钟内只靠 correlation id(关联标识) 从查询 API 返回完整事件段落(计时以 API 响应时间计, 不含人工操作). 其余样本必须标记 investigation_blocked(字段名示例) 并写明缺口类型枚举.

## Assumptions (假设)

- 调用方负责装配具体 OpenTelemetry(开放遥测) 或等价导出栈. 监督器只保证字段字典稳定.
- correlation id(关联标识) 的生成策略由计划阶段 data-model.md 冻结, 本规格只约束其存在性与可查询性.
- 背压策略默认配置为 `AlertAndBlock`(告警并阻塞, 不丢事件), 平台提供者可改为 `SampleAndAudit`(采样并记录审计); 此二选一的选择在默认配置文件中固化后不得在运行时由控制命令动态切换.
- audit(审计) 通道与普通 event(事件) 通道物理隔离: audit(审计) 通道有独立容量配置, 在背压触发时优先保障 audit(审计) 通道全量写入, 普通 event(事件) 通道按策略采样或阻塞.
