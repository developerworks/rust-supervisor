# Feature Specification(功能规格): 失败策略流水线与生产级退避

**Feature Branch(功能分支)**: `[005-1-failure-policy-reliability]`
**Created(创建日期)**: 2026-05-16
**Updated(更新日期)**: 2026-05-19
**Status(状态)**: Accepted(已接受)
**Input(输入)**: 用户描述: "项目已经有 PolicyEngine(策略引擎), BackoffPolicy(退避策略), MeltdownTracker(熔断跟踪器) 等模型. PolicyEngine(策略引擎) 会根据成功, 失败, 取消, panic(崩溃), timeout(超时) 返回 restart decision(重启决策). BackoffPolicy(退避策略) 也有指数退避和确定性 jitter(抖动) 计算. 但是 runtime control loop(运行时控制循环) 里没有看到 MeltdownTracker(熔断跟踪器) 和 restart limit(重启次数限制) 被真正使用. restart_execution_plan(重启执行计划) 虽然会把 restart limit(重启次数限制) 和 escalation policy(升级策略) 放进计划, 但是控制循环并没有用上这些字段.

这里要做三件事. 第一, 所有失败必须进入同一个 policy pipeline(策略流水线): classify exit(分类退出), record failure window(记录失败窗口), evaluate budget(评估预算), decide action(决定动作), emit typed event(发出类型化事件), execute action(执行动作). 第二, MeltdownTracker(熔断跟踪器) 必须按 child(子任务), group(分组), supervisor(监督器) 三个维度保存, 防止局部雪崩变成全局重启风暴. 第三, BackoffPolicy(退避策略) 要从确定性测试工具升级为生产退避策略, 支持 full jitter(全抖动), decorrelated jitter(去相关抖动), 最大并发重启限制, cold start budget(冷启动预算), hot loop detection(热循环检测).

尤其要注意, 当前 Permanent(永久重启) 策略会在任务成功退出后也重启. 这对 daemon(常驻任务) 是合理的, 但是对 job(一次性作业) 很危险. 工业产品应该明确区分 service(常驻服务), worker(工作任务), job(一次性作业), sidecar(辅助任务), supervisor(嵌套监督器), 并为每类任务设置不同默认策略. "

## Plain-language Summary(白话摘要)

下列三条只帮助读者读懂 **Input**, 不构成额外功能需求; 以正文用户故事, 功能需求与成功标准为准.

1. **`policy pipeline`(策略流水线)**: 凡是失败都必须走同一条流水线, 不得跳过或另开旁路; 六个阶段顺序固定; **`restart_execution_plan`(重启执行计划)** 所含的 **`restart limit`(重启次数限制)** 与 **`escalation policy`(升级策略)** 须在 **`evaluate budget`(评估预算)** 阶段参与限额核算并进入是否重启等判定结论, 且在 **`decide action`(决定动作)** 阶段形成可对账的记录.
2. **`MeltdownTracker`(熔断跟踪器) 三层分开记**: 按 `child`(子任务), `group`(分组), `supervisor`(监督器) 三层分别计数并各自判定熔断, 避免局部故障在短期内耗尽整机可用的重启配额.
3. **`BackoffPolicy`(退避策略) 用于线上**: 测试时可以注入时间与随机种子让结果可重复; 线上启用 **`full jitter`(全抖动)**, **`decorrelated jitter`(去相关抖动)**, 最大并发重启, `cold start budget`(冷启动预算), **`hot loop detection`(热循环检测)**, 减轻同一时间大批重启造成的尖峰, 并减少对 **`downstream`(下游)** 依赖与 **`control plane`(控制面)** 的冲击.

## Dependency Note(依赖说明)

本切片写清失败时要走的统一流水线, 多层熔断计数如何累计, 以及线上退避如何生效. **`job`(一次性作业) 若套用 `Permanent`(永久重启), 将与 "契约定义的成功退出出现后监督侧不应再自动拉起新一轮" 这一预期相冲突**, 这一点在 `specs/005-2-work-role-defaults/spec.md` 里用角色默认策略和验收场景单独说明; 本切片只要求流水线在执行过程中**读取并真正使用** **`restart_execution_plan`(重启执行计划)** 里已经有的 **`restart limit`(重启次数限制)** 与 **`escalation policy`(升级策略)** 字段, 并把熔断判定结果以及与限额和是否重启有关的结论写入可订阅或可导出的 **`TypedSupervisionEvent`(类型化监督事件)**.

