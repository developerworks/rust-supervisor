# Research(研究): 创建监督器核心

## Decision(决定): 构建项目自有 supervisor model(监督器模型),不包装现成 crate(库)

**Rationale(理由)**: 本功能需要精确的领域模型:`ChildSpec`(子任务规格),`SupervisorTree`(监督树),`TaskFactory`(任务工厂),typed exit reason(类型化退出原因),child/supervisor fuse(子任务和监督器熔断),control-plane audit(控制平面审计),current state(当前状态) 和 `When`(何时),`Where`(何处),`What`(发生内容) 事件.参考 crate(库) 分别覆盖部分能力,但没有一个能在不复制第三方 API(接口) 形状或引入框架假设的情况下满足完整契约.

**Alternatives considered(已考虑方案)**:

- `task-supervisor`: 它的 runtime control(运行时控制),status query(状态查询),health interval(健康间隔),restart limit(重启限制) 和 backoff(退避) 有参考价值.直接依赖被拒绝,因为它使用 clone-on-restart(克隆后重启) 任务模型,任务内部可变状态在重启时容易丢失语义.
- `ractor-supervisor`: 它的 `OneForOne`(一对一),`OneForAll`(一对全部),`RestForOne`(从失败处开始),`Permanent`(永久),`Transient`(瞬时),`Temporary`(临时) 和 meltdown window(熔断窗口) 有参考价值.直接依赖被拒绝,因为项目明确排除 actor framework(参与者框架).
- `taskvisor`: 它的 event(事件) 和 registry(注册表) 架构有参考价值.直接采用 API(接口) 被拒绝,因为本项目需要自己的 tree(树),audit(审计),typed error(类型化错误) 和双层 fuse(熔断) 模型.
- `tokio-graceful-shutdown`: 它的 shutdown protocol(关闭协议) 有参考价值.完整引入 API(接口) 被拒绝,因为本功能需要 supervisor tree(监督树) 专用控制和状态.
- `supertrees`: 它的 supervision tree(监督树) 词汇有参考价值.生产依赖被拒绝,因为它的文档说明当前状态偏 experimental(实验性),并缺少 monitoring(监控),tracing(追踪) 和 distributed messaging(分布式消息).

## Decision(决定): 使用 `TaskFactory`(任务工厂),不克隆任务实例

**Rationale(理由)**: 每次重启都必须构造带新 `TaskContext`(任务上下文) 的 fresh future(新异步任务).需要跨重启保留的状态必须通过 `Arc`(原子引用计数),存储或调用者拥有的 state repository(状态仓库) 显式表达.这样可以让重启语义诚实,避免隐藏状态丢失.

**Alternatives considered(已考虑方案)**:

- 重启时克隆任务实例:实现更简单,但会隐藏状态重置行为.
- 把可变任务实例存进 supervisor(监督器):这会增加锁和所有权复杂度,并且会把业务状态混入运行时治理.

## Decision(决定): 用 supervisor tree(监督树) 表达生命周期治理

**Rationale(理由)**: `OneForAll`(一对全部),`RestForOne`(从失败处开始),child-level quarantine(子任务级隔离),supervisor-level meltdown(监督器级熔断),局部关闭和父级升级都需要树边界.`/root/market/binance_ws` 这样的稳定路径,可以让日志,指标,事件和控制命令使用同一套位置词汇.

**Alternatives considered(已考虑方案)**:

- 只使用 flat registry(扁平注册表):它更简单,但无法表达分组重启顺序或父级升级.
- 使用 actor supervision tree(参与者监督树):它表达能力强,但违反不引入 actor framework(参与者框架) 的约束.

## Decision(决定): 直接使用 Tokio(异步运行时) 原语

**Rationale(理由)**: 本 crate(包) 面向 Tokio(异步运行时) 应用.`JoinSet`(任务集合) 符合 structured concurrency(结构化并发) 的任务所有权要求,因为 drop(丢弃) 集合会 abort(强制终止) 其中任务,`abort_all` 后也可以通过 `join_next` 排空.`CancellationToken`(取消令牌) 符合父到子关闭传播要求,因为 child token(子令牌) 取消不会取消 parent token(父令牌).

**Alternatives considered(已考虑方案)**:

- 每个 child(子任务) 只保存一个 `JoinHandle`(等待句柄):单任务可行,但对作用域关闭和无孤儿任务保证较弱.
- 做 executor-agnostic abstraction(执行器无关抽象):对 Tokio(异步运行时) 专用监督器来说过早.

## Decision(决定): 分离 current state(当前状态) 和 lifecycle event(生命周期事件)

