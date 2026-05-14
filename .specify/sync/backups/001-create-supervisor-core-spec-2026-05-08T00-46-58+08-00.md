# Feature Specification(功能规格): 创建监督器核心

**Feature Branch(功能分支)**: `001-create-supervisor-core`
**Created(创建日期)**: 2026-05-04
**Status(状态)**: Draft(草稿)
**Input(输入)**: 用户描述:"吸收 `task-supervisor`,`taskvisor`,`tokio-graceful-shutdown`,`ractor-supervisor`,`task_scope`,Tokio(异步运行时) `JoinSet`,`supertrees`,Tokio(异步运行时) `watch`,`tokio-util` `CancellationToken` 和 `tracing`(结构化追踪) 的成熟概念,创建一个基于 Tokio(异步运行时) 的轻量 supervisor(监督器) 运行时治理层.它负责启动,停止,重启,隔离,降级,熔断,状态查询,事件记录,健康检查和关闭顺序;不引入 actor(参与者) 框架,不照搬第三方 crate(库) API(接口)."

## Clarifications(澄清)

### Session(会话) 2026-05-04

- Q: 这个 supervisor(监督器) 除了自动重启,还必须满足哪些要求? → A: 它必须提供可解释的生命周期治理,并且必须包含声明式子任务,监督树,策略引擎,控制平面,状态平面,事件模型,指标,审计,关闭协议,结构化并发和可复现测试.
- Q: 是否立即采纳 `research-adoption-notes.md` 中"应直接采纳"的 8 条? → A: 立即采纳 control plane(控制面) 和 data plane(数据面) 分离,逆序关闭,readiness(就绪),`spawn_blocking`(阻塞任务启动) 隔离,reconcile(状态对账),event journal(事件日志缓冲区),`RunSummary`(运行摘要),指标标签低基数和 `Service trait`(服务特征) 适配层.
- Q: 任务上下文类型是否使用缩写形式? → A: 不使用缩写,统一使用全称 `TaskContext`(任务上下文).

### Session(会话) 2026-05-05

- Q: analyze(分析) 后需要怎样收敛 C1,G1,G2,G3 和 G4? → A: 规格继续保留 readiness(就绪),blocking task(阻塞任务),event journal(事件日志缓冲区),`RunSummary`(运行摘要) 和 four-stage shutdown(四阶段关闭) 为必须能力,计划和任务必须为它们提供独立测试和实现任务.任务拆分必须提高并行开发程度,并且不得制造同文件并行冲突.
- Q: 测试目录怎样划分? → A: `src/tests/` 是 integration test(集成测试) 的位置,unit test(单元测试) 放在对应模块自己的 `tests/` 目录,实现文件中不得写 inline unit test(内联单元测试) 代码.
- Q: observability(可观测性) 是否是一等能力? → A: observability(可观测性) 是一等能力,必须包含 structured log(结构化日志),tracing span/event(追踪范围和事件),metrics(指标),audit event(审计事件),event journal(事件日志缓冲区) 和 `RunSummary`(运行摘要),并且必须支持 test recorder(测试记录器) 验证.
- Q: 配置,示例和双语文档怎样治理? → A: 项目必须使用 rust-config-tree(集中配置树) 做 centralized configuration(集中化配置),配置禁止分散在各处,并且必须提供 examples(示例程序),complete manual(完整手册) 和 Chinese/English bilingual docs(中英双语文档).
- Q: 代码和文档怎样保持同步? → A: 每次 public API(公开接口),configuration schema(配置模式),example behavior(示例行为) 或 observability signal(可观测性信号) 变化时,代码,手册,文档,quickstart(快速开始),契约和示例必须在同一变更中同步.
- Q: 测试文件怎样命名? → A: 所有 integration test(集成测试),unit test(单元测试),契约测试和质量门禁测试文件必须以 `_test.rs` 结尾.
- Q: rust-config-tree(集中配置树) 使用什么版本和配置格式? → A: rust-config-tree(集中配置树) 必须使用 v0.1.9,并且必须使用 YAML(数据序列化格式) 配置文件,示例配置路径使用 `examples/config/supervisor.yaml`.
- Q: 专业词汇怎样治理? → A: 规格文档涉及的专业词汇必须放入独立 `glossary.md`(词汇表) 文件,反引号内的 Rust(编程语言) 类型名,枚举值,方法名,字段名,指标名,路径名和命令名也算词汇,也必须纳入词汇表.
- Q: 常量值怎样治理? → A: 禁止在代码中硬编码 runtime tunable constant(运行时可调常量).重启阈值,窗口,超时,退避,抖动,容量,开关,预算和默认策略值都必须通过 rust-config-tree(集中配置树) v0.1.9 的 YAML(数据序列化格式) 配置进入系统,并且必须可配置.
- Q: 状态相关代码怎样命名? → A: 代码命名不得使用 `*View` 后缀.监督器状态统一命名为 `SupervisorState`(监督器状态),子任务状态统一命名为 `ChildState`(子任务状态),源码模块使用 `state`(状态),不得使用 `state_view`(状态视图) 模块名.
- Q: 模块之间的依赖关系怎样说明? → A: 规格必须要求 module dependency map(模块依赖图),明确每个 module boundary(模块边界),owner module(所有者模块),dependent module(依赖模块),dependency direction(依赖方向),allowed dependency(允许依赖) 和 forbidden dependency(禁止依赖).模块依赖必须单向,不得出现 cycle dependency(循环依赖),跨模块访问只能通过公开契约类型发生.
- Q: 怎样拆分影响开发并行度的任务? → A: 规格必须要求把影响 development parallelism(开发并行度) 的工作拆分为 parallel workstream(并行工作流).每个 workstream(工作流) 必须有独立 owner(负责人),独立 module boundary(模块边界),独立主文件,独立 `_test.rs` 测试文件,清晰前置依赖和可单独验收结果.任何造成同文件并行写入,共享大文件修改或跨职责串行等待的任务都必须继续拆分.
- Q: 实现阶段怎样执行? → A: implementation phase(实现阶段) 必须采用 unattended implementation(无人值守实现) 模式,按 parallel workstream(并行工作流) 并行推进,不得在单个任务完成后等待人工继续.执行必须持续到 task completion ledger(任务完成台账) 证明所有任务完成,所有验收检查通过,并且没有遗留 pending task(待处理任务) 或 in-progress task(进行中任务).
- Q: 怎样消除影响并行执行的卡点? → A: 规格必须要求 parallel execution blocker(并行执行卡点) 在进入实现前完成识别和消除.卡点包括 shared file bottleneck(共享文件瓶颈),unstable contract(不稳定契约),blocking dependency(阻塞依赖),manual gate(人工门禁),long serial validation(长串行验证),unclear owner(负责人不清晰) 和 hidden coupling(隐藏耦合).每个卡点必须有 blocker elimination record(卡点消除记录),说明消除方式,责任边界,验收证据和剩余风险.
- Q: 并行开发中主代理怎样监督子代理? → A: parallel development(并行开发) 必须由 lead agent(主代理) 监督 subagent(子代理) 的开发工作.lead agent(主代理) 必须分派 workstream(工作流),检查 subagent output(子代理输出),识别 development drift(开发偏差),及时启动 correction loop(纠偏循环),并在 workstream(工作流) 标记完成前形成 correction record(纠偏记录) 或 clean review record(清洁审查记录).
- Q: Source Code(源代码) 模块结构怎样安排? → A: 核心源码必须采用 top-level directory module(顶层目录模块) 结构,模块直接位于 `src/<module>/`,不得保留 `src/supervision/` 中间层,不得使用 `src/<module>.rs` 平铺模块文件.`src/lib.rs` 只允许包含 crate doc(包文档) 和顶层 `pub mod <mod_name>;` 声明,每个 `src/<module>/mod.rs` 只允许包含 `pub mod <mod_name>;` 声明.

## User Scenarios & Testing(用户场景和测试) *(mandatory(必填))*

### User Story 1(用户故事一) - 声明并运行子任务 (Priority(优先级): P1)

维护者需要用声明式 `ChildSpec`(子任务规格) 定义每个 child(子任务):`id`,`name`,`kind`,`restart_policy`,`shutdown_policy`,`health_policy`,`readiness_policy`,`backoff_policy`,`dependencies`,`tags` 和 `criticality`.业务代码不应该分散调用 `tokio::spawn`,而应该把任务生命周期交给 supervisor(监督器).

**Why this priority(为什么是这个优先级)**: 声明式子任务是 supervisor(监督器) 的入口.没有稳定规格,系统就无法统一治理启动,关闭,重启,健康检查,状态查询和审计.

**Independent Test(独立测试)**: 测试定义一个 child(子任务) 规格并启动 supervisor(监督器),然后验证 child(子任务) 进入运行状态,按 readiness(就绪) 契约产生 `ChildStarting`,`ChildRunning` 和 `ChildReady` 事件,并且可以通过当前状态查询到稳定路径和当前状态.

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** 一个包含完整 `ChildSpec`(子任务规格) 的 worker(工作任务),**When(当)** supervisor(监督器) 启动它,**Then(则)** child(子任务) 按规格启动,并记录带稳定路径的生命周期事件.
2. **Given(假设)** 业务代码试图绕过 supervisor(监督器) 直接分散启动后台任务,**When(当)** 维护者审查任务定义,**Then(则)** 该行为不满足本功能规格,因为后台任务必须通过 `ChildSpec`(子任务规格) 接入治理.
3. **Given(假设)** 一个 child(子任务) 需要缓存预热,连接建立或订阅恢复,**When(当)** 它还没有显式报告 readiness(就绪),**Then(则)** supervisor(监督器) 不得把它标记为 ready(已就绪).

---

### User Story 2(用户故事二) - 构建监督树 (Priority(优先级): P2)

维护者需要构建 `SupervisorTree`(监督树).root supervisor(根监督器) 可以包含子 supervisor(监督器) 和 worker(工作任务).worker(工作任务) 负责真实业务任务,supervisor(监督器) 负责任务治理.树中每个节点必须有稳定路径,例如 `/root/market/binance_ws`,用于日志,指标,事件和控制命令定位.

**Why this priority(为什么是这个优先级)**: 单层任务列表无法表达隔离,局部关闭,分组重启和向父级升级.`SupervisorTree`(监督树) 让治理边界和故障传播路径可以解释.

**Independent Test(独立测试)**: 测试创建一个 root supervisor(根监督器),一个子 supervisor(监督器) 和两个 worker(工作任务),然后验证 current state(当前状态) 包含树结构,父子关系,稳定路径和定义顺序.

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** 一个包含子 supervisor(监督器) 和 worker(工作任务) 的树,**When(当)** root(根节点) 启动,**Then(则)** 所有节点按声明顺序启动,并在 current state(当前状态) 中显示父子关系.
2. **Given(假设)** 一个 child(子任务) 失败并触发上报,**When(当)** 父 supervisor(监督器) 收到事件,**Then(则)** 事件的 `Where`(何处) 信息明确包含 supervisor path(监督器路径),child id(子任务标识) 和 parent id(父标识).
3. **Given(假设)** 一个 supervisor(监督器) 正在关闭包含多个 child(子任务) 的树,**When(当)** 关闭流程开始,**Then(则)** 系统必须按声明顺序的逆序关闭 child(子任务).

---

