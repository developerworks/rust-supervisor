# Feature Specification (功能规格): 生产级重启策略与分组隔离观测

**Feature Branch (功能分支)**: `[006-4-restart-policy-production]`
**Created (创建日期)**: 2026-05-17
**Updated (更新日期)**: 2026-05-19
**Status (状态)**: Accepted (已接受)
**Input (输入)**: 本规格对应第二序列里程碑: 接入 restart budget(重启预算), meltdown fuse(熔断器), group strategy(分组策略), escalation policy(升级策略), backoff jitter(退避抖动). 快速失败不会造成无限重启风暴, group(分组) 故障不会误伤无关任务, critical child(关键子任务) 和 optional child(可选子任务) 有不同处理, 所有策略决策都有事件和指标.

## Dependency Note (依赖说明)

本切片强依赖:

- `specs/005-1-failure-policy-reliability/spec.md` — 失败流水线入口 (已完成)
- `specs/005-2-work-role-defaults/spec.md` — 工作角色默认策略 (已完成)
- `specs/006-3-lifecycle-shutdown-realism/spec.md` — ChildSlot 基础设施 (已完成, 本切片基于其 slots 架构)

若条文字面重复, 本条以度量字段完备性与分叉观测补齐为主.

## User Scenarios & Testing (用户场景和测试) _(mandatory (必填))_

### User Story 1 (用户故事一) - 快速失败不致风暴 (Priority (优先级): P1)

SRE(站点可靠性工程师) 需要子任务在同一窗口里连着崩溃 10_000(一万次) 时, 仍能看出重启节拍被 restart budget(重启预算) 与 backoff jitter(退避抖动) 压住. 监督器控制线程不能因为等某个故障子任务, 就把别的就绪任务长期晾在一边拿不到调度机会.

**Why this priority (为什么是这个优先级)**: 重启风暴是线上第一类点火源.

**Independent Test (独立测试)**: 输入固定失败波形脚本. 统计每分钟 effective restart attempts per minute(每分钟有效重启尝试), 与 YAML 中的预算曲线 CSV 对照.

**Acceptance Scenarios (验收场景)**:

1. **Given (假设)** 单次失败回路触发间隔低于配置文件里的抖动下限, **When (当)** 失败波形重复滚动 60 秒, **Then (则)** 实测重启间隔不得低于文档给出的下限曲线, 且 typed event(类型化事件) 载荷里附带本轮预算计数快照.
2. **Given (假设)** 预算令牌在快速失败期间耗尽, **When (当)** 故障停止超过 recovery(恢复) 窗口, **Then (则)** 令牌逐步恢复至配置上限, 下一个合法重启请求通过预算检查并正常调度, typed event(类型化事件) 标记 `BudgetVerdict::Granted`.

### User Story 2 (用户故事二) - 分组故障止步于组边界 (Priority (优先级): P1)

拓扑结构需要 group(分组) A 触发熔断后, group(分组) B 内的 optional child(可选子任务) 在线时长不因 A 的风暴掉到对照实验基线以下, 除非配置图里写明跨组 dependency edge(依赖边).

**Why this priority (为什么是这个优先级)**: 单监督器实例常被多租户拼装. 边界不清楚会直接造成无辜租户停机.

**Independent Test (独立测试)**: 双分组对照实验环境里统计 B 侧 uptime(在线时间) 比例, 并与隔离对照组比对 24 小时滑动窗口.

**Acceptance Scenarios (验收场景)**:

1. **Given (假设)** 仅在 group(分组) A 内注入熔断触发条件, **When (当)** meltdown fuse(熔断器) 生效, **Then (则)** group(分组) B 的熔断计数器(`GroupCounter`) 不升, `group_fuse_active("B")` 返回 `false`, 除非拓扑配置文件列出跨组依赖并被加载器校验通过.

### User Story 3 (用户故事三) - critical 与 optional 分叉可观测 (Priority (优先级): P2)

产品负责人需要在事后复盘导出的事件 CSV 与 metrics(指标) 抓取结果里一眼区分关键子任务的升级路径与可选子任务的降噪路径.

**Why this priority (为什么是这个优先级)**: 分叉不可观测就无法写值守脚本触发条件.

**Independent Test (独立测试)**: 对两条路径分别抓取最新 100 条事件记录与同一时间窗 metrics(指标) 标签集合. 核对字段基数差异.