## Clarifications

### Session 2026-05-16

- Q: 是否在规格末尾补充 `full jitter`(全抖动) 与 `decorrelated jitter`(去相关抖动) 的概念描述 → A: 是, 见"概念描述"编号列表并与 **`BackoffPolicy`(退避策略)** 用词一致.
- Q: 是否补充 `classify exit`(分类退出) 的概念描述 → A: 是, 见"概念描述"第 3 条并与 **`policy pipeline`(策略流水线)** 第一阶段用词一致.
- Q: 是否补充 `tie-break`(平局判定) 的概念描述 → A: 是, 见"概念描述"第 4 条并与 **`classify exit`(分类退出)** 里多规则同样适用时的说法一致.
- Q: **`child`(子任务)**, **`group`(分组)**, **`supervisor`(监督器)** 三层 **`MeltdownTracker`(熔断跟踪器)** 阈值同在时怎样合并多层判定并在事件里写明主导归因落在哪一层 **`scope`(作用域)** → A: 见 **`FR-002`**, **Normative Ordering** 与 **Edge Cases** 多层阈值条目; **`effective meltdown verdict`(有效熔断判定)** 取 **`protection restrictiveness ladder`(保护从严档位序)** 上各层 **`local verdict`(局部判定)** 里最严那一档作为对外公布的统一处置结论用语; **`TypedSupervisionEvent`(类型化监督事件)** 必须带 **`scopes_triggered`(已触发作用域列表)** 与 **`lead_scope`(主导归因作用域)**, 当多层 **`local verdict`(局部判定)** 一样严时 **`lead_scope`** 按固定次序取 **`child`(子任务)** → **`group`(分组)** → **`supervisor`(监督器)**.
- Q: **`FR-001` 流水线入口是否循环定义 **`exit kind`** 最小集合 **`restrictiveness ladder`** 并发闸门 **`cold start`** 与 **`hot loop`** 整条含义 → A: 见 **`FR-001`**, **`FR-002`**, **Normative Ordering**, **User Story 3**, **Edge Cases**, **Assumptions** 上文各节; 流水线入口改为"每一次运行结束情形"先进 **`classify exit`(分类退出)**; **`protection restrictiveness ladder`(保护从严档位序)** 在本 **`spec.md`** 写明固定的从严顺序; **`supervisor`(监督器)** 内 **`global`(全局)\*\* 闸门不与进程内其它监督器实例共享计数桶.

## User Scenarios & Testing(用户场景和测试) _(mandatory(必填))_

### User Story 1(用户故事一) - 失败路径进入单一可查流水线 (Priority(优先级): P1)

作为管一棵监督树的运维, 我希望任意受监督单元在系统就本轮失败作出是否自动重启, 是否受熔断限速, 以及是否停机的最终决定之前, 都按同一套先后顺序汇总本次失败在各阶段留下的可对账诊断输出, 这样各方复盘失败原因时用词一致, 与限额字段, 闸门档位以及是否允许自动重启有关的结论也能依据 **`TypedSupervisionEvent`(类型化监督事件)** 或导出记录逐项核对.

**Why this priority(为什么是这个优先级)**: 缺少单一流水线会使 **`restart limit`(重启次数限制)** 与 **`escalation policy`(升级策略)** 无法在 **`evaluate budget`(评估预算)** 起的后续阶段改变对外可见的处置结果, 也无法证明每条失败路径都按同一套规则处理.

**Independent Test(独立测试)**: 用一组固定的失败样本从外部检查, 每条样本触发的监督结论均能与同一套按顺序排列的阶段一致, 且每个阶段至少有一种可以被订阅或导出的诊断信息与之对应.

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** 某个 `child`(子任务) 因非零退出被视为失败, **When(当)** 监督控制面走完六阶段流水线, **Then(则)** 外部观察者按顺序能看到与 **`classify exit`(分类退出)** → **`record failure window`(记录失败窗口)** → **`evaluate budget`(评估预算)** → **`decide action`(决定动作)** → **`emit typed event`(发出类型化事件)** → **`execute action`(执行动作)** 顺序一致的诊断记录链条, 且若中间某一阶段已经写明禁止重启或写明固定处置, **`execute action`(执行动作)** 不得给出与之正面冲突的结论.
2. **Given(假设)** `restart_execution_plan`(重启执行计划) 携带非空的 **`restart limit`(重启次数限制)** 或 **`escalation policy`(升级策略)** 字段, **When(当)** 失败流水线运行, **Then(则)** **`evaluate budget`(评估预算)** 阶段必须读到这些字段并体现在 **`decide action`(决定动作)** 对外可见的结果里, 不得静默丢弃.