### User Story 3(用户故事三) - 应用重启,退避和熔断策略 (Priority(优先级): P3)

维护者需要用核心枚举表达 `SupervisionStrategy`(监督策略),`RestartPolicy`(重启策略),`BackoffPolicy`(退避策略) 和 `MeltdownPolicy`(熔断策略),让策略引擎根据 `ExitReason`(退出原因) 和错误类别做决定.

**Why this priority(为什么是这个优先级)**: supervisor(监督器) 不能靠硬编码业务逻辑重启任务.策略必须可声明,可测试,可审计,并且必须和退出原因解耦.

**Independent Test(独立测试)**: 测试让 child(子任务) 在不同策略下正常退出,失败,panic(恐慌),timeout(超时) 和 unhealthy(不健康),然后验证策略引擎分别返回不重启,延迟重启,隔离,向父级升级或关闭整棵树.

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** `OneForOne`(一对一) 策略,**When(当)** 一个 child(子任务) 失败,**Then(则)** 系统只重启失败的 child(子任务).
2. **Given(假设)** `OneForAll`(一对全部) 策略,**When(当)** 任意 child(子任务) 失败,**Then(则)** 同组所有 child(子任务) 先停止,再按定义顺序重启.
3. **Given(假设)** `RestForOne`(从失败处开始) 策略,**When(当)** 一个 child(子任务) 失败,**Then(则)** 失败 child(子任务) 之前的 child(子任务) 不重启,失败 child(子任务) 以及其后按定义顺序启动的 child(子任务) 一起重启.

---

### User Story 4(用户故事四) - 治理健康状态和运行时控制 (Priority(优先级): P4)

操作者需要通过 `SupervisorHandle`(监督器句柄) 在运行时治理 child(子任务) 和 supervisor tree(监督树).命令必须幂等,并且每个控制命令必须产生审计事件.

运行时操作必须包含下面列表:

- `add_child`: 向目标 supervisor(监督器) 添加一个新的 child(子任务),并在校验通过后按依赖和启动顺序接入 registry(注册表),current state(当前状态),event stream(事件流) 和 observability pipeline(可观测性管线).
- `remove_child`: 从目标 supervisor(监督器) 移除一个 child(子任务),并先执行关闭协议,再删除 registry(注册表) 记录,同时保留审计事件和最终 current state(当前状态).
- `restart_child`: 对目标 child(子任务) 发起受策略约束的重启,并记录触发原因,attempt(尝试次数),generation(代次),backoff(退避) 和 restart decision(重启决策).
- `pause_child`: 暂停目标 child(子任务) 的运行治理,使其不再被自动重启或健康推进,并在 current state(当前状态) 中显示 `Paused`(已暂停) 状态.
- `resume_child`: 恢复已暂停 child(子任务) 的运行治理,并按原策略重新进入 running(运行中),ready(已就绪) 或需要重启的状态.
- `quarantine_child`: 把目标 child(子任务) 放入 quarantine(隔离) 终态,阻止自动重启,并要求操作者显式介入.
- `shutdown_tree`: 对目标 supervisor tree(监督树) 执行 request stop(请求停止),graceful drain(优雅排空),abort stragglers(强制终止拖尾任务) 和 reconcile(状态对账) 四阶段关闭.
- `current_state`: 返回目标范围内最新 `SupervisorState`(监督器状态),用于回答当前真实状态,不得被当作事件历史.
- `subscribe_events`: 订阅目标范围内的生命周期事件流,用于观察 `When`(何时),`Where`(何处),`What`(发生内容),策略决定和审计记录.

**Why this priority(为什么是这个优先级)**: 工业级 supervisor(监督器) 必须能在运行时治理任务,而不是只能在启动时被动等待失败.

**Independent Test(独立测试)**: 测试对同一个 child(子任务) 重复执行 shutdown(关闭),pause(暂停),resume(恢复) 和 quarantine(隔离),然后验证命令返回当前状态或成功结果,不产生不可恢复错误,并且每次命令都有审计记录.

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** 一个已停止 child(子任务),**When(当)** 操作者重复请求 shutdown(关闭),**Then(则)** supervisor(监督器) 返回当前停止状态,并产生 command event(命令事件).
2. **Given(假设)** 一个运行中 child(子任务) 停止发送 heartbeat(心跳),**When(当)** 超过 `stale_after`,**Then(则)** supervisor(监督器) 把它判定为 unhealthy(不健康),并按策略处理.

---

### User Story 5(用户故事五) - 关闭后不留下孤儿任务 (Priority(优先级): P5)

操作者需要 root shutdown(根关闭) 触发 shutdown protocol(关闭协议).该协议对外保持 cancel-then-abort(先取消后强制终止) 边界,对内包含 request stop(请求停止),graceful drain(优雅排空),abort stragglers(强制终止拖尾任务) 和 reconcile(状态对账) 四个阶段.父 token(令牌) 取消必须传播到 child token(子令牌),child token(子令牌) 取消不能反向取消父 token(令牌).关闭完成后不能留下 orphan task(孤儿任务).

**Why this priority(为什么是这个优先级)**: supervisor(监督器) 必须可靠收尾.关闭不清晰会导致资源泄漏,任务悬挂和不可解释的退出结果.

**Independent Test(独立测试)**: 测试启动多个长运行 child(子任务) 后请求 root shutdown(根关闭),然后验证所有 child token(子令牌) 被取消,所有任务集合为空,超时 child(子任务) 被第二阶段终止,并产生关闭请求和关闭完成事件.

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** 多个正在运行的 child(子任务),**When(当)** 操作者请求 root shutdown(根关闭),**Then(则)** 每个 child(子任务) 的取消令牌被触发,并在完成后报告关闭完成.
2. **Given(假设)** 一个 child(子任务) 超过 graceful timeout(优雅关闭超时),**When(当)** 第一阶段等待结束,**Then(则)** supervisor(监督器) 进入第二阶段强制终止该 child(子任务),并报告超时原因.
3. **Given(假设)** root shutdown(根关闭) 已完成,**When(当)** 操作者检查运行时任务集合,**Then(则)** 不存在 orphan task(孤儿任务),所有 child(子任务) 都处于 terminal(终态) 或 quarantined(隔离) 状态,并且 registry(注册表),current state(当前状态),metrics(指标) 和 event journal(事件日志缓冲区) 已完成 reconcile(状态对账).
4. **Given(假设)** 一个 child(子任务) 使用 `spawn_blocking`(阻塞任务启动),**When(当)** root shutdown(根关闭) 超时,**Then(则)** supervisor(监督器) 不得假设 `abort`(强制终止) 一定有效,必须按独立 `TaskKind`(任务类型),关闭策略和升级策略处理.

---

### User Story 6(用户故事六) - 观测,审计并回放生命周期 (Priority(优先级): P6)

维护者需要同时获得最新 current state(当前状态),完整生命周期事件流和 observability(可观测性) 信号.状态平面保存最新 `SupervisorState`(监督器状态),事件平面发布完整 `SupervisorEvent`(监督器事件),observability pipeline(可观测性管线) 必须输出 structured log(结构化日志),`tracing`(结构化追踪),metrics(指标),audit log(审计日志) 和 subscriber(订阅者) 信号.

**Why this priority(为什么是这个优先级)**: 这个 supervisor(监督器) 的核心不是自动重启,而是可解释的生命周期治理.事故排查必须知道 `When`(何时),`Where`(何处),`What`(发生内容).

**Independent Test(独立测试)**: 测试让 child(子任务) 经历启动,心跳,失败,退避,重启,隔离和关闭,验证每次状态迁移都有事件,current state(当前状态) 能查询最新状态,structured log(结构化日志),tracing(结构化追踪),metrics(指标) 和审计日志反映同一事实.

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** 任意一次状态迁移,**When(当)** 事件被发布,**Then(则)** 事件包含 `When`(何时),`Where`(何处),`What`(发生内容),sequence(序号),correlation id(关联标识) 和策略决定.
2. **Given(假设)** 一个 child attempt(子任务尝试) 开始,**When(当)** 任务运行,**Then(则)** 该 attempt(尝试) 有自己的 tracing span(追踪范围),状态迁移有 tracing event(追踪事件).
3. **Given(假设)** supervisor(监督器) 发生 meltdown(熔断) 或关闭超时,**When(当)** 操作者读取诊断输出,**Then(则)** 系统提供 `RunSummary`(运行摘要),并包含最近 event journal(事件日志缓冲区) 中的关键生命周期事件.
4. **Given(假设)** 任意生命周期状态迁移,**When(当)** observability pipeline(可观测性管线) 收到该迁移,**Then(则)** structured log(结构化日志),tracing event(追踪事件),metrics(指标) 更新和 `SupervisorEvent`(监督器事件) 必须共享同一个 correlation id(关联标识) 或 sequence(序号).

---

### User Story 7(用户故事七) - 使用集中配置,示例和双语文档接入 (Priority(优先级): P7)

使用者需要通过 rust-config-tree(集中配置树) 加载一个集中化配置,运行 examples(示例程序),并阅读 Chinese/English bilingual manual(中英双语手册) 和 docs(文档) 来学习 supervisor(监督器).配置,示例,手册和文档必须跟代码保持同步.

**Why this priority(为什么是这个优先级)**: supervisor(监督器) 的策略,关闭,可观测性和配置项很多.如果配置分散或文档落后,使用者会误用默认值,复制过期示例,或者无法解释运行时行为.

**Independent Test(独立测试)**: 测试从 rust-config-tree(集中配置树) 加载配置,生成 `SupervisorSpec`(监督器规格),运行示例程序,并验证手册,文档,quickstart(快速开始),契约和示例中公开 API(接口) 名称一致.

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** 一个包含 include tree(包含树) 的 supervisor(监督器) 配置,**When(当)** 系统加载配置,**Then(则)** 所有策略默认值,child(子任务) 定义,可观测性选项,关闭预算,容量和运行时可调常量都来自同一个 config state(配置状态).
2. **Given(假设)** 代码尝试在模块内硬编码可调配置或 runtime tunable constant(运行时可调常量),**When(当)** 维护者运行配置边界检查,**Then(则)** 该实现不满足本功能规格,因为配置值必须集中在 rust-config-tree(集中配置树) 边界.
3. **Given(假设)** 使用者学习本项目,**When(当)** 他运行 `examples/` 目录中的示例程序,**Then(则)** 示例必须覆盖 quickstart(快速开始),集中配置,重启策略,关闭协议和可观测性场景.
4. **Given(假设)** public API(公开接口),configuration schema(配置模式),example behavior(示例行为) 或 observability signal(可观测性信号) 发生变化,**When(当)** 维护者提交代码,**Then(则)** 中英文手册,中英文文档,quickstart(快速开始),公开契约和示例程序必须同步更新.

---

### User Story 8(用户故事八) - 遵守编码和发布约定 (Priority(优先级): P8)