**Acceptance Scenarios (验收场景)**:

1. **Given (假设)** 同一类底层故障注入脚本触发失败, **When (当)** 目标 child(子任务) 一行标记 critical(关键) 另一行标记 optional(可选), **Then (则)** escalation policy(升级策略) 分叉必须在 typed event(类型化事件) 与 metrics(指标) 两条通道各自至少多出 3 个互不混淆的诊断键.

### Edge Cases (边界情况)

- 当一个 critical child(关键子任务) 同时挂在两个 group(分组) 且两边 policy(策略) 冲突时, tie-break(平局裁决) 规则见下表:

  | 冲突场景                                           | 裁决规则                                                | 审计记录                                               |
  | -------------------------------------------------- | ------------------------------------------------------- | ------------------------------------------------------ |
  | 同 child 在两个 group 中 severity 不同             | 以更高的 severity 为准 (critical > standard > optional) | 发射 `EscalationBifurcated` 携带 `tie_break_reason`    |
  | 同 child 在两个 group 中 budget 不同               | 以更严格的 budget 为准 (max_burst 更小者)               | 发射 `BudgetExhausted` 携带 `budget_source_group`      |
  | 同 child 在一组熔断另一组未熔断                    | 已熔断组的结果优先 (传播至该 child)                     | 发射 `GroupFuseTriggered` 携带 `propagated_from_group` |
  | 同 child 显式 severity 与 group 默认 severity 冲突 | child 级显式声明优先于 group 级默认值                   | 发射 `EscalationBifurcated` 携带 `tie_break_reason`    |

- 当 meltdown(熔断) 与手动 quarantine(隔离) 并发触发时, 人工指令优先于自动熔断裁定, 审计流水记录 `operator_id`(操作者标识) 和配置版本戳.- 当 `FairnessProbe`(公平性探针) 或 `RestartBudgetTracker`(重启预算跟踪器) 内部状态损坏(计数器溢出, 时间戳倒流等)时, 系统发射 `degraded_mode`(降级模式) 事件并跳过受影响子系统的检查: budget 损坏时跳过预算限流(回退到仅 backoff 控制), fairness 损坏时跳过饥饿检测. 降级期间所有 child 仍受 meltdown(熔断) 和 backoff(退避) 保护.
- `CorrelationId`(关联标识) 在故障首次进入评估管线时生成(即 `record failure window` 阶段), 跨 budget → meltdown → backoff → escalation 全阶段传递, 并在该次故障链路的所有事件发射完毕后退役. 跨子任务重启时 CorrelationId 不保持(每次故障链路独立). CorrelationId 使用 UUID v4 生成, 碰撞概率可忽略(在 10_000 次/秒的故障注入速率下, 连续运行 100 年碰撞概率 < 10^-12). 多个 child 在同一纳秒触发故障时各自获得独立 CorrelationId, 不共享.
- 策略配置不支持运行时热更新(hot-reload). 修改 budget, group 依赖边或 severity 映射需重启监督器实例. 重启后令牌计数和熔断状态从配置初始值重新开始. 从旧版(无 restart budget)升级到新版(含 restart budget)时, 旧版配置文件中缺失的 budget 字段在加载时使用内置安全默认值填充(`window=60s, max_burst=10, recovery_rate_per_sec=0.5`), 不拒绝启动.- 当 optional child(可选子任务) 抖动失败时, backoff jitter(退避抖动) 参数必须打散重启节拍(随机系数范围 [0.5×base_delay, 1.5×base_delay]), 避免出现同步 thundering herd(惊群).

## Requirements (需求) _(mandatory (必填))_

### Functional Requirements (功能需求)

