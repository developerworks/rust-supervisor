# Research(研究): 创建监督器核心

## Decision(决定): 构建项目自有 supervisor model(监督器模型)，不包装现成 crate(库)

**Rationale(理由)**: 本功能需要精确的领域模型：`ChildSpec`(子任务规格)、`SupervisorTree`(监督树)、`TaskFactory`(任务工厂)、typed exit reason(类型化退出原因)、child/supervisor fuse(子任务和监督器熔断)、control-plane audit(控制平面审计)、state snapshot(状态快照) 和 `When`(何时)、`Where`(何处)、`What`(发生内容) 事件。参考 crate(库) 分别覆盖部分能力，但没有一个能在不引入不兼容 API(接口) 或框架假设的情况下满足完整契约。

**Alternatives considered(已考虑方案)**:

- `task-supervisor`: 它的 runtime control(运行时控制)、status query(状态查询)、health interval(健康间隔)、restart limit(重启限制) 和 backoff(退避) 有参考价值。直接依赖被拒绝，因为它使用 clone-on-restart(克隆后重启) 任务模型，任务内部可变状态在重启时容易丢失语义。
- `ractor-supervisor`: 它的 `OneForOne`(一对一)、`OneForAll`(一对全部)、`RestForOne`(从失败处开始)、`Permanent`(永久)、`Transient`(瞬时)、`Temporary`(临时) 和 meltdown window(熔断窗口) 有参考价值。直接依赖被拒绝，因为项目明确排除 actor framework(参与者框架)。
- `taskvisor`: 它的 event(事件) 和 registry(注册表) 架构有参考价值。直接采用 API(接口) 被拒绝，因为本项目需要自己的 tree(树)、audit(审计)、typed error(类型化错误) 和双层 fuse(熔断) 模型。
- `tokio-graceful-shutdown`: 它的 shutdown protocol(关闭协议) 有参考价值。完整引入 API(接口) 被拒绝，因为本功能需要 supervisor tree(监督树) 专用控制和状态。
- `supertrees`: 它的 supervision tree(监督树) 词汇有参考价值。生产依赖被拒绝，因为它的文档说明当前状态偏 experimental(实验性)，并缺少 monitoring(监控)、tracing(追踪) 和 distributed messaging(分布式消息)。

## Decision(决定): 使用 `TaskFactory`(任务工厂)，不克隆任务实例

**Rationale(理由)**: 每次重启都必须构造带新 `TaskCtx`(任务上下文) 的 fresh future(新异步任务)。需要跨重启保留的状态必须通过 `Arc`(原子引用计数)、存储或调用者拥有的 state repository(状态仓库) 显式表达。这样可以让重启语义诚实，避免隐藏状态丢失。

**Alternatives considered(已考虑方案)**:

- 重启时克隆任务实例：实现更简单，但会隐藏状态重置行为。
- 把可变任务实例存进 supervisor(监督器)：这会增加锁和所有权复杂度，并且会把业务状态混入运行时治理。

## Decision(决定): 用 supervisor tree(监督树) 表达生命周期治理

**Rationale(理由)**: `OneForAll`(一对全部)、`RestForOne`(从失败处开始)、child-level quarantine(子任务级隔离)、supervisor-level meltdown(监督器级熔断)、局部关闭和父级升级都需要树边界。`/root/market/binance_ws` 这样的稳定路径，可以让日志、指标、事件和控制命令使用同一套位置词汇。

**Alternatives considered(已考虑方案)**:

- 只使用 flat registry(扁平注册表)：它更简单，但无法表达分组重启顺序或父级升级。
- 使用 actor supervision tree(参与者监督树)：它表达能力强，但违反不引入 actor framework(参与者框架) 的约束。

## Decision(决定): 直接使用 Tokio(异步运行时) 原语

**Rationale(理由)**: 本 crate(包) 面向 Tokio(异步运行时) 应用。`JoinSet`(任务集合) 符合 structured concurrency(结构化并发) 的任务所有权要求，因为 drop(丢弃) 集合会 abort(强制终止) 其中任务，`abort_all` 后也可以通过 `join_next` 排空。`CancellationToken`(取消令牌) 符合父到子关闭传播要求，因为 child token(子令牌) 取消不会取消 parent token(父令牌)。

**Alternatives considered(已考虑方案)**:

- 每个 child(子任务) 只保存一个 `JoinHandle`(等待句柄)：单任务可行，但对作用域关闭和无孤儿任务保证较弱。
- 做 executor-agnostic abstraction(执行器无关抽象)：对 Tokio(异步运行时) 专用监督器来说过早。