维护者需要在编码阶段就保持完整 code documentation(代码文档),清晰 top-level directory module(顶层目录模块) 结构,可解释 module dependency map(模块依赖图),稳定 import rule(导入规则),并确保 crate package(软件包) 符合 crates.io(软件包发布平台) 发布约定.核心模块必须直接位于 `src/<module>/`,不得保留 `src/supervision/` 中间层,也不得使用 `src/<module>.rs` 平铺模块文件.模块入口文件只能声明子模块,不得重导出或隐藏真实所有权边界.implementation phase(实现阶段) 必须以 parallel workstream(并行工作流) 和 unattended implementation(无人值守实现) 推进到所有任务完成,必须消除影响 parallel execution(并行执行) 的卡点,并且必须由 lead agent(主代理) 持续监督 subagent(子代理) 输出和及时纠偏.发布准备必须产生 SBOM(软件物料清单),用于说明 package(软件包) 和 dependency(依赖) 组成.

**Why this priority(为什么是这个优先级)**: supervisor(监督器) 是基础库.如果注释缺失,模块入口混入重导出,导入路径依赖相对层级,或者发布元数据不完整,使用者会更难理解 API(接口),docs.rs(文档托管平台) 也无法稳定呈现库表面.

**Independent Test(独立测试)**: 测试检查核心模块直接位于 `src/<module>/`,不存在 `src/supervision/` 中间层,不存在 `src/<module>.rs` 平铺模块文件,每个源文件有 module doc(模块文档),每个 struct(结构体),field(字段),public function(公共函数) 和 private function(私有函数) 有文档,`src/lib.rs` 只包含 crate doc(包文档) 和顶层 `pub mod <mod_name>;` 声明,每个 `src/<module>/mod.rs` 只包含 `pub mod <mod_name>;` 声明,源码导入不使用 `super::`,module dependency map(模块依赖图) 不存在循环依赖,parallel workstream(并行工作流) 不存在同文件写入冲突,parallel execution blocker(并行执行卡点) 都有消除记录,lead agent supervision record(主代理监督记录) 覆盖全部 subagent workstream(子代理工作流),SBOM(软件物料清单) 可以生成并通过格式校验,并且 `cargo package --list` 与 `cargo publish --dry-run` 通过.

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** 维护者新增一个模块,**When(当)** code documentation check(代码文档检查) 运行,**Then(则)** 模块,结构体,结构体字段,公共函数和私有函数都必须有明确文档,公共函数在可运行时必须有 doctest(文档测试).
2. **Given(假设)** 维护者新增核心模块,**When(当)** source layout check(源码布局检查) 运行,**Then(则)** 该模块必须直接位于 `src/<module>/`,并包含自己的 `mod.rs` 和 `tests/*_test.rs`,不得放在 `src/supervision/` 下,也不得以 `src/<module>.rs` 平铺文件形式存在.
3. **Given(假设)** 维护者修改 `src/lib.rs` 或 `src/<module>/mod.rs`,**When(当)** module boundary check(模块边界检查) 运行,**Then(则)** `src/lib.rs` 只能包含 crate doc(包文档) 和顶层 `pub mod <mod_name>;` 声明,`src/<module>/mod.rs` 只能出现 `pub mod <mod_name>;` 形式的模块声明,不得出现 `pub use`(公开重导出),类型定义,函数定义或其它逻辑.
4. **Given(假设)** 维护者在模块内导入项目内部类型,**When(当)** import rule check(导入规则检查) 运行,**Then(则)** 内部导入必须使用 `crate::` absolute path(绝对路径),不得使用 `super::` relative path(相对路径).
5. **Given(假设)** 维护者准备发布 crate(包),**When(当)** release readiness check(发布就绪检查) 运行,**Then(则)** `Cargo.toml` 必须包含 crates.io(软件包发布平台) 需要的发布元数据,README(说明文档),LICENSE(许可证),CHANGELOG(变更日志),SBOM(软件物料清单) 和 package contents(打包内容) 必须可验证,并且 `cargo publish --dry-run` 必须通过.
6. **Given(假设)** 维护者审查模块结构,**When(当)** module dependency check(模块依赖检查) 运行,**Then(则)** 每个模块必须说明 owner module(所有者模块),dependent module(依赖模块),dependency direction(依赖方向),允许依赖和禁止依赖,并且不得出现 cycle dependency(循环依赖).
7. **Given(假设)** 一个任务会让多个开发者同时修改同一文件,共享大文件或跨多个职责边界等待,**When(当)** parallelization check(并行化检查) 运行,**Then(则)** 该任务必须拆分为多个 parallel workstream(并行工作流),每个 workstream(工作流) 都有独立主文件,独立 `_test.rs` 测试文件和可单独验收结果.
8. **Given(假设)** implementation phase(实现阶段) 已经开始,**When(当)** 某个 workstream(工作流) 完成一个局部任务,**Then(则)** 执行过程必须继续调度其它 pending task(待处理任务) 或 in-progress task(进行中任务),直到 task completion ledger(任务完成台账) 证明所有任务完成且所有验收检查通过.
9. **Given(假设)** 一个 workstream(工作流) 被 shared file bottleneck(共享文件瓶颈),unstable contract(不稳定契约),blocking dependency(阻塞依赖) 或 manual gate(人工门禁) 阻塞,**When(当)** blocker elimination check(卡点消除检查) 运行,**Then(则)** 系统必须要求拆分文件边界,稳定公开契约,调整依赖顺序或移除人工等待,并记录 blocker elimination record(卡点消除记录).
10. **Given(假设)** 一个 subagent(子代理) 正在实现某个 workstream(工作流),**When(当)** lead agent(主代理) 发现 subagent output(子代理输出) 偏离规格,模块边界,依赖规则,测试规则,文档同步规则或禁止兼容规则,**Then(则)** lead agent(主代理) 必须记录 development drift(开发偏差),下达 correction action(纠偏动作),复核修正结果,并且不得在纠偏闭环前把该 workstream(工作流) 标记为完成.

### Edge Cases(边界情况)

- child(子任务) 在启动阶段立即失败或 panic(恐慌) 时,supervisor(监督器) 仍然必须记录失败阶段,attempt(尝试次数),generation(代次) 和重启决策.
- child(子任务) 正常完成且策略为 `Transient`(瞬时) 或 `Temporary`(临时) 时,supervisor(监督器) 不得错误重启.
- child(子任务) 请求关闭时,supervisor(监督器) 必须把它识别为 `Cancelled`(已取消) 或请求关闭结果,而不是普通失败.
- 同一个 child(子任务) 在配置的重启窗口内超过配置的最大重启次数时,该 child(子任务) 必须进入 `Quarantined`(已隔离),并且不得继续重启.
- 同一个 supervisor(监督器) 在配置的熔断窗口内超过配置的最大失败次数时,supervisor(监督器) 必须向父 supervisor(监督器) 上报 `Meltdown`(熔断).
- `OneForAll`(一对全部) 重启组内任务时,系统必须先停止整组,再按定义顺序重启,不能交叉启动和停止.
- `RestForOne`(从失败处开始) 重启时,失败 child(子任务) 之前的 child(子任务) 不得受影响.
- 生命周期事件消费者落后或溢出时,current state(当前状态) 仍然必须准确,并记录 `supervisor_event_lag_total`.
- 测试退避,超时,心跳和熔断窗口时,测试不得依赖真实 sleep(睡眠),必须能用 paused time(暂停时间) 推进.
- 高频业务消息不得每条都经过 supervisor(监督器).supervisor(监督器) 只管理生命周期,健康,控制命令和低频事件.
- control plane(控制面) 不得承载业务 data plane(数据面) 消息处理;业务任务只能通过生命周期,状态,事件和控制命令与 supervisor(监督器) 交互.
- blocking task(阻塞任务) 关闭超时时,supervisor(监督器) 必须记录不可立即终止的边界,并按策略升级,而不能把它当作普通 async task(异步任务).
- 指标标签不得包含错误全文,动态路径碎片,用户输入或其它无界值.
- observability(可观测性) 信号消费者缺失或滞后时,supervisor(监督器) 不得阻塞生命周期治理,并必须通过 test recorder(测试记录器) 可验证地记录丢弃或滞后情况.
- 配置文件 include tree(包含树) 中出现重复 child id(子任务标识),无效路径或非法策略值时,系统必须拒绝该 config state(配置状态),并不得启动部分树.
- 配置文件缺少任何必需 runtime tunable constant(运行时可调常量) 时,系统必须拒绝该 config state(配置状态),不得使用代码中的硬编码值或隐式回退值补齐.
- 示例程序不能依赖隐藏的本地环境.每个示例必须能通过 `cargo run --example <name>` 独立运行或明确说明所需输入文件.
- 文档与代码不一致时,同步检查必须失败,而不是把过期文档留到后续修补.
- 核心源码出现 `src/supervision/` 中间层,出现 `src/<module>.rs` 平铺模块文件,或模块缺少 `src/<module>/mod.rs` 时,source layout check(源码布局检查) 必须失败.
- `src/lib.rs` 出现 crate doc(包文档) 和顶层 `pub mod <mod_name>;` 声明之外的类型定义,函数定义,常量定义,逻辑代码或 `pub use`(公开重导出) 时,module boundary check(模块边界检查) 必须失败.
- `src/<module>/mod.rs`(模块入口文件) 出现 `pub use`(公开重导出),类型定义,函数定义,常量定义或逻辑代码时,模块边界检查必须失败.
- 源码使用 `super::` 或其它相对模块导入时,导入规则检查必须失败.
- 代码缺少 module doc(模块文档),struct doc(结构体文档),field doc(字段文档),public function doc(公共函数文档) 或 private function doc(私有函数文档) 时,代码文档检查必须失败.
- crates.io(软件包发布平台) 发布元数据缺失,package contents(打包内容) 包含不该发布的大文件,或 `cargo publish --dry-run` 失败时,发布就绪检查必须失败.
- SBOM(软件物料清单) 缺失,格式无效,缺少 crate(包) 本身,缺少直接依赖,或和 `Cargo.lock` 依赖版本不一致时,SBOM check(SBOM 检查) 必须失败.
- 源码,示例,契约或文档中出现任何以 `Snapshot` 或 `View` 结尾的代码命名,出现 `snapshot()` 运行时查询方法,或出现 `state_view` 模块名时,naming check(命名检查) 必须失败.正式命名必须使用 `ConfigState`(配置状态),`SupervisorState`(监督器状态),`ChildState`(子任务状态),`current_state`(当前状态) 和 `state`(状态).
- 源码,示例,契约或文档中出现旧接口别名,迁移层,历史行为保留开关,废弃 facade(门面),兼容包装函数或第三方 API(接口) 形状复制时,compatibility method check(兼容方法检查) 必须失败.
- 任何测试文件路径不以 `_test.rs` 结尾时,test naming check(测试命名检查) 必须失败.
- rust-config-tree(集中配置树) 配置示例,quickstart(快速开始),契约或任务使用 TOML(配置格式),JSON(数据交换格式) 或其它格式作为主配置格式时,configuration format check(配置格式检查) 必须失败.正式格式必须是 YAML(数据序列化格式).
- 源码中出现硬编码的运行时可调常量值,例如重启阈值,熔断窗口,退避时长,抖动比例,心跳间隔,关闭超时,事件日志容量,指标开关或审计开关时,configuration boundary check(配置边界检查) 必须失败.这些值必须来自 rust-config-tree(集中配置树) v0.1.9 的 YAML(数据序列化格式) 配置.
- 规格,计划,数据模型,公开契约,quickstart(快速开始) 或任务清单中出现未登记到 `glossary.md`(词汇表) 的专业词汇或反引号词汇时,glossary coverage check(词汇表覆盖检查) 必须失败.
- 文档或任务中出现含糊的 `No-Orphan Shutdown` 命名时,术语检查必须失败.正式术语必须使用 `Shutdown Without Orphaned Tasks`(关闭后不留下孤儿任务).
- 任何函数的 cognitive complexity(认知复杂度) 超过阈值时,复杂度检查必须失败.普通函数默认阈值为 15,生命周期调度函数默认阈值为 20,控制流嵌套不得超过 3 层.
- 任何模块同时承担互不相关职责,跨模块访问内部状态,公共 API(公开接口) 绕过契约类型,或新增行为没有对应测试和文档时,maintainability check(可维护性检查) 必须失败.
- module dependency map(模块依赖图) 出现 cycle dependency(循环依赖),反向依赖,跨模块内部访问或未登记依赖时,module dependency check(模块依赖检查) 必须失败.
- 一个实现任务同时修改多个 ownership boundary(所有权边界),要求多个 parallel workstream(并行工作流) 写同一文件,或依赖共享大文件才能继续推进时,parallelization check(并行化检查) 必须失败,并要求继续拆分.
- unattended implementation(无人值守实现) 在仍有 pending task(待处理任务),in-progress task(进行中任务),失败检查或未记录验收结果时停止,implementation completion check(实现完成检查) 必须失败.
- parallel workstream(并行工作流) 被 shared file bottleneck(共享文件瓶颈),unstable contract(不稳定契约),blocking dependency(阻塞依赖),manual gate(人工门禁),long serial validation(长串行验证),unclear owner(负责人不清晰) 或 hidden coupling(隐藏耦合) 阻塞时,blocker elimination check(卡点消除检查) 必须失败,直到卡点被拆分,重排,稳定契约或移除人工等待.
- blocker elimination record(卡点消除记录) 缺少卡点类型,影响范围,消除动作,责任边界,验收证据或剩余风险时,parallelization check(并行化检查) 必须失败.
- lead agent(主代理) 未监督 subagent(子代理) 输出,未识别 development drift(开发偏差),未记录 correction action(纠偏动作),或在 correction loop(纠偏循环) 未闭环前把 workstream(工作流) 标记完成时,lead agent supervision check(主代理监督检查) 必须失败.
- subagent(子代理) 修改不属于自己 ownership boundary(所有权边界) 的文件,绕过 module dependency map(模块依赖图),新增 compatibility method(兼容方法),破坏测试命名或遗漏文档同步时,lead agent(主代理) 必须在同一 implementation cycle(实现周期) 中纠偏.