---

### User Story 2(用户故事二) - 熔断压力按作用域隔离 (Priority(优先级): P2)

作为平台责任人, 我希望同一棵监督树里某一 `group`(分组) 或单个 `child`(子任务) 连续失败时, 勿在短期内耗尽整棵树的重启配额, 以便把故障限制在该分组或该子任务范围内, 并避免扰乱全局重启节奏.

**Why this priority(为什么是这个优先级)**: 如果只用一个计数桶做熔断, 关联度不高的故障也会被合并计数, 最终导致整棵树一并停摆, 不符合线上常见的按故障范围分区处置的做法.

**Independent Test(独立测试)**: 仅向某一 `group`(分组) 或单个 `child`(子任务) 注入密集失败, 检查其它分组和其它子任务是否仍能在各自熔断配额内单独走完熔断判断.

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** `MeltdownTracker`(熔断跟踪器) 已为同一监督实例记录多层计数, **When(当)** 仅分组 `G` 内多个 `child`(子任务) 在短期内连续失败, **Then(则)** 分组 `G` 进入熔断或限速状态时, 其它分组默认不受影响, 除非共享的监督器级阈值也被单独耗尽.
2. **Given(假设)** 单个 `child`(子任务) 触发高频失败, **When(当)** 达到该 `child`(子任务) 维度阈值, **Then(则)** 系统对该 `child`(子任务) 先停下来或转入升级路径, 同时不能把这次计数错误归属到无关的 `child`(子任务).
3. **Given(假设)** 在同一轮预算评估里, 同一次进程结束让某个 `child`(子任务), 其 **`group`(分组)** 与 **`supervisor`(监督器)** 三层 **`MeltdownTracker`(熔断跟踪器)** 计数一起越过阈值, 且三层 **`local verdict`(局部判定)** 在 **`protection restrictiveness ladder`(保护从严档位序)** 上并列同为 **`effective meltdown verdict`(有效熔断判定)**, **When(当)** 流水线合并熔断结论后发出与这一轮预算评估相关的 **`TypedSupervisionEvent`(类型化监督事件)**, **Then(则)** 事件正文包含 **`scopes_triggered`** 且 **`lead_scope`** 为 **`child`(子任务)**.

---

### User Story 3(用户故事三) - 生产级退避与并发重启闸门 (Priority(优先级): P3)

可靠性工程师需要延长相邻两次重启的等待间隔, 限制同时进行的自动重启数量, 并在冷启动和热循环场景下收紧预算. 这样可以减轻 **`thundering herd`(雷群效应)** 导致的资源竞争, 避免业务侧因瞬时负载过高而服务质量下降或积压失控.

**Why this priority(为什么是这个优先级)**: 确定性 **`jitter`(抖动)** 虽便于回归测试, 但无法有效分散真实环境中的重启高峰. 若无并发闸门, 监督状态维护方与指令下发方可能在短时间内同时达到容量上限.