## Decision(决定): 分离 state snapshot(状态快照) 和 lifecycle event(生命周期事件)

**Rationale(理由)**: 当前状态和历史事件回答不同问题。watch-style state plane(观察式状态平面) 只保存最新 `SupervisorSnapshot`(监督器快照)，event plane(事件平面) 保存有序生命周期事件，供 subscriber(订阅者)、audit(审计)、replay(回放) 和测试使用。

**Alternatives considered(已考虑方案)**:

- 只提供 event stream(事件流)：消费者必须回放历史才能知道当前状态。
- 只提供 snapshot(快照)：系统会丢失顺序、命令审计、重启决策和事件滞后信息。

## Decision(决定): 使用 `tracing`(结构化追踪) 和 `metrics`(指标) 作为可观察性基础

**Rationale(理由)**: `tracing`(结构化追踪) span(追踪范围) 表达 child attempt(子任务尝试)，event(追踪事件) 表达状态迁移。`metrics`(指标) facade(门面) 允许 supervisor(监督器) 发送所需 counter(计数器)、gauge(仪表) 和 histogram(直方图)，而不把核心绑定到单一 exporter(导出器)。Prometheus exporter(普罗米修斯导出器) 可以放在示例或可选集成中，而不是核心契约。

**Alternatives considered(已考虑方案)**:

- 只使用普通日志：它不足以支持结构化生命周期回放和字段化诊断。
- 在核心中嵌入具体 metrics backend(指标后端)：这会把过多运维策略放进核心。

## Decision(决定): 使用 typed error(类型化错误) 和明确 policy decision(策略决定)

**Rationale(理由)**: 策略决定依赖失败是否为 recoverable(可恢复)、configuration-fatal(配置致命)、bug-fatal(代码致命)、external(外部)、timeout(超时)、panic(恐慌) 或 cancellation(取消)。只返回字符串或泛型错误，会迫使策略引擎猜测。

**Alternatives considered(已考虑方案)**:

- 到处使用 `anyhow::Error`(通用错误)：它方便，但对重启治理来说太不透明。
- 依赖 panic(恐慌) 表达任务失败：它不可接受，因为预期失败需要结构化结果。

## Decision(决定): 使用 deterministic test time(确定性测试时间)

**Rationale(理由)**: backoff(退避)、stale heartbeat(过期心跳)、graceful timeout(优雅关闭超时)、abort wait(强制终止等待) 和 meltdown window(熔断窗口) 都由时间驱动。Tokio(异步运行时) paused time(暂停时间) 能让测试在毫秒级推进 60 秒窗口，而不进行真实等待。

**Alternatives considered(已考虑方案)**:

- 在测试中真实 sleep(睡眠)：它很慢，也容易不稳定。
- 为所有内容自定义 clock(时钟)：代码更多。第一版先使用 Tokio time(Tokio 时间)，再把帮助函数隔离在 `test_support.rs`。

## Decision(决定): 规划阶段依赖集合

**Rationale(理由)**: 规划前已经检查当前 crate(包) 元数据。规划依赖保持窄边界，并直接对应需求：

- `tokio` 1.52.1: runtime(运行时)、`JoinSet`(任务集合)、sync channel(同步通道)、time(时间) 和 test time(测试时间)。
- `tokio-util` 0.7.18: `CancellationToken`(取消令牌)。
- `tracing` 0.1.44 和 `tracing-subscriber` 0.3.23: span(追踪范围)、event(追踪事件) 和测试或示例 subscriber(订阅者)。
- `metrics` 0.24.5: required metric name(必需指标名) 的 metrics facade(指标门面)。
- `thiserror` 2.0.18: typed supervisor error(类型化监督器错误)。
- `serde` 1.0.228 和 `serde_json` 1.0.149: snapshot(快照)、event(事件) 和 audit(审计) 序列化。
- `uuid` 1.23.1: command id(命令标识) 和 correlation id(关联标识) 生成。
- `rand` 0.10.1: 生产 jitter(抖动) 来源，并在测试中提供确定性覆盖。
- 现有 `rust-config-tree` 0.1.7: 后续声明式配置 include tree(包含树) 可使用它；监督器运行时核心本身不依赖它。

**Alternatives considered(已考虑方案)**:

- 增加 actor(参与者) 或 supervision(监督) crate(库)：被功能范围拒绝。
- 把具体 Prometheus exporter(普罗米修斯导出器) 加进核心：推迟到示例，保持库表面小。
- 增加 `anyhow`(通用错误)：策略决定需要类型化类别，所以拒绝。