- **FR-001**: 系统必须把 restart budget(重启预算), meltdown fuse(熔断器), backoff jitter(退避抖动) 按 `budget → meltdown → backoff` 顺序接入 decide action(决定动作) 节拍之前同一评估管线里. 预算不足直接拒绝(不经过熔断与退避), 熔断后不计算退避. 预算耗尽后系统等待 `retry_after_ns` 到期自动重试, 不需人工干预. 在快速失败波形下实测 effective restart attempts per minute(每分钟有效重启尝试) 不得超过文档给出曲线上界的 105%. fairness(公平性) 探针记录在任意连续 10 秒窗口内, 其它就绪监督动作至少获得过调度机会的计数不低于文档阈值. 生产环境中应配置监控告警: 当 `BudgetExhausted` 事件率超过 10 次/分钟时触发告警(表示预算过紧), 连续 5 分钟内此类事件率为 0 时自动解除告警.
- **FR-002**: group strategy(分组策略) 必须保证在未声明跨组 dependency edge(依赖边) 的前提下, 任一 group(分组) 自家熔断或预算耗尽不得把关停的连带后果甩到不相干的 group(分组) 头上. 受影响分组内已处于 running(运行中) 的 child(子任务) 继续运行不受影响, 仅阻止该分组内新重启请求. 一旦发生跨组可见影响, 必须产出指向依赖图节点的 structured diagnostics(结构化诊断) 载荷. `ChildSpec.group` 引用的分组名必须在 `SupervisorSpec.group_configs` 中存在, 配置加载阶段校验不通过则拒绝启动, 不允许运行时兜底处理.
- **FR-003**: critical child(关键子任务) 与 optional child(可选子任务) 的失败处置必须有配置文件里的分叉默认值. Critical(关键) 失败必须触发升级路径(发射 EscalationBifurcated 并上报告警), Optional(可选) 失败走降噪路径(发射事件但不触发告警升级), Standard(默认) 走标准策略路径(按 WorkRole 默认行为). 每一条分叉路径上的预算耗尽与升级裁决都必须 100% 写入 typed event(类型化事件) 与 metrics(指标) 两组管道, 并能被同一个 correlation id(关联标识) 串联. CorrelationId(关联标识) 在评估管线入口生成, 贯穿整个故障链路(budget → meltdown → backoff → escalation), 即使中间某阶段被跳过(如 budget 直接通过, 无 Exhausted 事件)也继续传递至后续阶段事件.

### Key Entities (关键实体) _(涉及数据时填写)_

- **RestartBudgetSnapshot(重启预算快照)**: 某个评估窗口内 consumed(已消耗) 与 remaining(剩余) 重启额度字段的结构化视图.
- **RestartBudgetTracker(重启预算跟踪器)**: 实现滑动窗口 + 令牌桶混合模型, 维护故障时间戳队列和当前令牌计数.
- **GroupFaultBoundary(分组故障边界)**: 描述熔断停在分组叶节点还是沿依赖边上溯的配置切片. 由 `GroupDependencyEdge(分组依赖边)` 和 `GroupIsolationPolicy(分组隔离策略)` 组成.
- **GroupCounter(分组计数器)**: 每个分组独立的熔断故障计数与 fuse_active(熔断激活) 状态.
- **SeverityClass(严重程度分类枚举)**: 配置文件里划分 critical(关键) 与 optional(可选) 及 standard(默认) 的标签轴.
- **FairnessProbe(公平性探针)**: 控制循环主路径上的轻量探针, 检测调度饥饿并产出 `StarvationAlert(饥饿告警)`.
- **GroupConfig(分组配置)**: 定义分组名称、成员子任务列表、独立重启预算配置的结构体. 由 `SupervisorSpec.group_configs` 持有.
- **GroupDependencyEdge(分组依赖边)**: 声明跨组故障传播关系的配置切片. `from_group` 为依赖方, `to_group` 为被依赖方, `propagation` 控制传播策略(Full/EscalateOnly/None). 由 `SupervisorSpec.group_dependencies` 持有.
- **SeverityDefaults(严重程度默认值)**: 按 `WorkRole` 映射默认 `SeverityClass` 的配置表. Service → Critical, Supervisor → Critical, Worker → Standard, Job → Optional, Sidecar → Standard. 由 `SupervisorSpec.severity_defaults` 持有.
- **ChildSpec.severity(子任务显式严重程度)**: 可选字段, 覆盖 `WorkRole` 默认的 `SeverityClass` 映射. 当同时存在 group 级默认值时, child 级显式值优先 (见 tie-break 规则表第 4 行).
- **ChildSpec.group(子任务所属分组)**: 可选字段, 将 child 分配到命名分组, 用于 group-level budget(分组级预算) 和熔断隔离.

## Constitution Alignment (宪章对齐) _(mandatory (必填))_

### Supervision Contract (监督契约)