**Independent Test(独立测试)**: 在可控时钟和随机种子下, 对比 **`full jitter`(全抖动)** 与 **`decorrelated jitter`(去相关抖动)** 相对于固定 **`jitter`(抖动)** 的等待时长分散程度. 使用并发失败样本验证 **`hot loop detection`(热循环检测)** 与 **`cold start budget`(冷启动预算)** 的限速行为是否符合预期.

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** 策略声明使用 **`full jitter`(全抖动)** 或 **`decorrelated jitter`(去相关抖动)**, **When(当)** 为同类失败批量安排重启前等待, **Then(则)** 相邻两次重启间隔比固定 **`jitter`(抖动)** 更分散, 且在回归模式下可用固定种子复现相同等待序列.
2. **Given(假设)** 在同一 **`supervisor`(监督器)** 实例内启用 **实例全局闸门计数** (对该实例托管的全部 **`child`(子任务)** 生效, **计数不与进程内其他 **`supervisor`(监督器)** 实例合并**), **或为某 **`group`(分组)** 启用分组闸门计数**, **When(当)** 同一时段触发失败数超出闸门上限, **Then(则)** 超出部分必须进入 **`restart_queued`(排队重启)** 或 **`restart_denied`(拒绝重启)** 等符合 **`protection restrictiveness ladder`(保护从严档位序)** 的保护档位, 且该限速决策写入类型化事件流, **事件中须注明闸门作用于 **`supervisor` 实例全局** 还是某 **`group`(分组)\*\*.
3. **Given(假设)** 监督树处于 **`cold start`(冷启动)** 定义的初始窗口且失败密集到来, **When(当)** **`cold start budget`(冷启动预算)** 耗尽或硬性触发条件满足, **Then(则)** **`effective_protective_action`(生效的保护处置)** 至少收紧到 **`restart_denied`(拒绝重启)**, **除非 **`restart_execution_plan`(重启执行计划)** 明确声明耗尽档位仅为 **`restart_queued`(排队重启)**; **`TypedSupervisionEvent`(类型化监督事件)\*\* 必须写明所选档位与耗尽依据.
4. **Given(假设)** 滑动时间窗内可观察到崩溃后短时间再次被拉起的记录序列, **When(当)** **`hot loop detection`(热循环检测)** 触发, **Then(则)** **`effective_protective_action`(生效的保护处置)** 必须与仅凭 **`restart limit`(重启次数限制)** 超限给出的处置在事件字段上可区分, **且取值必须是 **`restart_denied`(拒绝重启)**、**`supervision_paused`(暂停监督)**、**`escalated`(升级)** 或 **`supervised_stop`(监督停止)** 之一**, **`TypedSupervisionEvent`(类型化监督事件)** 必须写明检测窗口阈值与档位.

---

### Edge Cases(边界情况)

- **多层阈值同在**: **计数**仍在 **`child`(子任务)**, 所属 **`group`(分组)**, **`supervisor`(监督器)** 三层各自单独累计; **写进对外结论里的补救动作**在同一轮 **`evaluate budget`(评估预算)** 内必须符合 **`FR-002`** 规定的取最严档位的合并规则以及事件里 **`scopes_triggered`(已触发作用域列表)** 与 **`lead_scope`(主导归因作用域)** 的约定; **`local verdict`(局部判定)** 只能是 **`protection restrictiveness ladder`(保护从严档位序)** 中的某一档, **或是写在契约里的别名与本档位序中的唯一一档建立一一对应且从严顺序不乱**, **契约不得把本规格写明的严松关系改窄或颠倒**.
- 当 **`restart limit`(重启次数限制)** 与 **`MeltdownTracker`(熔断跟踪器)** 计数算法不一致时, 必须以契约文档写明的计数规则为准, 并在 **`evaluate budget`(评估预算)** 阶段写明采纳的规则来源之后再作出判定.
- 当外部取消或人为停止与自动重启竞争执行权时, **`execute action`(执行动作)** 不得将已经标明必须结束的任务再次自动拉起.
- 当 **`cold start budget`(冷启动预算)** 与 **`hot loop detection`(热循环检测)** 在同一轮预算评估里同时触发时, **`effective_protective_action`(生效的保护处置)** 必须按 **`protection restrictiveness ladder`(保护从严档位序)** 从这两类触发各自给出的候选档位里挑出 **更严** 那一档作为结果; **`TypedSupervisionEvent`(类型化监督事件)** 必须把两条触发原因和最终选用的档位一并写出, **不得在未写明取舍理由的情况下合并或省略其中任一触发原因**.

## Requirements(需求) _(mandatory(必填))_

### Functional Requirements(功能需求)