## Requirements(需求) *(mandatory(必填))*

### Functional Requirements(功能需求)

- **FR-001**: 系统必须为每个 child(子任务) 提供声明式 `ChildSpec`(子任务规格),并包含 `id`,`name`,`kind`,`restart_policy`,`shutdown_policy`,`health_policy`,`readiness_policy`,`backoff_policy`,`dependencies`,`tags` 和 `criticality`.
- **FR-002**: 系统必须防止被监督后台工作以分散且无人管理的 spawn(启动任务) 表达;受生命周期治理的工作必须通过 child(子任务) 规格进入系统.
- **FR-003**: 系统必须支持 `TaskFactory`(任务工厂) 模型,使每次重启都构造新的任务尝试;必须跨重启保留的状态需要显式放入共享状态,持久化存储或状态仓库.
- **FR-004**: 系统必须支持 `TaskContext`(任务上下文),其中包含 child(子任务) 身份,supervisor path(监督器路径),generation(代次),attempt(尝试次数),cancellation token(取消令牌),event sink(事件接收点) 和 heartbeat(心跳) 接口.
- **FR-005**: 系统必须支持 `SupervisorTree`(监督树),其中 root supervisor(根监督器) 可以包含子 supervisor(监督器) 和 worker(工作任务).
- **FR-006**: 系统必须为每个受监督节点分配稳定路径,例如 `/root/market/binance_ws`.
- **FR-007**: 系统必须把 `OneForOne`(一对一),`OneForAll`(一对全部) 和 `RestForOne`(从失败处开始) 作为核心 supervision strategy(监督策略).
- **FR-008**: 系统必须把 `Permanent`(永久),`Transient`(瞬时) 和 `Temporary`(临时) 作为核心 restart policy(重启策略).
- **FR-009**: 系统必须把 restart policy(重启策略) 和 `ExitReason`(退出原因) 分开建模.
- **FR-010**: 系统必须把任务退出分类为 completed(已完成),failed(已失败),cancelled(已取消),timed out(已超时),unhealthy(不健康) 或 panicked(已恐慌).
- **FR-011**: 系统必须把任务失败分类为 `Recoverable`(可恢复),`FatalConfig`(致命配置错误),`FatalBug`(致命代码错误),`ExternalDependency`(外部依赖错误),`Timeout`(超时),`Panic`(恐慌) 和 `Cancelled`(已取消).
- **FR-012**: 系统必须使用失败类别,退出原因,criticality(关键程度),restart policy(重启策略),meltdown policy(熔断策略) 和当前状态来做重启决策.
- **FR-013**: 系统必须支持 child-level fuse(子任务级熔断).同一个 child(子任务) 是否进入 quarantine(隔离) 必须由 rust-config-tree(集中配置树) 配置中的重启窗口和最大重启次数决定,不得由代码硬编码决定.
- **FR-014**: 系统必须支持 supervisor-level fuse(监督器级熔断).同一个 supervisor(监督器) 是否报告 meltdown(熔断) 必须由 rust-config-tree(集中配置树) 配置中的熔断窗口和最大失败次数决定,不得由代码硬编码决定.
- **FR-015**: 系统必须支持 reset-after(稳定后重置) 语义,使稳定运行可以重置重启计数和熔断计数.
- **FR-016**: 系统必须支持 exponential backoff with jitter(带随机抖动的指数退避).初始退避,最大退避,jitter(抖动) 比例和 reset_after(重置时间) 必须全部来自 rust-config-tree(集中配置树) 配置,不得作为代码硬编码常量存在.
- **FR-017**: 系统必须允许测试关闭 jitter(抖动),使退避断言保持确定.
- **FR-018**: 系统必须支持基于 heartbeat(心跳) 的健康检查,而不能只检查任务是否仍在运行.
- **FR-019**: 系统必须在 `stale_after` 内没有收到 heartbeat(心跳) 时把 child(子任务) 标记为 unhealthy(不健康).heartbeat interval(心跳间隔) 和 stale threshold(过期阈值) 必须全部来自 rust-config-tree(集中配置树) 配置,不得作为代码硬编码常量存在.
- **FR-020**: 系统必须支持 cancel-then-abort shutdown boundary(先取消后强制终止的关闭边界):先取消并等待 graceful timeout(优雅关闭超时),超时后才 abort(强制终止),并且内部流程必须满足 FR-045 的四阶段要求.
- **FR-021**: 系统必须把取消从父 token(令牌) 传播到 child token(子令牌),并且不得要求 child token(子令牌) 取消父 token(令牌).
- **FR-022**: 系统必须保证 root shutdown(根关闭) 后不留下 orphan task(孤儿任务).
- **FR-023**: 系统必须提供 `SupervisorHandle`(监督器句柄) 命令:`add_child`,`remove_child`,`restart_child`,`pause_child`,`resume_child`,`quarantine_child`,`shutdown_tree`,`current_state` 和 `subscribe_events`.
- **FR-024**: 系统必须让控制命令保持幂等;对已停止,已暂停,已恢复或已隔离 child(子任务) 重复执行命令时,必须返回当前状态或已接受结果.
- **FR-025**: 系统必须把最新 current state(当前状态) 和生命周期事件历史分开保存.
- **FR-026**: 系统必须通过适合 watch-style receiver(观察式接收者) 的状态平面暴露最新状态,使只需要最新值的消费者可以读取它.
- **FR-027**: 系统必须通过适合 broadcast subscriber(广播订阅者) 或自定义 fan-out(扇出) 机制的事件总线暴露完整生命周期历史.
- **FR-028**: 系统必须用 `When`(何时),`Where`(何处) 和 `What`(发生内容) 字段建模生命周期事件.
- **FR-029**: 系统必须在 `When`(何时) 数据中包含 wall time(墙钟时间),monotonic time(单调时间),sequence(序号),attempt(尝试次数) 和 generation(代次).
- **FR-030**: 系统必须在可用时把 supervisor path(监督器路径),child id(子任务标识),parent id(父标识),task name(任务名称),task id(任务标识),host(主机),pid(进程标识),thread name(线程名称) 和 registration location(注册位置) 放入 `Where`(何处) 数据.
- **FR-031**: 系统必须在适用时把 event type(事件类型),state transition(状态迁移),exit reason(退出原因),error category(错误类别),restart decision(重启决策),backoff duration(退避时长),health status(健康状态) 和 triggering command(触发命令) 放入 `What`(发生内容) 数据.
- **FR-032**: 系统必须为每次状态迁移创建 observability signal(可观测性信号),并至少覆盖 lifecycle event(生命周期事件),structured log(结构化日志),tracing event(追踪事件) 和适用的 metrics(指标) 更新.
- **FR-033**: 系统必须以 `tracing`(结构化追踪) 作为结构化观察基础.每个 child attempt(子任务尝试) 必须有自己的 span(追踪范围),每次状态迁移必须发出 event(追踪事件).
- **FR-034**: 系统必须至少导出这些 metrics(指标):`supervisor_restart_total`,`supervisor_child_state`,`supervisor_child_uptime_seconds`,`supervisor_backoff_seconds`,`supervisor_healthcheck_latency_seconds`,`supervisor_meltdown_total`,`supervisor_shutdown_duration_seconds` 和 `supervisor_event_lag_total`.
- **FR-035**: 系统必须把 supervisor(监督器) 工作移出 business hot path(业务热路径);高频交易,盘口和撮合逻辑不得让每条消息都经过 supervisor(监督器).
- **FR-036**: 系统必须支持 test time(测试时间),使 backoff(退避),timeout(超时),heartbeat(心跳) 和 meltdown window(熔断窗口) 测试可以确定性推进虚拟时间.
- **FR-037**: 系统必须为每个控制命令产生 audit command event(审计命令事件),并包含 `command_id`,`requested_by`,`reason`,`target_path`,`accepted_at` 和 `result`.
- **FR-038**: 系统不得把 actor-model(参与者模型) 要求引入用户可见模型;监督必须用 children(子任务),trees(树),policies(策略),outcomes(结果),events(事件),handles(句柄),states(状态) 和 commands(命令) 表达.
- **FR-039**: 系统不得采用任何 compatibility method(兼容方法).本项目是全新开发项目,没有历史包袱,不得添加旧接口别名,迁移层,包装旧 API(接口),历史行为保留开关,废弃 facade(门面),兼容模块或第三方 API(接口) 形状复制.
- **FR-040**: 系统必须只把 `supertrees` 当作概念输入,不能把它作为生产核心依赖.
- **FR-041**: 系统必须明确分离 control plane(控制面) 和 data plane(数据面).supervisor(监督器) 只管理生命周期,状态,事件,重启,关闭和控制命令,业务消息处理必须留在 data plane(数据面).
- **FR-042**: 系统必须按声明顺序启动 child(子任务),并按声明顺序的逆序关闭 child(子任务).
- **FR-043**: 系统必须把 readiness(就绪) 建模为一等生命周期信号.需要预热,连接建立或订阅恢复的 child(子任务) 必须显式报告 ready(已就绪),默认立即就绪只能作为明确策略存在.
- **FR-044**: 系统必须单独建模 `spawn_blocking`(阻塞任务启动) 和其它 blocking task(阻塞任务).blocking task(阻塞任务) 必须有独立 `TaskKind`(任务类型),关闭策略和升级策略,并且不得复用普通 async task(异步任务) 可强制终止的假设.
- **FR-045**: 系统的关闭协议必须包含 request stop(请求停止),graceful drain(优雅排空),abort stragglers(强制终止拖尾任务) 和 reconcile(状态对账) 四个内部阶段.reconcile(状态对账) 必须统一更新 registry(注册表),current state(当前状态),metrics(指标) 和 event journal(事件日志缓冲区).
- **FR-046**: 系统必须维护固定容量 event journal(事件日志缓冲区),并在 meltdown(熔断),关闭超时或父级升级时生成 `RunSummary`(运行摘要),用于解释最近生命周期事件,失败原因,重启次数,关闭原因和最终状态.
- **FR-047**: 系统必须限制 metrics label(指标标签) 为低基数值.指标标签可以使用 supervisor path(监督器路径),child id(子任务标识),state(状态),decision(决定) 和 failure category(失败类别),不得包含错误全文,动态路径碎片,用户输入或其它无界值.
- **FR-048**: 系统可以在 `TaskFactory`(任务工厂) 内核之上提供 `Service trait`(服务特征) 和 `service_fn`(函数适配器) 人体工学层,但该适配层不得替换 `TaskFactory`(任务工厂) 内核,也不得引入旧接口别名,迁移层,兼容包装函数或第三方 API(接口) 形状.
- **FR-049**: 系统必须提供项目自有 observability pipeline(可观测性管线),用于统一发送 structured log(结构化日志),tracing span/event(追踪范围和事件),metrics(指标),audit event(审计事件),event journal(事件日志缓冲区) 和 `RunSummary`(运行摘要).该管线必须支持 test recorder(测试记录器),并且不得把核心绑定到具体 exporter(导出器).
- **FR-050**: 系统必须使用 rust-config-tree(集中配置树) v0.1.9 加载 centralized configuration(集中化配置),并从同一个 `ConfigState`(配置状态) 派生 `SupervisorSpec`(监督器规格),策略默认值,可观测性选项,关闭预算,容量,阈值,窗口,超时,开关和其它 runtime tunable constant(运行时可调常量).
- **FR-051**: 系统不得在各模块内分散保存可调配置,也不得在代码中硬编码任何 runtime tunable constant(运行时可调常量).可调配置只能通过 rust-config-tree(集中配置树) 的 YAML(数据序列化格式) 配置,centralized configuration(集中化配置) 加载器和 `ConfigState`(配置状态) 进入系统.
- **FR-052**: 系统必须在 `examples/` 目录提供学习和研究用示例程序,至少覆盖 quickstart(快速开始),rust-config-tree(集中配置树) 加载,重启策略,四阶段关闭和可观测性.
- **FR-053**: 系统必须提供 complete manual(完整手册) 和 docs(文档),并支持 Chinese/English bilingual content(中英双语内容).中英文内容必须保持同构目录和同等语义.
- **FR-054**: 系统必须提供 documentation sync check(文档同步检查),验证代码,public API(公开接口),configuration schema(配置模式),quickstart(快速开始),examples(示例程序),contracts(契约),manual(手册) 和 docs(文档) 不发生漂移.
- **FR-055**: 系统必须在编码阶段要求完整 code documentation(代码文档).每个 module(模块),struct(结构体),struct field(结构体字段),public function(公共函数) 和 private function(私有函数) 都必须有文档.source comment(源码注释) 和 rustdoc(代码文档注释) 必须使用英文.公共函数在可运行时必须提供 doctest(文档测试).
- **FR-056**: 系统必须规定 `src/lib.rs` 只能包含 crate doc(包文档) 和顶层 `pub mod <mod_name>;` 声明,每个 `src/<module>/mod.rs`(模块入口文件) 只能包含 `pub mod <mod_name>;` 形式的模块声明,不得包含 `pub use`(公开重导出),类型定义,函数定义,常量定义或其它逻辑.
- **FR-057**: 系统必须规定所有内部模块导入使用 `crate::` absolute path(绝对路径),外部依赖导入使用 crate name(软件包名) absolute path(绝对路径),不得使用 `super::` 或其它 relative path(相对路径) 表达模块关系.
- **FR-058**: 系统必须符合 crates.io(软件包发布平台) 发布约定.`Cargo.toml` 必须包含 package metadata(软件包元数据),README(说明文档),license(许可证),repository(代码仓库),documentation(文档地址),keywords(关键词),categories(分类) 和明确 package include/exclude(打包包含或排除) 策略.
- **FR-059**: 系统必须提供 release readiness check(发布就绪检查),至少运行 `cargo package --list`,检查 `.crate` package(打包文件) 内容和大小,并运行 `cargo publish --dry-run`.
- **FR-060**: 系统必须控制 cognitive complexity(认知复杂度).普通函数的认知复杂度不得超过 15,生命周期调度函数不得超过 20,控制流嵌套不得超过 3 层.超过阈值的逻辑必须拆分为 state machine(状态机),policy function(策略函数),small helper function(小辅助函数) 或独立模块.
- **FR-061**: 系统必须保证 high maintainability(高可维护性).每个模块必须有单一清晰职责,公开 API(公开接口) 必须通过契约类型表达,共享状态必须集中在运行时边界,行为变化必须有测试,文档和示例同步,并且不得通过全局可变状态,隐式副作用或跨模块内部访问降低可维护性.
- **FR-062**: 系统必须在发布准备阶段生成 SBOM(软件物料清单).SBOM(软件物料清单) 至少必须包含 crate(包) 本身,所有直接依赖,所有传递依赖,版本,license(许可证),package URL(软件包地址),checksum(校验和),source repository(源码仓库) 和生成工具信息,并输出 CycloneDX JSON(CycloneDX JSON 格式) 与 SPDX JSON(SPDX JSON 格式) 两种文件.
- **FR-063**: 系统代码命名不得使用任何 `*Snapshot` 或 `*View` 后缀,也不得提供 `snapshot()` 查询方法.配置加载结果必须命名为 `ConfigState`(配置状态),监督器当前状态必须命名为 `SupervisorState`(监督器状态),子任务当前状态必须命名为 `ChildState`(子任务状态),运行时查询命令必须命名为 `current_state`(当前状态),源码模块必须命名为 `state`(状态),不得命名为 `state_view`(状态视图).
- **FR-064**: 系统必须规定所有测试文件以 `_test.rs` 结尾.integration test(集成测试) 文件必须位于 `src/tests/*_test.rs`,unit test(单元测试) 文件必须位于对应模块自己的 `tests/*_test.rs` 目录,不得使用其它测试文件后缀.
- **FR-065**: 系统必须规定 rust-config-tree(集中配置树) 的主配置格式为 YAML(数据序列化格式).配置示例,quickstart(快速开始),文档,契约和任务必须使用 `*.yaml` 文件,不得把 TOML(配置格式),JSON(数据交换格式) 或其它格式作为主配置格式.
- **FR-066**: 系统必须维护独立 `glossary.md`(词汇表),覆盖规格文档中出现的专业词汇和所有反引号词汇.反引号内的 Rust(编程语言) 类型名,枚举值,方法名,字段名,指标名,路径名,命令名,配置键和测试目标都必须被视为词汇表条目.
- **FR-067**: 系统必须提供 hard-coded constant check(硬编码常量检查),验证源码,示例和测试支持中不存在用于运行时行为的硬编码配置值.允许存在的字面量只能表达类型不变量,枚举名,字段名,错误码或测试输入本身,不能作为生产行为的隐式默认值.
- **FR-068**: 系统必须提供 module dependency map(模块依赖图),说明声明式规格,身份,配置,任务上下文,任务工厂,策略,健康,状态,事件,可观测性,控制命令,注册表,关闭,运行时编排,错误类型,测试支持,示例和文档模块之间的依赖关系.
- **FR-069**: 系统必须规定 module dependency rule(模块依赖规则).基础类型,错误类型,配置状态,公开契约和事件模型可以被上层模块依赖;策略,健康,状态,可观测性和关闭模块只能通过公开契约协作;运行时编排可以组合下层模块;下层模块不得反向依赖运行时编排,控制命令或示例.
- **FR-070**: 系统必须提高 development parallelism(开发并行度).任何影响并行度的实现工作都必须拆分为 parallel workstream(并行工作流),并且每个 workstream(工作流) 必须有独立 owner(负责人),独立主文件,独立 `_test.rs` 测试文件,明确前置依赖,明确交付边界和可单独验收结果.
- **FR-071**: 系统必须规定 implementation phase(实现阶段) 采用 unattended implementation(无人值守实现) 模式.实现执行必须自动选择可以继续推进的 parallel workstream(并行工作流),持续完成 pending task(待处理任务),不得在单个任务完成后停止等待人工继续.
- **FR-072**: 系统必须提供 task completion ledger(任务完成台账),记录每个任务的 workstream(工作流),状态,主文件,测试文件,验收检查,完成证据和剩余阻塞.只有全部任务为 completed task(已完成任务),全部检查通过且没有 pending task(待处理任务) 或 in-progress task(进行中任务) 时,implementation phase(实现阶段) 才能被判定完成.
- **FR-073**: 系统必须提供 blocker elimination check(卡点消除检查),识别并消除影响 parallel execution(并行执行) 的 shared file bottleneck(共享文件瓶颈),unstable contract(不稳定契约),blocking dependency(阻塞依赖),manual gate(人工门禁),long serial validation(长串行验证),unclear owner(负责人不清晰) 和 hidden coupling(隐藏耦合).
- **FR-074**: 系统必须为每个 parallel execution blocker(并行执行卡点) 生成 blocker elimination record(卡点消除记录).记录必须包含 blocker type(卡点类型),affected workstream(受影响工作流),affected file boundary(受影响文件边界),elimination action(消除动作),owner(负责人),acceptance evidence(验收证据) 和 residual risk(剩余风险).
- **FR-075**: 系统必须规定 lead agent(主代理) 在 parallel development(并行开发) 中监督所有 subagent(子代理) workstream(工作流).lead agent(主代理) 必须分派任务边界,审查 subagent output(子代理输出),对照规格,模块依赖图,文件边界,测试规则,文档同步规则和禁止兼容规则识别 development drift(开发偏差).
- **FR-076**: 系统必须提供 correction loop(纠偏循环) 和 correction record(纠偏记录).当 subagent(子代理) 输出出现偏差时,lead agent(主代理) 必须记录 drift type(偏差类型),affected workstream(受影响工作流),affected files(受影响文件),expected requirement(期望要求),actual output(实际输出),correction action(纠偏动作),review result(复核结果) 和 final evidence(最终证据).
- **FR-077**: 系统必须采用 top-level directory module(顶层目录模块) 源码结构.核心模块必须直接位于 `src/<module>/`,不得使用 `src/supervision/` 中间层,不得使用 `src/<module>.rs` 平铺模块文件,每个模块必须在自己的目录内维护 `mod.rs` 和 `tests/*_test.rs`.

