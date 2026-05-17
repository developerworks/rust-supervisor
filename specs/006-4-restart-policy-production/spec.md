# Feature Specification (功能规格): 生产级重启策略与分组隔离观测

**Feature Branch (功能分支)**: `[006-4-restart-policy-production]`
**Created (创建日期)**: 2026-05-17
**Status (状态)**: Draft (草稿)
**Input (输入)**: 本规格对应第二序列里程碑: 接入 restart budget(重启预算), meltdown fuse(熔断器), group strategy(分组策略), escalation policy(升级策略), backoff jitter(退避抖动). 快速失败不会造成无限重启风暴, group(分组) 故障不会误伤无关任务, critical child(关键子任务) 和 optional child(可选子任务) 有不同处理, 所有策略决策都有事件和指标.

## Dependency Note (依赖说明)

本切片强依赖 specs/005-1-failure-policy-reliability/spec.md 以及 specs/005-2-work-role-defaults/spec.md 已经给出的失败流水线入口. 若条文字面重复, 本条以度量字段完备性与分叉观测补齐为主.

## User Scenarios & Testing (用户场景和测试) _(mandatory (必填))_

### User Story 1 (用户故事一) - 快速失败不致风暴 (Priority (优先级): P1)

SRE(站点可靠性工程师) 需要子任务在同一窗口里连着崩溃 10_000 次时, 仍能看出再起节拍被 restart budget(重启预算) 与 backoff jitter(退避抖动) 压住. 监督器控制线程不能因为等某个故障子任务, 就把别的就绪任务长期晾在一边拿不到调度机会.

**Why this priority (为什么是这个优先级)**: 再起风暴是线上第一类点火源.

**Independent Test (独立测试)**: 输入固定失败波形脚本. 统计每分钟 effective restart attempts per minute(每分钟有效再起尝试), 与 YAML 中的预算曲线 CSV 对照.

**Acceptance Scenarios (验收场景)**:

1. **Given (假设)** 单次失败回路触发间隔低于配置文件里的抖动下限, **When (当)** 失败波形重复滚动 60 秒, **Then (则)** 实测再起间隔不得低于文档给出的下限曲线, 且 typed event(类型化事件) 载荷里附带本轮预算计数快照.

### User Story 2 (用户故事二) - 分组故障止步于组边界 (Priority (优先级): P1)

拓扑设计师需要 group(分组) A 触发熔断后, group(分组) B 内的 optional child(可选子任务) 在线时长不因 A 的风暴掉到对照实验基线以下, 除非配置图里写明跨组 dependency edge(依赖边).

**Why this priority (为什么是这个优先级)**: 单监督器实例常被多租户拼装. 边界不清楚会直接造成无辜租户停机.

**Independent Test (独立测试)**: 双分组对照实验室环境里统计 B 侧 uptime(在线时间) 比例, 并与隔离对照组比对 24 小时滑动窗口.

**Acceptance Scenarios (验收场景)**:

1. **Given (假设)** 仅在 group(分组) A 内注入熔断触发条件, **When (当)** meltdown fuse(熔断器) 生效, **Then (则)** group(分组) B 的 HealthyBaseline(健康基线) 计数不降, 除非拓扑配置文件列出跨组依赖并被加载器校验通过.

### User Story 3 (用户故事三) - critical 与 optional 分叉可观测 (Priority (优先级): P2)

产品负责人需要在事后复盘导出的事件 CSV 与 metrics(指标) 抓取结果里一眼区分关键子任务的升级路径与可选子任务的降噪路径.

**Why this priority (为什么是这个优先级)**: 分叉不可观测就无法写值守脚本触发条件.

**Independent Test (独立测试)**: 对两条路径分别抓取最新 100 条事件记录与同一时间窗 metrics(指标) 标签集合. 核对字段基数差异.

**Acceptance Scenarios (验收场景)**:

1. **Given (假设)** 同一类底层故障注入脚本触发失败, **When (当)** 目标 child(子任务) 一行标记 critical(关键) 另一行标记 optional(可选), **Then (则)** escalation policy(升级策略) 分叉必须在 typed event(类型化事件) 与 metrics(指标) 两条通道各自至少多出 3 个互不混淆的诊断键.

### Edge Cases (边界情况)