- **FR-001**: 系统必须把每一条受监督单元的运行结束情形送入单一 **`policy pipeline`(策略流水线)**. **不得以"预判是否会补救"作为放进流水线的门槛**; **是否采取补救措施以及补救措施的具体内容** 只在 **`decide action`(决定动作)** 与 **`execute action`(执行动作)** 才会实际落地. 流水线入口固定为 **`classify exit`(分类退出)**, **每一条运行结束情形必须先归入 **`exit kind`(退出类别)**. **最小必选集合** 至少包含 **`success`(成功)**, **`nonzero_exit`(非零退出)**, **`panic`(崩溃)**, **`timeout`(超时)**, **`external_cancel`(外部取消)**, **`manual_stop`(人工停止)**; 六种都必须进入 **`classify exit`(分类退出)** 以满足测试夹具覆盖全集. 契约只能增添细分标签并声明其与最小集合的 **`tie-break`(平局判定)**, **不得删除任一最小标签**. **六个阶段顺序固定**: **`classify exit`(分类退出)** → **`record failure window`(记录失败窗口)** → **`evaluate budget`(评估预算)** → **`decide action`(决定动作)** → **`emit typed event`(发出类型化事件)** → **`execute action`(执行动作)**; **`success`(成功)** 在后继阶段可为 **`no-op`(空操作)**, **但每一阶段至少在事件流里留下可对账的记录点**, **禁止跳过流水线直接去自动重启\*\*.
- **FR-002**: 系统必须把 **`MeltdownTracker`(熔断跟踪器)** 的状态按三个互不混算的 **`scope`(作用域)** 分开保存: 单个 `child`(子任务), 配置里绑定的 `group`(分组), 以及托管这棵监督树的 `supervisor`(监督器) 实例; 每一层都要有清楚的阈值扣减与越线判定路径, 在未约定计数迁移规则的前提下默认不得使单层计数占用其它层的配额. 在同一轮 **`evaluate budget`(评估预算)** 需要同时看多 **`MeltdownTracker`(熔断跟踪器)** 的结论时, 必须先得出每一层的 **`local verdict`(局部判定)**, **`local verdict`(局部判定)** 只能落在 **`protection restrictiveness ladder`(保护从严档位序)** 的某一档上, **或是写在契约里的别名与本档位序中的唯一一档建立一一对应且从严顺序不乱**; **`effective meltdown verdict`(有效熔断判定)** 等于各层 **`local verdict`(局部判定)** 在该档位序上取其中最严一档的总结果. **`emit typed event`(发出类型化事件)** 里与熔断合并结论有关的字段必须带 **`scopes_triggered`(已触发作用域列表)**, 列出这一轮达到或越过阈值的 **`scope`(作用域)**; 还必须带 **`lead_scope`(主导归因作用域)**, 当多层 **`local verdict`(局部判定)** 都与 **`effective meltdown verdict`(有效熔断判定)** 一样严时, 按 **`tie-break`(平局判定)** 固定次序取 **`child`(子任务)** 先于 **`group`(分组)** 先于 **`supervisor`(监督器)**, 便于驱动状态机并支撑对外验收查阅, 亦无须依赖源码中仅为边界情形预备的分支方可获知判定依据.
- **FR-003**: 系统必须把 **`BackoffPolicy`(退避策略)** 扩展成在线上负载下约束相邻两次自动重启之间等待时长时必须遵守的规则, 至少支持 **`full jitter`(全抖动)**, **`decorrelated jitter`(去相关抖动)**, **最大并发重启限制**, **`cold start budget`(冷启动预算)**, **`hot loop detection`(热循环检测)**; 并允许在固定种子或注入时钟的测试模式下保持测试结果可重复, **以免同一验收夹具因随机种子漂移而得不到稳定结论**.

### Normative Ordering(规范性排序)

#### `protection restrictiveness ladder`(保护从严档位序)

文中亦称 **从严保护档位序**, 与英文名 **`protection restrictiveness ladder`** 指同一套说法.

下列档位是本规格的 **`canonical`(下文统一使用的标准名称)** ; **契约**可以使用别名, 但每一种别名都必须能对应到本档位序里的某一档, **且不得把更严的档位说成更松或反过来**. **从左到右逐级从严**; **凡是把多层结论收成一条对外公布的处置结论时**, 都在档位序上取 **最靠右那一档** 即最严一档.

1. **`restart_allowed`(允许按计划重启)**: 基准档, 这一轮里不因 **`MeltdownTracker`(熔断跟踪器)** 或 **`BackoffPolicy`(退避策略)** 附加闸门而改变原先打算做的重启决定.