### Key Entities(关键实体) *(include if feature involves data(涉及数据时填写))*

- **Supervisor(监督器)**: 运行时治理节点,负责 child(子任务) 注册,策略评估,状态跟踪,事件发送,重启编排和关闭协调.
- **SupervisorTree(监督树)**: 分层结构,其中 root(根节点) 和子 supervisor(监督器) 治理 worker(工作任务) 和嵌套监督范围.
- **ChildSpec(子任务规格)**: 声明式 child(子任务) 配置,包含身份,任务种类,策略,依赖,标签,关键程度和 factory(工厂).
- **SupervisorSpec(监督器规格)**: 声明式 supervisor(监督器) 配置,包含策略,children(子任务集合),fuse policy(熔断策略),默认策略,路径前缀,group strategy(分组策略),per-child override(子任务级覆盖),restart limit(重启次数限制),escalation policy(升级策略) 和 dynamic supervisor policy(动态监督器策略).
- **SupervisorPath(监督器路径)**: 稳定树路径,用于事件,指标,日志,current state(当前状态) 和控制命令.
- **ChildId(子任务标识)**: child(子任务) 在父 supervisor(监督器) 内的稳定唯一标识.
- **TaskFactory(任务工厂)**: 每次启动或重启时构造新任务尝试的工厂.
- **TaskContext(任务上下文)**: 传给任务尝试的上下文,包含身份,路径,代次,尝试次数,取消,心跳和事件接收点.
- **ChildRuntime(子任务运行态)**: 当前运行态记录,包含状态,代次,尝试次数,心跳,join handle(等待句柄),取消令牌,重启计数和最近失败.
- **Registry(任务注册表)**: 当前运行时索引,保存 child(子任务) 规格和运行态.
- **SupervisionStrategy(监督策略)**: 重启范围决定,包含 `OneForOne`(一对一),`OneForAll`(一对全部) 和 `RestForOne`(从失败处开始).
- **GroupStrategy(分组策略)**: 基于 child tag(子任务标签) 约束重启范围的策略覆盖,用于让同一个 supervisor(监督器) 内不同 group(分组) 拥有不同重启范围.
- **ChildStrategyOverride(子任务级覆盖)**: 指定单个 child(子任务) 的 strategy(策略),restart limit(重启次数限制) 和 escalation policy(升级策略),优先级高于 group strategy(分组策略).
- **RestartLimit(重启次数限制)**: 重启计划可使用的最大重启次数和计数窗口,用于约束策略执行而不是替代 restart policy(重启策略).
- **EscalationPolicy(升级策略)**: 本地重启治理无法继续时的后续动作,包含 parent escalation(父级升级),tree shutdown(整棵树关闭) 和 scope quarantine(范围隔离).
- **DynamicSupervisorPolicy(动态监督器策略)**: 控制运行时动态添加 child manifest(子任务清单文本) 的开关和数量上限.
- **StrategyExecutionPlan(策略执行计划)**: child exit(子任务退出) 后由策略,分组,覆盖,预算和升级规则合并得到的单次执行计划.
- **RestartPolicy(重启策略)**: 退出到重启的规则,包含 `Permanent`(永久),`Transient`(瞬时) 和 `Temporary`(临时).
- **BackoffPolicy(退避策略)**: 重启延迟规则,包含指数增长,最大延迟,抖动和稳定后重置.
- **MeltdownPolicy(熔断策略)**: child-level(子任务级) 和 supervisor-level(监督器级) 熔断阈值及重置窗口.
- **HealthPolicy(健康策略)**: heartbeat interval(心跳间隔) 和 stale-after threshold(过期阈值),用于检测不健康任务.
- **ReadinessPolicy(就绪策略)**: 定义 child(子任务) 何时可以从 running(运行中) 进入 ready(已就绪),并支持默认立即就绪和显式就绪两种策略.
- **ShutdownPolicy(关闭策略)**: graceful timeout(优雅关闭超时) 和 abort-after-timeout(超时后强制终止) 行为.
- **TaskKind(任务类型)**: 区分 async worker(异步工作任务),blocking worker(阻塞工作任务) 和 supervisor(监督器),并决定关闭和升级边界.
- **TaskExit(任务退出)**: 退出分类,例如已完成,已失败,已取消,已超时,不健康或已恐慌.
- **TaskFailureKind(任务失败类别)**: 策略引擎使用的类型化错误类别.
- **RestartDecision(重启决策)**: 策略结果,例如不重启,延迟后重启,隔离,向父级升级或关闭整棵树.
- **SupervisorHandle(监督器句柄)**: 运行时控制平面,用于命令,关闭,current state(当前状态) 和事件订阅.
- **ControlCommand(控制命令)**: 可审计的运行时命令,包含请求者,原因,目标路径和结果.
- **SupervisorState(监督器状态)**: 最新监督器状态,包含树,children(子任务集合),健康状态,计数器和终态.
- **SupervisorEvent(监督器事件)**: 完整生命周期记录,携带 `When`(何时),`Where`(何处),`What`(发生内容),策略决定,序号和 correlation id(关联标识).
- **EventJournal(事件日志缓冲区)**: 固定容量生命周期事件缓冲区,用于 meltdown(熔断),关闭超时和父级升级后的诊断回放.
- **RunSummary(运行摘要)**: 运行结束或故障升级时产生的摘要,包含开始时间,结束时间,关闭原因,重启次数,失败列表,最近事件和最终状态.
- **Service(服务特征)**: 建立在 `TaskFactory`(任务工厂) 之上的可选人体工学适配层,用于让调用者以服务对象或 `service_fn`(函数适配器) 形式接入监督器.
- **ObservabilityPipeline(可观测性管线)**: 项目自有的可观测性边界,负责把生命周期事实同步到 structured log(结构化日志),tracing(结构化追踪),metrics(指标),audit(审计),event journal(事件日志缓冲区),`RunSummary`(运行摘要) 和 test recorder(测试记录器).
- **SupervisorConfig(监督器配置)**: rust-config-tree(集中配置树) 加载后的配置模型,包含 supervisor tree(监督树),策略默认值,可观测性选项,关闭预算和示例配置入口.
- **ConfigState(配置状态)**: 一次配置加载的不可变结果,包含 version(版本),checksum(校验和),source tree(来源树) 和派生后的 `SupervisorSpec`(监督器规格).
- **RuntimeConfigurationValue(运行时配置值)**: 通过 rust-config-tree(集中配置树) 进入系统的可调值,包含阈值,窗口,超时,退避,抖动,容量,开关,预算和默认策略值.
- **ExampleSuite(示例套件)**: `examples/` 目录下的学习和研究示例集合,用于展示配置,生命周期,重启,关闭和可观测性.
- **DocumentationSet(文档集合)**: `manual/zh`,`manual/en`,`docs/zh` 和 `docs/en` 中的双语手册与文档集合,必须和代码及示例同步.
- **GlossarySet(词汇表集合)**: `specs/001-create-supervisor-core/glossary.md` 中的专业词汇和反引号词汇集合,用于约束规格,计划,契约,数据模型,quickstart(快速开始),任务,手册和文档的术语一致性.
- **CodingStandard(编码标准)**: 编码阶段必须执行的源码布局,文档,模块入口和导入路径规则,覆盖 top-level directory module(顶层目录模块),英文 source comment(源码注释),英文 rustdoc(代码文档注释),module doc(模块文档),struct doc(结构体文档),field doc(字段文档),function doc(函数文档),`src/lib.rs`,`src/<module>/mod.rs` 和 absolute import(绝对导入).
- **CognitiveComplexityBudget(认知复杂度预算)**: 编码阶段必须执行的复杂度预算,覆盖函数认知复杂度,控制流嵌套层级,模块职责数量和超限拆分记录.
- **MaintainabilityProfile(可维护性画像)**: 编码阶段必须执行的可维护性约束,覆盖 module cohesion(模块内聚),coupling boundary(耦合边界),test coverage(测试覆盖),documentation sync(文档同步),API stability(API 稳定性) 和 change locality(变更局部性).
- **ModuleDependencyMap(模块依赖图)**: 编码阶段必须执行的模块关系约束,覆盖 owner module(所有者模块),dependent module(依赖模块),dependency direction(依赖方向),allowed dependency(允许依赖),forbidden dependency(禁止依赖) 和 cycle dependency(循环依赖) 检查.
- **ParallelWorkstream(并行工作流)**: 影响 development parallelism(开发并行度) 的工作拆分单元,每个单元必须有独立负责人,独立模块边界,独立主文件,独立测试文件和可单独验收结果.
- **WorkstreamSplitRecord(工作流拆分记录)**: 记录串行任务被拆分的原因,拆分后的 workstream(工作流),文件边界,测试边界,前置依赖和验收结果.
- **UnattendedImplementationRun(无人值守实现运行)**: implementation phase(实现阶段) 的执行记录,证明系统按 parallel workstream(并行工作流) 持续推进,直到所有任务完成.
- **TaskCompletionLedger(任务完成台账)**: 实现阶段的完成记录,覆盖任务状态,工作流,文件边界,测试边界,验收证据和剩余阻塞.
- **ParallelExecutionBlocker(并行执行卡点)**: 阻碍 workstream(工作流) 并行推进的问题,包括共享文件,不稳定契约,阻塞依赖,人工门禁,长串行验证,负责人不清晰和隐藏耦合.
- **BlockerEliminationRecord(卡点消除记录)**: 记录并行执行卡点的类型,影响范围,消除动作,负责人,验收证据和剩余风险.
- **LeadAgentSupervision(主代理监督)**: lead agent(主代理) 对 subagent(子代理) 工作进行分派,审查,偏差识别,纠偏和复核的治理过程.
- **SubagentWorkstream(子代理工作流)**: subagent(子代理) 执行的 parallel workstream(并行工作流),必须受模块边界,文件边界,测试边界和验收结果约束.
- **CorrectionRecord(纠偏记录)**: lead agent(主代理) 对 development drift(开发偏差) 的处理证据,包含偏差类型,影响范围,纠偏动作,复核结果和最终证据.
- **ReleasePackage(发布包)**: crates.io(软件包发布平台) 发布前的 package metadata(软件包元数据),README(说明文档),LICENSE(许可证),CHANGELOG(变更日志),package contents(打包内容),package size(打包大小) 和 dry-run result(试运行结果).
- **SBOMArtifact(软件物料清单产物)**: 发布准备阶段生成的依赖清单,包含 CycloneDX JSON(CycloneDX JSON 格式),SPDX JSON(SPDX JSON 格式),生成时间,生成工具,root package(根软件包),直接依赖,传递依赖,license(许可证),checksum(校验和) 和 source reference(来源引用).