**Rationale(理由)**: 当前状态和历史事件回答不同问题.watch-style state plane(观察式状态平面) 只保存最新 `SupervisorState`(监督器当前状态),event plane(事件平面) 保存有序生命周期事件,供 subscriber(订阅者),audit(审计),replay(回放) 和测试使用.

**Alternatives considered(已考虑方案)**:

- 只提供 event stream(事件流):消费者必须回放历史才能知道当前状态.
- 只提供 current state(当前状态):系统会丢失顺序,命令审计,重启决策和事件滞后信息.

## Decision(决定): 禁止 `*Snapshot` 和 `*View` 代码命名

**Rationale(理由)**: 用户要求删除所有 `*Snapshot` 和 `*View` 命名方式.状态查询在本项目中表达的是当前状态,不是复制某个历史对象,也不是只读视图对象.因此配置加载结果命名为 `ConfigState`(配置状态),监督器状态命名为 `SupervisorState`(监督器状态),子任务状态命名为 `ChildState`(子任务状态),运行时查询命令命名为 `current_state`(当前状态),源码模块命名为 `state`(状态).

**Alternatives considered(已考虑方案)**:

- 继续使用 `ConfigSnapshot` 和 `SupervisorSnapshot`:被拒绝,因为它违反新的代码命名要求.
- 继续使用 `SupervisorStateView`,`ChildStateView` 或 `state_view`:被拒绝,因为它违反新的代码命名要求.
- 只在文档中改名而保留代码别名:被拒绝,因为项目禁止兼容方法和旧接口别名.

## Decision(决定): 使用 `tracing`(结构化追踪) 和 `metrics`(指标) 作为可观察性基础

**Rationale(理由)**: `tracing`(结构化追踪) span(追踪范围) 表达 child attempt(子任务尝试),event(追踪事件) 表达状态迁移.`metrics`(指标) facade(门面) 允许 supervisor(监督器) 发送所需 counter(计数器),gauge(仪表) 和 histogram(直方图),而不把核心绑定到单一 exporter(导出器).Prometheus exporter(普罗米修斯导出器) 可以放在示例或可选集成中,而不是核心契约.

**Alternatives considered(已考虑方案)**:

- 只使用普通日志:它不足以支持结构化生命周期回放和字段化诊断.
- 在核心中嵌入具体 metrics backend(指标后端):这会把过多运维策略放进核心.

## Decision(决定): 使用 typed error(类型化错误) 和明确 policy decision(策略决定)

**Rationale(理由)**: 策略决定依赖失败是否为 recoverable(可恢复),configuration-fatal(配置致命),bug-fatal(代码致命),external(外部),timeout(超时),panic(恐慌) 或 cancellation(取消).只返回字符串或泛型错误,会迫使策略引擎猜测.

**Alternatives considered(已考虑方案)**:

- 到处使用 `anyhow::Error`(通用错误):它方便,但对重启治理来说太不透明.
- 依赖 panic(恐慌) 表达任务失败:它不可接受,因为预期失败需要结构化结果.

## Decision(决定): 使用 deterministic test time(确定性测试时间)

**Rationale(理由)**: backoff(退避),stale heartbeat(过期心跳),graceful timeout(优雅关闭超时),abort wait(强制终止等待) 和 meltdown window(熔断窗口) 都由时间驱动.Tokio(异步运行时) paused time(暂停时间) 能让测试在毫秒级推进 60 秒窗口,而不进行真实等待.

**Alternatives considered(已考虑方案)**:

- 在测试中真实 sleep(睡眠):它很慢,也容易不稳定.
- 为所有内容自定义 clock(时钟):代码更多.第一版先使用 Tokio time(Tokio 时间),再把帮助函数隔离在 `test_support.rs`.

## Decision(决定): 把 readiness(就绪) 建成一等生命周期信号

**Rationale(理由)**: 许多受监督任务在进入 running(运行中) 后仍需要缓存预热,连接建立或订阅恢复.系统如果把 running(运行中) 直接当作 ready(已就绪),操作者会误判服务状态.`ReadinessPolicy`(就绪策略) 必须支持 immediate readiness(立即就绪) 和 explicit readiness(显式就绪),并且 explicit readiness(显式就绪) 必须通过 `TaskContext`(任务上下文) 明确报告.

**Alternatives considered(已考虑方案)**:

- 只使用 running(运行中) 状态:实现更简单,但不能表达预热和恢复订阅的真实边界.
- 只使用 heartbeat(心跳) 判断就绪:heartbeat(心跳) 表示任务还活着,不等于业务已经可用.

## Decision(决定): 单独建模 blocking task(阻塞任务)