2. **`restart_queued`(排队重启)**: **推迟或排队重启**, **但仍须标明本轮意图仍为重启, 只是要先排队或延后**.
3. **`restart_denied`(拒绝重启)**: **在策略写明的一段时间内不得发起新的自动重启**.
4. **`supervision_paused`(暂停监督)**: **暂停针对该受监督单元的自动化监督动作, 直到约定的解除条件达成**.
5. **`escalated`(升级)**: 转入 **`escalation policy`(升级策略)** 写明的一套外层处置步骤.
6. **`supervised_stop`(监督停止)**: **停止自动拉起并保持停住直到有人明确说要再运行**.

### Key Entities(关键实体) _(include if feature involves data(涉及数据时填写))_

- **`PolicyPipelineStage`(策略流水线阶段)**: 六阶段流水线中的固定一节; 承接上一阶段落在订阅端或导出文件里的摘要字段; 为本阶段 **`TypedSupervisionEvent`(类型化监督事件)** 载荷应出现的字段名给出核对清单, **不写具体 Rust trait 或函数签名**.
- **`FailureWindow`(失败窗口)**: 按时间滑动或按次数滑动, 把相近的失败归到一起的一段区间或一格计数, 用来滚动累计失败并与阈值比较.
- **`MeltdownScopeState`(熔断作用域状态)**: 绑定在某个 `child`(子任务), `group`(分组) 或 `supervisor`(监督器) 上的额度, 计数, 以及本轮 **`evaluate budget`(评估预算)** 得出的 **`local verdict`(局部判定)** 档位的一份汇总说明.
- **`RestartThrottlePlan`(重启节流计划)**: 说明并发上限, 冷启动收紧与热循环降级如何根据策略参数算出最终等待时长以及是否允许重启, 写成验收时能逐项核对的一份说明.
- **`TypedSupervisionEvent`(类型化监督事件)**: 面向诊断订阅的统一事件类型, 须能判定事件对应的流水线阶段, 并须能从字段读出预算驳回, 熔断触发, 退避选择以及最终是否重启等结论; 涉及多层 **`MeltdownTracker`(熔断跟踪器)** 合并结论时, 事件中的 **`scopes_triggered`(已触发作用域列表)** 与 **`lead_scope`(主导归因作用域)** 须符合 **`FR-002`** 的字段约定.

## Constitution Alignment(宪章一致) _(mandatory(必填))_

### Supervision Contract(监督契约)

- **Lifecycle impact(生命周期影响)**: 本变更加强进程结束之后的自动补救动作以及闸门给出的限制, **并要求每一次运行结束情形都必须依次走完六阶段流水线并能核对**; 契约已写的 **`success`(成功)** 与 **`manual_stop`(人工停止)** 等业务语义边界不改, **但成功路径也必须在每一阶段至少在事件流里留下可对账的记录点**.
- **Failure behavior(失败行为)**: 失败必须产出可追溯的诊断内容, 写明在哪一层 **`scope`(作用域)** 触发熔断判定, **`evaluate budget`(评估预算)** 阶段的限额与闸门相关结论是什么以及采用了何种退避; **`restart limit`(重启次数限制)** 与 **`escalation policy`(升级策略)** 不得仅写在计划里却不改变最终结果.
- **Shutdown behavior(关闭行为)**: 停止, 取消与 join(等待任务结束) 的契约保持有效; 流水线在收到明确的收尾或停机指令后必须走收尾完成或中断分支, 不得再次自动拉起进程.

### Rust Boundary and Observability Requirements(Rust 边界和可观察性需求)

- **Module ownership(模块所有权)**: 运行循环, 策略评估, 熔断状态与退避调度各自归属哪个模块必须写清楚, 不得将新增阶段逻辑散落在无名辅助函数里.
- **Compatibility exports(兼容导出)**: None(无)
- **Diagnostics(诊断)**: 每个流水线阶段至少一种可测试的 **`TypedSupervisionEvent`(类型化监督事件)** 或另一种能用固定格式读出来的诊断输出; tracing(结构化追踪) 级别信息必须能还原整条阶段顺序.
- **Dependency impact(依赖影响)**: 若引入受信的随机数库或时钟封装库, 必须在计划中说明可选性与审计理由; 默认优先可测注入.

### Chinese Writing(中文写作)