## Constitution Alignment(宪章对齐) *(mandatory(必填))*

### Supervision Contract(监督契约)

- **Lifecycle impact(生命周期影响)**: 本功能定义声明,启动,运行,就绪,暂停,恢复,健康检查,失败,重启,隔离,升级,关闭,强制终止,状态对账和报告 child(子任务) 工作的生命周期治理.
- **Failure behavior(失败行为)**: 失败必须类型化,必须关联 child path(子任务路径) 和 attempt(尝试次数),必须经过重启,退避,熔断,关键程度和策略评估,并必须作为可查询事件发送.
- **Shutdown behavior(关闭行为)**: 关闭是一等 shutdown protocol(关闭协议).父取消必须传播到 child token(子令牌),系统必须等待 graceful timeout(优雅关闭超时),只有超时后才 abort(强制终止),root shutdown(根关闭) 必须证明没有 orphan task(孤儿任务),并且必须在 request stop(请求停止),graceful drain(优雅排空),abort stragglers(强制终止拖尾任务) 和 reconcile(状态对账) 四个内部阶段后完成.

### Rust Boundary and Observability Requirements(Rust 边界和可观测性需求)

- **Module ownership(模块所有权)**: 计划必须把声明式规格,身份,任务工厂和上下文,运行时绑定,child runner(子任务运行器),树编排,策略引擎,健康,控制平面,注册表,事件模型,state store(状态存储),可观测性,关闭,错误类型和测试支持拆成独立所有权边界.
- **Source layout(源码布局)**: 核心模块必须直接位于 `src/<module>/`,不得使用 `src/supervision/` 中间层,不得使用 `src/<module>.rs` 平铺模块文件.`src/lib.rs` 是 crate(包) 入口,每个核心模块目录内的 `mod.rs` 是模块入口.
- **Module dependency relationship(模块依赖关系)**: 计划必须说明模块之间的依赖方向.基础类型,错误类型,配置状态,公开契约和事件模型处于下层边界,策略,健康,状态,关闭和可观测性通过公开契约协作,运行时编排只负责组合下层模块,示例和文档只能依赖公开 API(接口),不得反向影响核心模块.
- **Parallel development(并行开发)**: 计划必须把影响并行度的任务拆成 parallel workstream(并行工作流).同一并行组不得要求修改同一主文件,不得把多个职责塞入共享大文件,不得通过跨模块内部访问制造串行等待.
- **Unattended implementation(无人值守实现)**: implementation phase(实现阶段) 必须持续推进所有 parallel workstream(并行工作流),直到 task completion ledger(任务完成台账) 证明全部任务完成,全部验收检查通过,并且没有 pending task(待处理任务) 或 in-progress task(进行中任务).
- **Blocker elimination(卡点消除)**: 计划和实现必须消除影响 parallel execution(并行执行) 的卡点.任何共享文件瓶颈,不稳定契约,阻塞依赖,人工门禁,长串行验证,负责人不清晰或隐藏耦合都必须先拆分,重排,稳定契约或记录清晰消除动作.
- **Lead agent supervision(主代理监督)**: lead agent(主代理) 必须监督 subagent(子代理) 的开发工作,在同一 implementation cycle(实现周期) 中识别偏差,下达纠偏动作,复核结果,并阻止未纠偏的 workstream(工作流) 被标记完成.
- **No compatibility methods(禁止兼容方法)**: 本项目是全新开发项目,没有历史包袱.计划,实现,示例和文档不得提供旧接口别名,迁移层,历史行为保留开关,废弃 facade(门面),兼容包装函数或第三方 API(接口) 形状复制.
- **Test placement(测试位置)**: integration test(集成测试) 必须放在 `src/tests/*_test.rs`,unit test(单元测试) 必须放在对应模块自己的 `tests/*_test.rs`,任务不得要求把测试代码写入实现文件.
- **Configuration boundary(配置边界)**: 所有可调配置和 runtime tunable constant(运行时可调常量) 必须通过 rust-config-tree(集中配置树),YAML(数据序列化格式) 文件和 `ConfigState`(配置状态) 集中进入系统,不得在模块内部散落配置默认值,也不得用代码硬编码值补齐缺失配置.
- **Code documentation(代码文档)**: 编码阶段必须同时完成英文 module doc(模块文档),struct doc(结构体文档),field doc(字段文档),public function doc(公共函数文档),private function doc(私有函数文档),source comment(源码注释) 和可运行 public doctest(公共文档测试).
- **Module entry(模块入口)**: `src/lib.rs` 只能包含 crate doc(包文档) 和顶层 `pub mod <mod_name>;` 声明.每个 `src/<module>/mod.rs`(模块入口文件) 只能包含 `pub mod <mod_name>;` 声明,不得重导出,不得定义类型,函数,常量或逻辑.
- **Import rule(导入规则)**: 内部模块导入必须使用 `crate::` absolute path(绝对路径),不得使用 `super::` relative path(相对路径).
- **Cognitive complexity(认知复杂度)**: 普通函数认知复杂度不得超过 15,生命周期调度函数不得超过 20,控制流嵌套不得超过 3 层.超限逻辑必须拆分为更小函数,状态机或策略对象.
- **Maintainability(可维护性)**: 模块必须高内聚,低耦合,变更局部,测试可定位,文档可追踪.共享可变状态只能出现在明确运行时边界,不得把业务 hot path(业务热路径) 或 data plane(数据面) 逻辑混入 supervisor core(监督器核心).
- **Crates.io readiness(发布就绪)**: 发布前必须满足 crates.io(软件包发布平台) manifest(清单),README(说明文档),LICENSE(许可证),CHANGELOG(变更日志),SBOM(软件物料清单),package list(打包清单) 和 dry-run(试运行) 约定.
- **Naming rule(命名规则)**: 代码命名必须使用 `ConfigState`(配置状态),`SupervisorState`(监督器状态),`ChildState`(子任务状态),`current_state`(当前状态) 和 `state`(状态),不得使用任何 `*Snapshot`,`*View`,`snapshot()` 查询方法或 `state_view` 模块名.
- **Diagnostics(诊断)**: 每次生命周期迁移都必须能通过 current state(当前状态),事件流,event journal(事件日志缓冲区),`RunSummary`(运行摘要),structured log(结构化日志),tracing span(追踪范围),tracing event(追踪事件),指标更新和命令审计记录解释.
- **Dependency impact(依赖影响)**: 计划确认 Tokio(异步运行时) 运行时原语,取消,tracing(结构化追踪),metrics(指标) 和事件 fan-out(扇出) 支持生命周期契约后,可以使用它们.actor framework(参与者框架) 和复制第三方 supervisor(监督器) API(接口) 不在范围内.
- **Glossary coverage(词汇表覆盖)**: 所有专业词汇和反引号词汇都必须登记在 `glossary.md`(词汇表) 中,并保持同一中文说明.
- **Documentation synchronization(文档同步)**: 代码,配置模式,公开契约,示例,quickstart(快速开始),glossary(词汇表),manual(手册) 和 docs(文档) 必须在同一变更中保持同步.