**Rationale(理由)**: `spawn_blocking`(阻塞任务启动) 和其它 blocking worker(阻塞工作任务) 不能假设 `abort`(强制终止) 一定立即有效.它们必须通过 `TaskKind`(任务类型), shutdown policy(关闭策略) 和 escalation policy(升级策略) 独立表达, 并在关闭超时时记录不可立即终止边界.

**Alternatives considered(已考虑方案)**:

- 把 blocking task(阻塞任务) 当作普通 async task(异步任务):这会隐藏无法立即终止的真实风险.
- 禁止 blocking task(阻塞任务):这会让监督器无法治理现实业务中的阻塞边界.

## Decision(决定): 使用 four-stage shutdown(四阶段关闭)

**Rationale(理由)**: cancel-then-abort(先取消后强制终止) 是对外边界, 但内部必须明确 request stop(请求停止), graceful drain(优雅排空), abort stragglers(强制终止拖尾任务) 和 reconcile(状态对账). reconcile(状态对账) 负责统一 registry(注册表), current state(当前状态), metrics(指标) 和 event journal(事件日志缓冲区), 防止关闭完成后状态互相矛盾.

**Alternatives considered(已考虑方案)**:

- 只保留 two-phase shutdown(两阶段关闭):它表达了取消和强制终止, 但没有明确关闭后的状态对账.
- 把状态对账交给调用者:这会让不同调用者得到不同关闭语义.

## Decision(决定): 使用 fixed-capacity event journal(固定容量事件日志缓冲区) 和 RunSummary(运行摘要)

**Rationale(理由)**: event stream(事件流) 面向订阅者, 但事故发生时还需要最近生命周期事件的本地诊断窗口. fixed-capacity event journal(固定容量事件日志缓冲区) 保留最近关键事件, `RunSummary`(运行摘要) 在 meltdown(熔断), 关闭超时或父级升级时汇总失败原因, 重启次数, 关闭原因和最终状态.

**Alternatives considered(已考虑方案)**:

- 只依赖外部日志系统:测试和本地嵌入式使用无法稳定获得诊断上下文.
- 保存无限事件历史:这会让核心承担持久化和容量治理问题.

## Decision(决定): 限制 metrics label(指标标签) 为低基数值

**Rationale(理由)**: supervisor(监督器) 指标会被长期采集. 如果 label(标签) 包含错误全文, 用户输入或动态路径碎片, 指标后端会遭遇高基数压力, 事故排查也会变得更难. 因此核心只允许 supervisor path(监督器路径), child id(子任务标识), state(状态), decision(决定) 和 failure category(失败类别) 等低基数值.

**Alternatives considered(已考虑方案)**:

- 把完整错误文本放进 label(标签):它便于临时调试, 但不适合作为长期指标.
- 不做 label(标签) 校验:实现更少, 但会把运维风险留给调用者.

## Decision(决定): 规划阶段依赖集合

**Rationale(理由)**: 规划前已经检查当前 crate(包) 元数据.规划依赖保持窄边界,并直接对应需求:

- `tokio` 1.52.1: runtime(运行时),`JoinSet`(任务集合),sync channel(同步通道),time(时间) 和 test time(测试时间).
- `tokio-util` 0.7.18: `CancellationToken`(取消令牌).
- `tracing` 0.1.44 和 `tracing-subscriber` 0.3.23: span(追踪范围),event(追踪事件) 和测试或示例 subscriber(订阅者).
- `metrics` 0.24.5: required metric name(必需指标名) 的 metrics facade(指标门面).
- `thiserror` 2.0.18: typed supervisor error(类型化监督器错误).
- `serde` 1.0.228 和 `serde_json` 1.0.149: current state(当前状态),event(事件) 和 audit(审计) 序列化.
- `serde_yaml` 0.9: YAML(数据序列化格式) 配置解析支持,用于 rust-config-tree(集中配置树) v0.1.9 的主配置格式.
- `uuid` 1.23.1: command id(命令标识) 和 correlation id(关联标识) 生成.
- `rand` 0.10.1: 生产 jitter(抖动) 来源,并在测试中提供确定性覆盖.
- 现有 `rust-config-tree` 0.1.9: 必须作为 centralized configuration(集中化配置) 唯一入口,加载 YAML(数据序列化格式) 主配置,并生成 `ConfigState`(配置状态).配置不得分散在模块内部.

**Alternatives considered(已考虑方案)**:

- 增加 actor(参与者) 或 supervision(监督) crate(库):被功能范围拒绝.
- 把具体 Prometheus exporter(普罗米修斯导出器) 加进核心:推迟到示例,保持库表面小.
- 增加 `anyhow`(通用错误):策略决定需要类型化类别,所以拒绝.