- **Writing language(写作语言)**: 本文档必须使用中文.
- **Term format(术语格式)**: 英文术语必须写成 `English(中文说明)`.
- **Forbidden style(禁止风格)**: 禁止非中文写作, 片段式语言, 生僻词和方言.

## Success Criteria(成功标准) _(mandatory(必填))_

### Measurable Outcomes(可衡量结果)

- **SC-001**: 在固定验收场景集上, 至少 100% 的模拟失败样本触发的监督结果均能与六阶段流水线的先后顺序一致, 审查者不看源码也能从事件或诊断导出里核对顺序是否正确.
- **SC-002**: 在分组隔离测试中, 局部密集失败触发保护后, 其它分组在同等时间窗内至少保持 90% 以上用例仍可独立完成一次受控重启尝试, 除非监督器级阈值被独立耗尽.
- **SC-003**: 在并发失败压力样本中, 观察到的瞬时并行自动重启峰值不超过声明上限的 100%, 超出部分必须体现在显式推迟或队列诊断里.
- **SC-004**: 启用 **`full jitter`(全抖动)** 或 **`decorrelated jitter`(去相关抖动)** 时, 同一批因相近原因触发的重启在时间轴上须较固定 **`jitter`(抖动)** 更为分散; 衡量分散程度的算法写在验收夹具里且不绑死某一家云服务商; 分散程度的数值要比固定 **`jitter`(抖动)** 基准高出至少三成.

## Assumptions(假设)

- `group`(分组) 已在配置或拓扑描述里具有稳定标识; 若某 `child`(子任务) 未绑定分组, 则分组作用域对这一 `child`(子任务) 不参与计数也不参与熔断判定, 除非计划在契约里写明另有中立处理方式且仍不得绕过 **`supervisor`(监督器)** 级阈值.
- **`cold start budget`(冷启动预算)** 默认绑到这个监督实例启动后的有限时间窗或有限重启次数配额, 可由上层配置覆盖.
- **`hot loop detection`(热循环检测)** 依赖可配置的短时间窗内最小重启次数或最小间隔阈值组合; 默认阈值以保证测试可在秒级时钟下稳定触发为前提.
- 现有 **`restart_execution_plan`(重启执行计划)** 字段名称含义照旧; 若字段缺失则流水线必须在 **`evaluate budget`(评估预算)** 阶段采用契约文档声明的安全默认而非静默忽略.
- **最大并发重启限制**: 单个 **`supervisor`(监督器)** 实例内的 **全局闸门计数** 表示该实例所托管的全部 **`child`(子任务)** 共用一套闸门计数, 计数不与进程内其它 **`supervisor`(监督器)** 实例合并, 也不隐含跨主机集群层面的全局计数; **`group`(分组)** 级闸门只在同一 **`supervisor`(监督器)** 实例内对该分组计数; 默认至少启用实例全局闸门, 分组闸门为可选, 未启用分组闸门时回落到实例全局.

## Concept Descriptions(概念描述)

以下为文中术语的简要概念说明, 不涉及具体参数默认值或实现公式.

1. **`full jitter`(全抖动)**: 在为某次重试或重启决定等待时长时, 先在当前策略给出的上限之内确定本次可用的最大等待值, 再在从零到该上限的区间内做一次均匀随机抽样, 把本次实际等待设为抽样结果.

2. **`decorrelated jitter`(去相关抖动)**: 决定下一次等待时长时, 在一个同时依赖初始基数与上一轮实际等待长度且上限下限都写清楚的区间内随机取值, 使相邻两轮等待不必捆在同一倍数关系里, 从而减少两轮等待总是一起变长或一起变短的现象.

3. **`classify exit`(分类退出)**: 依据退出码, 信号, 取消, 崩溃或超时等直接从进程拿到的结束事实, 把一次运行结束归到一个有限的 **`exit kind`(退出类别)** 标签上; 同类结束事实始终得到同一标签.

4. **`tie-break`(平局判定)**: 当多条归类规则对同一批结束事实同样适用时, 按事先约定且不带随机性的优先级次序选出唯一的 **`exit kind`(退出类别)**, 在同分时排出唯一先后.

5. **`thundering herd`(雷群效应)**: 大量实例在同一时段被同类事件唤醒或重试, 短时内竞相占用锁, 连接或 **`control plane`(控制面)** 等资源, 从而在观测指标上形成急剧抬升的负载尖峰.