### Chinese Writing(中文写作)

- **Writing language(写作语言)**: 本规格使用中文写作.
- **Term format(术语格式)**: 英文术语以 `English(中文说明)` 形式出现.
- **Forbidden style(禁止风格)**: 本规格不使用非中文正文,片段式语言,生僻词或方言.

## Success Criteria(成功标准) *(mandatory(必填))*

### Measurable Outcomes(可衡量结果)

- **SC-001**: 维护者可以在 15 分钟内通过一个示例声明包含身份,策略,健康,关闭,依赖,标签和关键程度的 child(子任务).
- **SC-002**: 任意 child(子任务) panic(恐慌) 后,系统必须记录 `ChildPanicked`,`BackoffScheduled` 和 `ChildRestarting` 事件,并且必须在重启后的 child(子任务) 运行前递增 attempt(尝试次数).
- **SC-003**: 当测试配置把 child restart window(子任务重启窗口) 配置为 60 秒,并把最大重启次数配置为 10 时,同一个 child(子任务) 第 11 次重启必须进入 `Quarantined`(已隔离),并阻止后续自动重启.
- **SC-004**: 当测试配置把 supervisor failure window(监督器失败窗口) 配置为 60 秒,并把最大失败次数配置为 30 时,同一个 supervisor(监督器) 范围内第 31 次 child(子任务) 失败必须发送 `Meltdown`(熔断) 并向父 supervisor(监督器) 升级.
- **SC-005**: root shutdown(根关闭) 必须取消所有 child token(子令牌),并且在任务集合为空时完成,不留下 orphan task(孤儿任务).
- **SC-006**: 在 `OneForAll`(一对全部) 下,任意 child(子任务) 失败后,系统必须先停止所有 sibling(同级任务),再按定义顺序重启整组.
- **SC-007**: 在 `RestForOne`(从失败处开始) 下,失败 child(子任务) 之前定义的 child(子任务) 不得重启,失败 child(子任务) 和之后的 child(子任务) 必须重启.
- **SC-008**: 每次状态迁移都必须产生一条包含 `When`(何时),`Where`(何处) 和 `What`(发生内容) 字段的事件.
- **SC-009**: current state(当前状态) 必须返回每个 child(子任务) 的当前状态,健康状态,generation(代次),attempt(尝试次数),restart count(重启次数),last failure(最近失败) 和 path(路径).
- **SC-010**: 所有 backoff(退避),timeout(超时),heartbeat(心跳) 和 meltdown(熔断) 测试都必须使用确定的 test time(测试时间),不得依赖真实 sleep(睡眠).
- **SC-011**: 100% 控制命令审计日志必须说明请求者,原因,目标路径,接受时间,command id(命令标识) 和结果.
- **SC-012**: 公开模型必须包含 supervisor tree(监督树),child spec(子任务规格),task factory(任务工厂),policy(策略),health(健康),shutdown(关闭),event(事件),state(状态),metrics(指标),audit(审计) 和 handle(句柄) 概念,并且不得出现 actor-model(参与者模型) 术语.
- **SC-013**: root shutdown(根关闭) 必须按声明顺序的逆序关闭 child(子任务),并在完成后证明 registry(注册表),current state(当前状态),metrics(指标) 和 event journal(事件日志缓冲区) 的最终状态一致.
- **SC-014**: 需要显式 readiness(就绪) 的 child(子任务) 在报告 ready(已就绪) 前,不得在 current state(当前状态) 或 event(事件) 中显示为 ready(已就绪).
- **SC-015**: blocking task(阻塞任务) 在关闭超时后必须产生说明不可立即终止边界的事件和策略决定,并且必须按升级策略处理.
- **SC-016**: 指标导出检查必须验证所有 metrics label(指标标签) 均为低基数值,并拒绝错误全文,用户输入和无界动态值.
- **SC-017**: meltdown(熔断),关闭超时或父级升级发生时,系统必须生成 `RunSummary`(运行摘要),并包含最近 event journal(事件日志缓冲区) 中的关键事件.
- **SC-018**: observability smoke test(可观测性冒烟测试) 必须在同一次生命周期迁移中收集到 `SupervisorEvent`(监督器事件),structured log(结构化日志),tracing event(追踪事件),metrics(指标) 更新和 audit event(审计事件),并验证它们可以通过 correlation id(关联标识) 或 sequence(序号) 关联.
- **SC-019**: centralized configuration test(集中化配置测试) 必须证明 `SupervisorSpec`(监督器规格),默认策略,可观测性选项,关闭预算,阈值,窗口,超时,容量和开关都从同一个 rust-config-tree(集中配置树) v0.1.9 `ConfigState`(配置状态) 派生.
- **SC-020**: examples smoke test(示例冒烟测试) 必须运行 `examples/` 中的 quickstart(快速开始),集中配置,重启策略,四阶段关闭和可观测性示例.
- **SC-021**: bilingual documentation check(双语文档检查) 必须证明 `manual/zh`,`manual/en`,`docs/zh` 和 `docs/en` 的目录结构一致,并覆盖同一组公开概念.
- **SC-022**: documentation sync check(文档同步检查) 必须在 public API(公开接口),configuration schema(配置模式),example behavior(示例行为) 或 observability signal(可观测性信号) 变化但文档未同步时失败.
- **SC-023**: code documentation check(代码文档检查) 必须证明所有 module(模块),struct(结构体),struct field(结构体字段),public function(公共函数) 和 private function(私有函数) 已经有英文文档,所有 source comment(源码注释) 使用英文,并且 public doctest(公共文档测试) 可以运行.
- **SC-024**: module boundary check(模块边界检查) 必须证明 `src/lib.rs` 只包含 crate doc(包文档) 和顶层 `pub mod <mod_name>;` 声明,每个 `src/<module>/mod.rs`(模块入口文件) 只包含 `pub mod <mod_name>;` 声明,并且源码中不存在 `pub use`(公开重导出).
- **SC-025**: import rule check(导入规则检查) 必须证明源码内部导入使用 `crate::` absolute path(绝对路径),并且源码中不存在 `super::` relative path(相对路径).
- **SC-026**: release readiness check(发布就绪检查) 必须证明 crates.io(软件包发布平台) 必需 metadata(元数据),README(说明文档),LICENSE(许可证),CHANGELOG(变更日志),package contents(打包内容),package size(打包大小) 和 `cargo publish --dry-run` 均通过.
- **SC-027**: terminology check(术语检查) 必须证明文档使用 `Shutdown Without Orphaned Tasks`(关闭后不留下孤儿任务),而不是含糊的 `No-Orphan Shutdown`.
- **SC-028**: cognitive complexity check(认知复杂度检查) 必须证明普通函数认知复杂度不超过 15,生命周期调度函数不超过 20,控制流嵌套不超过 3 层,并且每个超限候选都有已完成拆分记录.
- **SC-029**: maintainability check(可维护性检查) 必须证明每个模块只有清晰职责,跨模块依赖只通过公开契约类型发生,行为变化有对应测试和文档,共享可变状态集中在运行时边界,并且新增代码没有把业务 data plane(数据面) 混入 supervisor core(监督器核心).
- **SC-030**: SBOM check(SBOM 检查) 必须证明 `artifacts/sbom/rust-supervisor.cdx.json` 和 `artifacts/sbom/rust-supervisor.spdx.json` 存在,格式有效,包含 crate(包) 本身,直接依赖,传递依赖,license(许可证),checksum(校验和) 和生成工具信息,并且依赖版本与 `Cargo.lock` 一致.
- **SC-031**: naming check(命名检查) 必须证明源码,示例,公开契约和文档中不存在任何 `*Snapshot`,`*View`,`snapshot()` 查询方法或 `state_view` 模块名,并且统一使用 `ConfigState`(配置状态),`SupervisorState`(监督器状态),`ChildState`(子任务状态),`current_state`(当前状态) 和 `state`(状态).
- **SC-032**: test naming check(测试命名检查) 必须证明所有测试文件都以 `_test.rs` 结尾,并且 integration test(集成测试) 只出现在 `src/tests/*_test.rs`,unit test(单元测试) 只出现在模块自己的 `tests/*_test.rs`.
- **SC-033**: YAML configuration check(YAML 配置检查) 必须证明 rust-config-tree(集中配置树) v0.1.9 只通过 `*.yaml` 主配置文件加载 supervisor(监督器) 配置,并且 quickstart(快速开始),示例,契约和文档都不把 TOML(配置格式) 或 JSON(数据交换格式) 作为主配置格式.
- **SC-034**: glossary coverage check(词汇表覆盖检查) 必须证明 `specs/001-create-supervisor-core/glossary.md` 存在,并覆盖规格,计划,数据模型,公开契约,quickstart(快速开始) 和任务清单中的专业词汇以及所有反引号词汇.
- **SC-035**: hard-coded constant check(硬编码常量检查) 必须证明 runtime tunable constant(运行时可调常量) 没有在源码中以 `const`,`static`,字面量回退值或模块局部默认值存在,并且缺失配置会导致配置错误而不是自动使用硬编码值.
- **SC-036**: module dependency check(模块依赖检查) 必须证明 module dependency map(模块依赖图) 覆盖所有核心模块,每条依赖都有方向和理由,并且不存在 cycle dependency(循环依赖),反向依赖或跨模块内部访问.
- **SC-037**: parallelization check(并行化检查) 必须证明影响开发并行度的任务已经拆分为至少 8 个 parallel workstream(并行工作流),并且每个 workstream(工作流) 都有独立 owner(负责人),独立主文件,独立 `_test.rs` 测试文件,明确前置依赖和可单独验收结果.
- **SC-038**: 同一 parallel workstream(并行工作流) 组中的任务不得存在同文件写入冲突.如果发现共享大文件,跨职责任务或串行等待,必须先完成 workstream split(工作流拆分),再进入后续计划和实现.
- **SC-039**: unattended implementation check(无人值守实现检查) 必须证明 implementation phase(实现阶段) 在没有人工逐项触发的情况下持续推进,并且在存在可执行 pending task(待处理任务) 时没有停止等待.
- **SC-040**: implementation completion check(实现完成检查) 必须证明 task completion ledger(任务完成台账) 中全部任务都是 completed task(已完成任务),全部 workstream(工作流) 的验收检查通过,并且没有 pending task(待处理任务),in-progress task(进行中任务),失败检查或未记录完成证据.
- **SC-041**: blocker elimination check(卡点消除检查) 必须证明每个 parallel execution blocker(并行执行卡点) 都已经分类,消除或重排,并且没有 shared file bottleneck(共享文件瓶颈),unstable contract(不稳定契约),blocking dependency(阻塞依赖),manual gate(人工门禁),long serial validation(长串行验证),unclear owner(负责人不清晰) 或 hidden coupling(隐藏耦合) 阻塞可执行工作.
- **SC-042**: 每个 blocker elimination record(卡点消除记录) 必须包含 blocker type(卡点类型),affected workstream(受影响工作流),affected file boundary(受影响文件边界),elimination action(消除动作),owner(负责人),acceptance evidence(验收证据) 和 residual risk(剩余风险).
- **SC-043**: lead agent supervision check(主代理监督检查) 必须证明每个 subagent workstream(子代理工作流) 都有 lead agent(主代理) 审查记录,并且审查覆盖规格一致性,模块边界,文件边界,测试命名,文档同步,禁止兼容方法和验收证据.
- **SC-044**: correction loop check(纠偏循环检查) 必须证明每个 development drift(开发偏差) 都有 correction record(纠偏记录),并且 workstream(工作流) 只有在纠偏复核通过后才能进入 completed task(已完成任务) 状态.
- **SC-045**: source layout check(源码布局检查) 必须证明核心模块都直接位于 `src/<module>/`,不存在 `src/supervision/` 中间层,不存在 `src/<module>.rs` 平铺模块文件,并且每个核心模块目录都包含 `mod.rs` 和 `tests/*_test.rs`.