- **Lifecycle impact (生命周期影响)**: 改动重启节拍与 shutdown(关闭) 耦合节奏, 必须与 006-3 关停切片联合验收.
- **Failure behavior (失败行为)**: 必须写明预算耗尽引起的 escalate(升级) 与普通失败重启之间的分界枚举值.

### Rust Boundary and Observability Requirements (Rust 边界和可观察性需求)

- **Module ownership (模块所有权)**: 策略裁决代码只能落在 policy(策略目录) 与 observe(观测目录) 之间的契约边界内.
- **Compatibility exports (兼容导出)**: None (无).
- **Diagnostics (诊断)**: typed event(类型化事件) 先于自由文本 message(消息) 字段对外承诺稳定性. 当系统发射 `BudgetExhausted(预算耗尽)`, `GroupFuseTriggered(分组熔断触发)`, `EscalationBifurcated(升级分叉)` 三种 typed event 时, observability pipeline(观测流水线) 自动为每个事件生成 `PipelineStageDiagnostic(流水线阶段诊断)`, 包含事件序列号、CorrelationId(关联标识)、`budget_evaluation(预算评估)` 字段(携带预算耗尽退避时长/熔断传播来源/分叉严重程度). `PipelineStageDiagnostic` 中通过 `evaluated: bool` 字段区分"阶段已执行但无事件产生"(`evaluated=true, event=none`)与"阶段因预算通过或熔断跳过"(`evaluated=false, skip_reason: Option<String>`). 诊断记录通过 `TestRecorder.pipeline_stage_diagnostics` 通道可消费.

### Chinese Writing (中文写作)

- **Writing language (写作语言)**: 本文档必须使用中文.
- **Term format (术语格式)**: 英文术语必须写成 English(中文说明).
- **Forbidden style (禁止风格)**: 禁止全角标点, 禁止用形容词堆叠替换可对账阈值百分号写法.

## Success Criteria (成功标准) _(mandatory (必填))_

### Measurable Outcomes (可衡量结果)

- **SC-000**: 所有策略决策路径(预算通过, 预算耗尽, 熔断触发, 升级分叉)均可被 typed event(类型化事件) 日志完整复现, 即任意时刻的监督器状态可从事件流中重建. (定性成功标准)
- **SC-001**: 在 10_000(一万次) 瞬时失败波形下(每次故障间隔 ≤1ms, 单线程注入, 持续 60s 窗口), effective restart attempts per minute(每分钟有效重启尝试) 实测样本不得超过文档曲线包络上界的 105%(曲线公式见 contracts/restart-budget-api.md).
- **SC-002**: 双分组对照实验中在未声明跨组依赖的前提下, 对照组 B 侧初始在线时长 24h, A 侧注入熔断触发故障(频率 1 次/s, 持续 24h). B 侧额外非计划停机时间相对 24h(二十四小时) 对照窗不得超过 5%.
- **SC-003**: typed event(类型化事件) 与 metrics(指标) 针对同一 SupervisorDecision(监督器裁决) 键的一致率抽检样本不低于 98%. "同一键" 指 `(child_id, decision_type, correlation_id)` 三字段组合. 该验证在发布前作为单次门禁执行; 生产环境中由夜间 CI 定时抽检最近 24 小时内的 event/metrics 样本, 持续不达标时阻塞下次发布.

## Assumptions (假设)

- 默认 metrics(指标) 后端由集成方注入适配层. 监督器只暴露稳定的打点字段契约.
- 分组故障隔离依赖运行时拓扑中的 dependency edge(依赖边) 声明, 该声明由 006-6 切片中的配置模型加载.
- 系统时钟为 monotonic clock(单调时钟), 不受 NTP(网络时间协议) 校时或闰秒影响, 预算令牌恢复和熔断倒计时的正确性依赖此假设.
- 单进程内 group(分组) 数量上限为 64, 单个 group 内 child(子任务) 数量上限为 256.
- 滑动窗口中故障时间戳队列内存上限由 `window` 与 `max_burst` 共同约束, 最坏情况内存占用不超过 `max_burst × sizeof(u128)`.
- 全部分组同时触发熔断时, `MeltdownTracker` 的 `group_counters: HashMap<String, GroupCounter>` 在 64 个分组规模下扩容延迟在微秒级, 不影响控制循环主路径延迟 p99 < 100µs 的性能目标.