## Decision(决定): rust-config-tree(集中配置树) 固定使用 v0.1.9 和 YAML(数据序列化格式)

**Rationale(理由)**: 用户要求项目使用 rust-config-tree(集中配置树) 做 centralized configuration(集中化配置),并要求配置不能分散在各处.当前规格把 `rust-config-tree`(集中配置树软件包) 版本固定为 v0.1.9,主配置格式固定为 YAML(数据序列化格式),示例路径固定为 `examples/config/supervisor.yaml`.这样可以让配置加载,示例,quickstart(快速开始),测试和文档使用同一个来源.

**Alternatives considered(已考虑方案)**:

- 继续支持 TOML(配置格式) 或 JSON(数据交换格式) 主配置:被拒绝,因为它会扩大配置入口并制造多格式维护成本.
- 允许模块自己保存可调默认值:被拒绝,因为它会绕过 rust-config-tree(集中配置树) 的集中化配置边界.

## Decision(决定): 用 glossary.md(词汇表) 治理专业词汇和反引号词汇

**Rationale(理由)**: 规格文档包含大量 supervisor(监督器),policy(策略),runtime(运行时),observability(可观测性),configuration(配置),release(发布) 和 Rust(编程语言) 类型词汇.用户要求专业词汇单独成文,并且反引号内的词汇也算词汇.因此 `glossary.md`(词汇表) 成为正式术语来源,并且必须覆盖类型名,枚举值,方法名,字段名,指标名,路径名,命令名,配置键和测试目标.

**Alternatives considered(已考虑方案)**:

- 把词汇说明散落在各文档章节:被拒绝,因为它会导致同一英文术语出现不同中文说明.
- 只登记自然语言专业词汇,不登记反引号词汇:被拒绝,因为反引号内的公开名称同样影响 API(接口),配置和测试理解.

## Decision(决定): 禁止 compatibility method(兼容方法)

**Rationale(理由)**: 本项目是全新开发项目,没有历史包袱.公开 API(接口) 应该直接表达第一版自有模型,不能为了旧接口,第三方 API(接口) 或迁移路径增加额外表面.

**Alternatives considered(已考虑方案)**:

- 提供旧接口别名或 deprecated facade(废弃门面):被拒绝,因为它会制造不存在的历史包袱.
- 提供第三方 API(接口) 包装层:被拒绝,因为它会让使用者误以为本项目承诺第三方语义.

## Decision(决定): 使用 module dependency map(模块依赖图) 和 parallel workstream(并行工作流) 提高开发并行度

**Rationale(理由)**: 实现阶段必须并行开发,但并行开发只有在模块依赖,文件所有权和测试边界清晰时才不会互相阻塞.计划把实现拆成 contract foundation(契约基础),configuration(集中配置),declaration and task(声明和任务),policy and time(策略和时间),runtime tree(运行时树),control and shutdown(控制和关闭),observability diagnostics(可观测性和诊断),docs examples release(文档示例和发布) 和 quality governance(质量治理) 九个 workstream(工作流).每个 workstream(工作流) 拥有明确 primary files(主文件),independent tests(独立测试) 和 blocker removal action(卡点消除动作).

**Alternatives considered(已考虑方案)**:

- 按 user story(用户故事) 串行实现:被拒绝,因为 runtime(运行时),configuration(配置),observability(可观测性),docs(文档) 和 quality gate(质量门禁) 可以在契约稳定后并行推进.
- 只用任务列表标记 `[P]`:被拒绝,因为它不能解释模块依赖关系,也不能约束共享文件冲突.
- 允许子代理自由选择文件范围:被拒绝,因为它会增加 merge conflict(合并冲突) 和 contract drift(契约漂移) 风险.

## Decision(决定): 使用 unattended implementation(无人值守实现),task completion ledger(任务完成台账) 和 lead agent supervision(主代理监督) 治理实现执行

**Rationale(理由)**: 用户要求实现阶段必须并行开发,无人值守,直到所有任务完成,并且主代理必须监督子代理开发工作并及时纠偏.因此实现阶段必须维护 task completion ledger(任务完成台账),subagent workstream record(子代理工作流记录),lead agent review(主代理审查),correction record(纠偏记录) 和 final evidence(最终证据).每个 workstream(工作流) 只有在测试,文档同步,模块边界,命名约束和完成证据都通过后才能完成.

**Alternatives considered(已考虑方案)**:

- 每个子代理完成后直接合并:被拒绝,因为它缺少主代理监督和偏差闭环.
- 只在最后运行一次总验收:被拒绝,因为并行开发中的偏差会积累到后期,增加返工成本.
- 依赖人工逐项催办:被拒绝,因为它违反 unattended implementation(无人值守实现) 要求.