## Assumptions(假设)

- quickstart(快速开始) 可以提供一组 YAML(数据序列化格式) 示例值,但这些值只属于示例配置,不得成为代码中的硬编码默认值.
- 网络连接类 worker(工作任务) 和核心协调类 worker(工作任务) 的 restart policy(重启策略),supervision strategy(监督策略),backoff(退避),jitter(抖动),reset_after(重置时间),heartbeat interval(心跳间隔),stale_after(过期时间),graceful shutdown timeout(优雅关闭超时),abort wait(强制终止等待),熔断阈值和熔断窗口都必须由 rust-config-tree(集中配置树) YAML(数据序列化格式) 配置提供.
- `Permanent`(永久) 表示正常退出或异常退出后都重启;`Transient`(瞬时) 表示仅在异常退出,panic(恐慌),timeout(超时) 或 unhealthy(不健康) 后重启;`Temporary`(临时) 表示永不重启.
- `OneForOne`(一对一) 只重启失败 child(子任务);`OneForAll`(一对全部) 停止并重启 supervisor(监督器) 范围内所有 child(子任务);`RestForOne`(从失败处开始) 重启失败 child(子任务) 和其后按定义顺序排列的 child(子任务).
- child-level fuse(子任务级熔断) 和 supervisor-level fuse(监督器级熔断) 都存在;quarantine(隔离) 是 child-level(子任务级) 终态治理状态,meltdown(熔断) 是升级信号.
- current state(当前状态) 和生命周期事件是两种不同产物:current state(当前状态) 回答当前状态,事件回答历史顺序.
- `When`(何时),`Where`(何处) 和 `What`(发生内容) 是标准事件词汇,不能被模糊日志术语替代.
- child(子任务) 可以通过配置选择 immediate readiness(立即就绪),但任何需要预热,建连或恢复订阅的 child(子任务) 必须通过配置选择 explicit readiness(显式就绪).
- `TaskFactory`(任务工厂) 是监督器内核入口;`Service trait`(服务特征) 和 `service_fn`(函数适配器) 只是可选适配层.
- 策略值,阈值,窗口,超时,容量,开关和预算来自 rust-config-tree(集中配置树) v0.1.9 的 `ConfigState`(配置状态).代码不得提供 schema default(模式默认值),硬编码回退值或模块局部默认值.
- rust-config-tree(集中配置树) 的主配置文件使用 YAML(数据序列化格式),示例路径为 `examples/config/supervisor.yaml`.
- 所有测试文件必须以 `_test.rs` 结尾.
- 专业词汇和反引号词汇都以 `glossary.md`(词汇表) 作为正式解释来源.
- examples(示例程序) 是学习入口,不是旧接口适配层.示例只能展示项目自有 API(接口).
- 中文和英文手册必须表达同一语义,不能让英文版或中文版落后.
- `Shutdown Without Orphaned Tasks`(关闭后不留下孤儿任务) 表达关闭完成后的系统状态,不是指关闭某一种"非孤儿任务".
- crates.io(软件包发布平台) 发布准备属于第一版完成条件,但真实上传发布不属于本功能实现任务.
- SBOM(软件物料清单) 是发布准备产物,必须随 release readiness(发布就绪) 检查生成并校验.
- 参考 crate(库) 只提供概念来源,不需要也不允许第三方 API(接口) 形状复制,旧接口别名,迁移层或任何 compatibility method(兼容方法).
- 第一版实现面向一个进程和一个 Tokio(异步运行时).distributed supervision(分布式监督),cross-process messaging(跨进程消息) 和 remote control(远程控制) 不在本功能范围内.
- 模块依赖以公开契约和单向依赖为基础,不是以源码文件中的相对路径或重导出作为依赖说明.
- 开发并行度以 ownership boundary(所有权边界) 和独立文件边界为基础,不是以多人分段修改同一个大文件作为并行方式.
- 无人值守实现以任务清单,工作流拆分,检查结果和完成台账为结束条件,不是以单个局部任务完成作为结束条件.
- 并行执行卡点必须优先通过拆分文件边界,稳定公开契约,调整任务顺序,收窄验收检查或明确负责人来消除.
- 主代理监督以同一实现周期内的审查,纠偏和复核为基础,不是等全部子代理完成后再统一返工.