- 当一个 critical child(关键子任务) 同时挂在两个 group(分组) 且两边 policy(策略) 冲突时, tie-break(平局裁决) 规则必须写成可读表格而不是仅靠代码默认值.
- 当 meltdown(熔断) 与手动 quarantine(隔离) 并发触发时, 必须写明人工指令是否优先, 并在审计流水记下版本戳.
- 当 optional child(可选子任务) 抖动失败时, backoff jitter(退避抖动) 参数必须打散再起节拍, 避免出现同步 thundering herd(惊群).

## Requirements (需求) _(mandatory (必填))_

### Functional Requirements (功能需求)

- **FR-001**: 系统必须把 restart budget(重启预算), meltdown fuse(熔断器), backoff jitter(退避抖动) 接到 decide action(决定动作) 节拍之前同一评估管线里. 在快速失败波形下实测 effective restart attempts per minute(每分钟有效再起尝试) 不得超过文档给出曲线上界的 105%. fairness(公平性) 探针记录在任意连续 10 秒窗口内, 其它就绪监督动作至少获得过调度机会的计数不低于文档阈值.
- **FR-002**: group strategy(分组策略) 必须保证在未声明跨组 dependency edge(依赖边) 的前提下, 任一 group(分组) 自家熔断或预算耗尽不得把关停的连带后果甩到不相干的 group(分组) 头上. 一旦发生跨组可见影响, 必须产出指向依赖图节点的 structured diagnostics(结构化诊断) 载荷.
- **FR-003**: critical child(关键子任务) 与 optional child(可选子任务) 的失败处置必须有配置文件里的分叉默认值. 每一条分叉路径上的预算耗尽与升级裁决都必须 100% 写入 typed event(类型化事件) 与 metrics(指标) 两组管道, 并能被同一个 correlation id(关联标识) 串联.

### Key Entities (关键实体) _(涉及数据时填写)_

- **RestartBudgetSnapshot(重启预算快照)**: 某个评估窗口内 consumed(已消耗) 与 remaining(剩余) 再起额度字段的结构化视图.
- **GroupFaultBoundary(分组故障边界)**: 描述熔断停在分组叶节点还是沿依赖边上溯的配置切片.
- **SeverityClass(严重程度分类枚举)**: 配置文件里划分 critical(关键) 与 optional(可选) 及其它扩展枚举的标签轴.

## Constitution Alignment (宪章对齐) _(mandatory (必填))_

### Supervision Contract (监督契约)

- **Lifecycle impact (生命周期影响)**: 改动再起节拍与 shutdown(关闭) 耦合节奏, 必须与 006-3 关停切片联合验收.
- **Failure behavior (失败行为)**: 必须写明预算耗尽引起的 escalate(升级) 与普通失败再起之间的分界枚举值.

### Rust Boundary and Observability Requirements (Rust 边界和可观察性需求)

- **Module ownership (模块所有权)**: 策略裁决代码只能落在 policy(策略目录) 与 observe(观测目录) 之间的契约边界内.
- **Compatibility exports (兼容导出)**: None (无).
- **Diagnostics (诊断)**: typed event(类型化事件) 先于自由文本 message(消息) 字段对外承诺稳定性.

### Chinese Writing (中文写作)

- **Writing language (写作语言)**: 本文档必须使用中文.
- **Term format (术语格式)**: 英文术语必须写成 English(中文说明).
- **Forbidden style (禁止风格)**: 禁止全角标点, 禁止用形容词堆叠替换可对账阈值百分号写法.

## Success Criteria (成功标准) _(mandatory (必填))_

### Measurable Outcomes (可衡量结果)

- **SC-001**: 在 10k(万次) 瞬时失败波形下, effective restart attempts per minute(每分钟有效再起尝试) 实测样本不得超过文档曲线包络上界的 105%.
- **SC-002**: 双分组对照实验中在未声明跨组依赖的前提下, B 侧额外非计划停机时间相对 24h (二十四小时) 对照窗不得超过 5%.
- **SC-003**: typed event(类型化事件) 与 metrics(指标) 针对同一 SupervisorDecision(监督器裁决) 键的一致率抽检样本不低于 98%.

## Assumptions (假设)

- 默认 metrics(指标) 后端由集成方注入适配层. 监督器只暴露稳定的打点字段契约.
- 分组故障隔离依赖运行时拓扑中的 dependency edge(依赖边) 声明, 该声明由 006-6 切片中的配置模型加载.
