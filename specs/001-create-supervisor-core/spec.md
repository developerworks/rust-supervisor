# Feature Specification(功能规格): 创建监督器核心

**Feature Branch(功能分支)**: `001-create-supervisor-core`
**Created(创建日期)**: 2026-05-04
**Status(状态)**: Draft(草稿)
**Input(输入)**: 用户描述：“吸收 `task-supervisor`、`taskvisor`、`tokio-graceful-shutdown`、`ractor-supervisor`、`task_scope`、Tokio(异步运行时) `JoinSet`、`supertrees`、Tokio(异步运行时) `watch`、`tokio-util` `CancellationToken` 和 `tracing`(结构化追踪) 的成熟概念，创建一个基于 Tokio(异步运行时) 的轻量 supervisor(监督器) 运行时治理层。它负责启动、停止、重启、隔离、降级、熔断、状态查询、事件记录、健康检查和关闭顺序；不引入 actor(参与者) 框架，不照搬第三方 crate(库) API(接口)。”

## Clarifications(澄清)

### Session(会话) 2026-05-04

- Q: 这个 supervisor(监督器) 除了自动重启，还必须满足哪些要求？ → A: 它必须提供可解释的生命周期治理，并且必须包含声明式子任务、监督树、策略引擎、控制平面、状态平面、事件模型、指标、审计、关闭协议、结构化并发和可复现测试。
- Q: 是否立即采纳 `research-adoption-notes.md` 中“应直接采纳”的 8 条？ → A: 立即采纳 control plane(控制面) 和 data plane(数据面) 分离、逆序关闭、readiness(就绪)、`spawn_blocking`(阻塞任务启动) 隔离、reconcile(状态对账)、event journal(事件日志缓冲区)、`RunSummary`(运行摘要)、指标标签低基数和 `Service trait`(服务特征) 适配层。

## User Scenarios & Testing(用户场景和测试) *(mandatory(必填))*

### User Story 1(用户故事一) - 声明并运行子任务 (Priority(优先级): P1)

维护者需要用声明式 `ChildSpec`(子任务规格) 定义每个 child(子任务)：`id`、`name`、`kind`、`restart_policy`、`shutdown_policy`、`health_policy`、`readiness_policy`、`backoff_policy`、`dependencies`、`tags` 和 `criticality`。业务代码不应该分散调用 `tokio::spawn`，而应该把任务生命周期交给 supervisor(监督器)。

**Why this priority(为什么是这个优先级)**: 声明式子任务是 supervisor(监督器) 的入口。没有稳定规格，系统就无法统一治理启动、关闭、重启、健康检查、状态查询和审计。

**Independent Test(独立测试)**: 测试定义一个 child(子任务) 规格并启动 supervisor(监督器)，然后验证 child(子任务) 进入运行状态，按 readiness(就绪) 契约产生 `ChildStarting`、`ChildRunning` 和 `ChildReady` 事件，并且可以通过快照查询到稳定路径和当前状态。

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** 一个包含完整 `ChildSpec`(子任务规格) 的 worker(工作任务)，**When(当)** supervisor(监督器) 启动它，**Then(则)** child(子任务) 按规格启动，并记录带稳定路径的生命周期事件。
2. **Given(假设)** 业务代码试图绕过 supervisor(监督器) 直接分散启动后台任务，**When(当)** 维护者审查任务定义，**Then(则)** 该行为不满足本功能规格，因为后台任务必须通过 `ChildSpec`(子任务规格) 接入治理。
3. **Given(假设)** 一个 child(子任务) 需要缓存预热、连接建立或订阅恢复，**When(当)** 它还没有显式报告 readiness(就绪)，**Then(则)** supervisor(监督器) 不得把它标记为 ready(已就绪)。

---

### User Story 2(用户故事二) - 构建监督树 (Priority(优先级): P2)

维护者需要构建 `SupervisorTree`(监督树)。root supervisor(根监督器) 可以包含子 supervisor(监督器) 和 worker(工作任务)。worker(工作任务) 负责真实业务任务，supervisor(监督器) 负责任务治理。树中每个节点必须有稳定路径，例如 `/root/market/binance_ws`，用于日志、指标、事件和控制命令定位。

**Why this priority(为什么是这个优先级)**: 单层任务列表无法表达隔离、局部关闭、分组重启和向父级升级。`SupervisorTree`(监督树) 让治理边界和故障传播路径可以解释。

**Independent Test(独立测试)**: 测试创建一个 root supervisor(根监督器)、一个子 supervisor(监督器) 和两个 worker(工作任务)，然后验证快照包含树结构、父子关系、稳定路径和定义顺序。

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** 一个包含子 supervisor(监督器) 和 worker(工作任务) 的树，**When(当)** root(根节点) 启动，**Then(则)** 所有节点按声明顺序启动，并在快照中显示父子关系。
2. **Given(假设)** 一个 child(子任务) 失败并触发上报，**When(当)** 父 supervisor(监督器) 收到事件，**Then(则)** 事件的 `Where`(何处) 信息明确包含 supervisor path(监督器路径)、child id(子任务标识) 和 parent id(父标识)。
3. **Given(假设)** 一个 supervisor(监督器) 正在关闭包含多个 child(子任务) 的树，**When(当)** 关闭流程开始，**Then(则)** 系统必须按声明顺序的逆序关闭 child(子任务)。

---

### User Story 3(用户故事三) - 应用重启、退避和熔断策略 (Priority(优先级): P3)

维护者需要用核心枚举表达 `SupervisionStrategy`(监督策略)、`RestartPolicy`(重启策略)、`BackoffPolicy`(退避策略) 和 `MeltdownPolicy`(熔断策略)，让策略引擎根据 `ExitReason`(退出原因) 和错误类别做决定。

**Why this priority(为什么是这个优先级)**: supervisor(监督器) 不能靠硬编码业务逻辑重启任务。策略必须可声明、可测试、可审计，并且必须和退出原因解耦。

**Independent Test(独立测试)**: 测试让 child(子任务) 在不同策略下正常退出、失败、panic(恐慌)、timeout(超时) 和 unhealthy(不健康)，然后验证策略引擎分别返回不重启、延迟重启、隔离、向父级升级或关闭整棵树。

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** `OneForOne`(一对一) 策略，**When(当)** 一个 child(子任务) 失败，**Then(则)** 系统只重启失败的 child(子任务)。
2. **Given(假设)** `OneForAll`(一对全部) 策略，**When(当)** 任意 child(子任务) 失败，**Then(则)** 同组所有 child(子任务) 先停止，再按定义顺序重启。
3. **Given(假设)** `RestForOne`(从失败处开始) 策略，**When(当)** 一个 child(子任务) 失败，**Then(则)** 失败 child(子任务) 之前的 child(子任务) 不重启，失败 child(子任务) 以及其后按定义顺序启动的 child(子任务) 一起重启。

---

### User Story 4(用户故事四) - 治理健康状态和运行时控制 (Priority(优先级): P4)

操作者需要通过 `SupervisorHandle`(监督器句柄) 在运行时执行 `add_child`、`remove_child`、`restart_child`、`pause_child`、`resume_child`、`quarantine_child`、`shutdown_tree`、`snapshot` 和 `subscribe_events`。命令必须幂等，并且每个控制命令必须产生审计事件。

**Why this priority(为什么是这个优先级)**: 工业级 supervisor(监督器) 必须能在运行时治理任务，而不是只能在启动时被动等待失败。

**Independent Test(独立测试)**: 测试对同一个 child(子任务) 重复执行 shutdown(关闭)、pause(暂停)、resume(恢复) 和 quarantine(隔离)，然后验证命令返回当前状态或成功结果，不产生不可恢复错误，并且每次命令都有审计记录。

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** 一个已停止 child(子任务)，**When(当)** 操作者重复请求 shutdown(关闭)，**Then(则)** supervisor(监督器) 返回当前停止状态，并产生 command event(命令事件)。
2. **Given(假设)** 一个运行中 child(子任务) 停止发送 heartbeat(心跳)，**When(当)** 超过 `stale_after`，**Then(则)** supervisor(监督器) 把它判定为 unhealthy(不健康)，并按策略处理。

---

### User Story 5(用户故事五) - 关闭时不留下孤儿任务 (Priority(优先级): P5)

操作者需要 root shutdown(根关闭) 触发 shutdown protocol(关闭协议)。该协议对外保持 cancel-then-abort(先取消后强制终止) 边界，对内包含 request stop(请求停止)、graceful drain(优雅排空)、abort stragglers(强制终止拖尾任务) 和 reconcile(状态对账) 四个阶段。父 token(令牌) 取消必须传播到 child token(子令牌)，child token(子令牌) 取消不能反向取消父 token(令牌)。关闭完成后不能留下 orphan task(孤儿任务)。

**Why this priority(为什么是这个优先级)**: supervisor(监督器) 必须可靠收尾。关闭不清晰会导致资源泄漏、任务悬挂和不可解释的退出结果。

**Independent Test(独立测试)**: 测试启动多个长运行 child(子任务) 后请求 root shutdown(根关闭)，然后验证所有 child token(子令牌) 被取消、所有任务集合为空、超时 child(子任务) 被第二阶段终止，并产生关闭请求和关闭完成事件。

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** 多个正在运行的 child(子任务)，**When(当)** 操作者请求 root shutdown(根关闭)，**Then(则)** 每个 child(子任务) 的取消令牌被触发，并在完成后报告关闭完成。
2. **Given(假设)** 一个 child(子任务) 超过 graceful timeout(优雅关闭超时)，**When(当)** 第一阶段等待结束，**Then(则)** supervisor(监督器) 进入第二阶段强制终止该 child(子任务)，并报告超时原因。
3. **Given(假设)** root shutdown(根关闭) 已完成，**When(当)** 操作者检查运行时任务集合，**Then(则)** 不存在 orphan task(孤儿任务)，所有 child(子任务) 都处于 terminal(终态) 或 quarantined(隔离) 状态，并且 registry(注册表)、snapshot(快照)、metrics(指标) 和 event journal(事件日志缓冲区) 已完成 reconcile(状态对账)。
4. **Given(假设)** 一个 child(子任务) 使用 `spawn_blocking`(阻塞任务启动)，**When(当)** root shutdown(根关闭) 超时，**Then(则)** supervisor(监督器) 不得假设 `abort`(强制终止) 一定有效，必须按独立 `TaskKind`(任务类型)、关闭策略和升级策略处理。

---

### User Story 6(用户故事六) - 观察、审计并回放生命周期 (Priority(优先级): P6)

维护者需要同时获得最新状态快照和完整生命周期事件流。状态平面保存最新 `SupervisorSnapshot`(监督器快照)，事件平面发布完整 `SupervisorEvent`(监督器事件)，并通过 `tracing`(结构化追踪)、metrics(指标)、audit log(审计日志) 和 subscriber(订阅者) 输出。

**Why this priority(为什么是这个优先级)**: 这个 supervisor(监督器) 的核心不是自动重启，而是可解释的生命周期治理。事故排查必须知道 `When`(何时)、`Where`(何处)、`What`(发生内容)。

**Independent Test(独立测试)**: 测试让 child(子任务) 经历启动、心跳、失败、退避、重启、隔离和关闭，验证每次状态迁移都有事件，快照能查询最新状态，指标和审计日志反映同一事实。

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** 任意一次状态迁移，**When(当)** 事件被发布，**Then(则)** 事件包含 `When`(何时)、`Where`(何处)、`What`(发生内容)、sequence(序号)、correlation id(关联标识) 和策略决定。
2. **Given(假设)** 一个 child attempt(子任务尝试) 开始，**When(当)** 任务运行，**Then(则)** 该 attempt(尝试) 有自己的 tracing span(追踪范围)，状态迁移有 tracing event(追踪事件)。
3. **Given(假设)** supervisor(监督器) 发生 meltdown(熔断) 或关闭超时，**When(当)** 操作者读取诊断输出，**Then(则)** 系统提供 `RunSummary`(运行摘要)，并包含最近 event journal(事件日志缓冲区) 中的关键生命周期事件。

### Edge Cases(边界情况)

- child(子任务) 在启动阶段立即失败或 panic(恐慌) 时，supervisor(监督器) 仍然必须记录失败阶段、attempt(尝试次数)、generation(代次) 和重启决策。
- child(子任务) 正常完成且策略为 `Transient`(瞬时) 或 `Temporary`(临时) 时，supervisor(监督器) 不得错误重启。
- child(子任务) 请求关闭时，supervisor(监督器) 必须把它识别为 `Cancelled`(已取消) 或请求关闭结果，而不是普通失败。
- 同一个 child(子任务) 在 60 秒内重启超过 10 次时，该 child(子任务) 必须进入 `Quarantined`(已隔离)，并且不得继续重启。
- 同一个 supervisor(监督器) 在 60 秒内发生超过 30 次 child(子任务) 失败时，supervisor(监督器) 必须向父 supervisor(监督器) 上报 `Meltdown`(熔断)。
- `OneForAll`(一对全部) 重启组内任务时，系统必须先停止整组，再按定义顺序重启，不能交叉启动和停止。
- `RestForOne`(从失败处开始) 重启时，失败 child(子任务) 之前的 child(子任务) 不得受影响。
- 生命周期事件消费者落后或溢出时，状态快照仍然必须准确，并记录 `supervisor_event_lag_total`。
- 测试退避、超时、心跳和熔断窗口时，测试不得依赖真实 sleep(睡眠)，必须能用 paused time(暂停时间) 推进。
- 高频业务消息不得每条都经过 supervisor(监督器)。supervisor(监督器) 只管理生命周期、健康、控制命令和低频事件。
- control plane(控制面) 不得承载业务 data plane(数据面) 消息处理；业务任务只能通过生命周期、状态、事件和控制命令与 supervisor(监督器) 交互。
- blocking task(阻塞任务) 关闭超时时，supervisor(监督器) 必须记录不可立即终止的边界，并按策略升级，而不能把它当作普通 async task(异步任务)。
- 指标标签不得包含错误全文、动态路径碎片、用户输入或其它无界值。

## Requirements(需求) *(mandatory(必填))*

### Functional Requirements(功能需求)

- **FR-001**: 系统必须为每个 child(子任务) 提供声明式 `ChildSpec`(子任务规格)，并包含 `id`、`name`、`kind`、`restart_policy`、`shutdown_policy`、`health_policy`、`readiness_policy`、`backoff_policy`、`dependencies`、`tags` 和 `criticality`。
- **FR-002**: 系统必须防止被监督后台工作以分散且无人管理的 spawn(启动任务) 表达；受生命周期治理的工作必须通过 child(子任务) 规格进入系统。
- **FR-003**: 系统必须支持 `TaskFactory`(任务工厂) 模型，使每次重启都构造新的任务尝试；必须跨重启保留的状态需要显式放入共享状态、持久化存储或状态仓库。
- **FR-004**: 系统必须支持 `TaskCtx`(任务上下文)，其中包含 child(子任务) 身份、supervisor path(监督器路径)、generation(代次)、attempt(尝试次数)、cancellation token(取消令牌)、event sink(事件接收点) 和 heartbeat(心跳) 接口。
- **FR-005**: 系统必须支持 `SupervisorTree`(监督树)，其中 root supervisor(根监督器) 可以包含子 supervisor(监督器) 和 worker(工作任务)。
- **FR-006**: 系统必须为每个受监督节点分配稳定路径，例如 `/root/market/binance_ws`。
- **FR-007**: 系统必须把 `OneForOne`(一对一)、`OneForAll`(一对全部) 和 `RestForOne`(从失败处开始) 作为核心 supervision strategy(监督策略)。
- **FR-008**: 系统必须把 `Permanent`(永久)、`Transient`(瞬时) 和 `Temporary`(临时) 作为核心 restart policy(重启策略)。
- **FR-009**: 系统必须把 restart policy(重启策略) 和 `ExitReason`(退出原因) 分开建模。
- **FR-010**: 系统必须把任务退出分类为 completed(已完成)、failed(已失败)、cancelled(已取消)、timed out(已超时)、unhealthy(不健康) 或 panicked(已恐慌)。
- **FR-011**: 系统必须把任务失败分类为 `Recoverable`(可恢复)、`FatalConfig`(致命配置错误)、`FatalBug`(致命代码错误)、`ExternalDependency`(外部依赖错误)、`Timeout`(超时)、`Panic`(恐慌) 和 `Cancelled`(已取消)。
- **FR-012**: 系统必须使用失败类别、退出原因、criticality(关键程度)、restart policy(重启策略)、meltdown policy(熔断策略) 和当前状态来做重启决策。
- **FR-013**: 系统必须支持 child-level fuse(子任务级熔断)。默认情况下，同一个 child(子任务) 在 60 秒内重启超过 10 次后必须进入 quarantine(隔离)。
- **FR-014**: 系统必须支持 supervisor-level fuse(监督器级熔断)。默认情况下，同一个 supervisor(监督器) 在 60 秒内发生超过 30 次 child(子任务) 失败后必须报告 meltdown(熔断)。
- **FR-015**: 系统必须支持 reset-after(稳定后重置) 语义，使稳定运行可以重置重启计数和熔断计数。
- **FR-016**: 系统默认必须支持 exponential backoff with jitter(带随机抖动的指数退避)：初始 100ms(毫秒)、最大 5s(秒)、jitter(抖动) 10%、reset_after(重置时间) 60s(秒)。
- **FR-017**: 系统必须允许测试关闭 jitter(抖动)，使退避断言保持确定。
- **FR-018**: 系统必须支持基于 heartbeat(心跳) 的健康检查，而不能只检查任务是否仍在运行。
- **FR-019**: 系统必须在 `stale_after` 内没有收到 heartbeat(心跳) 时把 child(子任务) 标记为 unhealthy(不健康)；默认 heartbeat interval(心跳间隔) 为 1 秒，默认 stale threshold(过期阈值) 为 3 秒。
- **FR-020**: 系统必须支持 cancel-then-abort shutdown boundary(先取消后强制终止的关闭边界)：先取消并等待 graceful timeout(优雅关闭超时)，超时后才 abort(强制终止)，并且内部流程必须满足 FR-045 的四阶段要求。
- **FR-021**: 系统必须把取消从父 token(令牌) 传播到 child token(子令牌)，并且不得要求 child token(子令牌) 取消父 token(令牌)。
- **FR-022**: 系统必须保证 root shutdown(根关闭) 后不留下 orphan task(孤儿任务)。
- **FR-023**: 系统必须提供 `SupervisorHandle`(监督器句柄) 命令：`add_child`、`remove_child`、`restart_child`、`pause_child`、`resume_child`、`quarantine_child`、`shutdown_tree`、`snapshot` 和 `subscribe_events`。
- **FR-024**: 系统必须让控制命令保持幂等；对已停止、已暂停、已恢复或已隔离 child(子任务) 重复执行命令时，必须返回当前状态或已接受结果。
- **FR-025**: 系统必须把最新状态快照和生命周期事件历史分开保存。
- **FR-026**: 系统必须通过适合 watch-style receiver(观察式接收者) 的状态平面暴露最新状态，使只需要最新值的消费者可以读取它。
- **FR-027**: 系统必须通过适合 broadcast subscriber(广播订阅者) 或自定义 fan-out(扇出) 机制的事件总线暴露完整生命周期历史。
- **FR-028**: 系统必须用 `When`(何时)、`Where`(何处) 和 `What`(发生内容) 字段建模生命周期事件。
- **FR-029**: 系统必须在 `When`(何时) 数据中包含 wall time(墙钟时间)、monotonic time(单调时间)、sequence(序号)、attempt(尝试次数) 和 generation(代次)。
- **FR-030**: 系统必须在可用时把 supervisor path(监督器路径)、child id(子任务标识)、parent id(父标识)、task name(任务名称)、task id(任务标识)、host(主机)、pid(进程标识)、thread name(线程名称) 和 registration location(注册位置) 放入 `Where`(何处) 数据。
- **FR-031**: 系统必须在适用时把 event type(事件类型)、state transition(状态迁移)、exit reason(退出原因)、error category(错误类别)、restart decision(重启决策)、backoff duration(退避时长)、health status(健康状态) 和 triggering command(触发命令) 放入 `What`(发生内容) 数据。
- **FR-032**: 系统必须为每次状态迁移创建 observability event(可观察性事件)。
- **FR-033**: 系统必须以 `tracing`(结构化追踪) 作为结构化观察基础。每个 child attempt(子任务尝试) 必须有自己的 span(追踪范围)，每次状态迁移必须发出 event(追踪事件)。
- **FR-034**: 系统必须至少导出这些 metrics(指标)：`supervisor_restart_total`、`supervisor_child_state`、`supervisor_child_uptime_seconds`、`supervisor_backoff_seconds`、`supervisor_healthcheck_latency_seconds`、`supervisor_meltdown_total`、`supervisor_shutdown_duration_seconds` 和 `supervisor_event_lag_total`。
- **FR-035**: 系统必须把 supervisor(监督器) 工作移出 business hot path(业务热路径)；高频交易、盘口和撮合逻辑不得让每条消息都经过 supervisor(监督器)。
- **FR-036**: 系统必须支持 test time(测试时间)，使 backoff(退避)、timeout(超时)、heartbeat(心跳) 和 meltdown window(熔断窗口) 测试可以确定性推进虚拟时间。
- **FR-037**: 系统必须为每个控制命令产生 audit command event(审计命令事件)，并包含 `command_id`、`requested_by`、`reason`、`target_path`、`accepted_at` 和 `result`。
- **FR-038**: 系统不得把 actor-model(参与者模型) 要求引入用户可见模型；监督必须用 children(子任务)、trees(树)、policies(策略)、outcomes(结果)、events(事件)、handles(句柄)、snapshots(快照) 和 commands(命令) 表达。
- **FR-039**: 系统不得添加第三方 compatibility exports(兼容导出)，也不得复制参考 crate(库) 的公开 API(接口) 形状。
- **FR-040**: 系统必须只把 `supertrees` 当作概念输入，不能把它作为生产核心依赖。
- **FR-041**: 系统必须明确分离 control plane(控制面) 和 data plane(数据面)。supervisor(监督器) 只管理生命周期、状态、事件、重启、关闭和控制命令，业务消息处理必须留在 data plane(数据面)。
- **FR-042**: 系统必须按声明顺序启动 child(子任务)，并按声明顺序的逆序关闭 child(子任务)。
- **FR-043**: 系统必须把 readiness(就绪) 建模为一等生命周期信号。需要预热、连接建立或订阅恢复的 child(子任务) 必须显式报告 ready(已就绪)，默认立即就绪只能作为明确策略存在。
- **FR-044**: 系统必须单独建模 `spawn_blocking`(阻塞任务启动) 和其它 blocking task(阻塞任务)。blocking task(阻塞任务) 必须有独立 `TaskKind`(任务类型)、关闭策略和升级策略，并且不得复用普通 async task(异步任务) 可强制终止的假设。
- **FR-045**: 系统的关闭协议必须包含 request stop(请求停止)、graceful drain(优雅排空)、abort stragglers(强制终止拖尾任务) 和 reconcile(状态对账) 四个内部阶段。reconcile(状态对账) 必须统一更新 registry(注册表)、snapshot(快照)、metrics(指标) 和 event journal(事件日志缓冲区)。
- **FR-046**: 系统必须维护固定容量 event journal(事件日志缓冲区)，并在 meltdown(熔断)、关闭超时或父级升级时生成 `RunSummary`(运行摘要)，用于解释最近生命周期事件、失败原因、重启次数、关闭原因和最终状态。
- **FR-047**: 系统必须限制 metrics label(指标标签) 为低基数值。指标标签可以使用 supervisor path(监督器路径)、child id(子任务标识)、state(状态)、decision(决定) 和 failure category(失败类别)，不得包含错误全文、动态路径碎片、用户输入或其它无界值。
- **FR-048**: 系统可以在 `TaskFactory`(任务工厂) 内核之上提供 `Service trait`(服务特征) 和 `service_fn`(函数适配器) 人体工学层，但该适配层不得替换 `TaskFactory`(任务工厂) 内核，也不得引入第三方 compatibility exports(兼容导出)。

### Key Entities(关键实体) *(include if feature involves data(涉及数据时填写))*

- **Supervisor(监督器)**: 运行时治理节点，负责 child(子任务) 注册、策略评估、状态跟踪、事件发送、重启编排和关闭协调。
- **SupervisorTree(监督树)**: 分层结构，其中 root(根节点) 和子 supervisor(监督器) 治理 worker(工作任务) 和嵌套监督范围。
- **ChildSpec(子任务规格)**: 声明式 child(子任务) 配置，包含身份、任务种类、策略、依赖、标签、关键程度和 factory(工厂)。
- **SupervisorSpec(监督器规格)**: 声明式 supervisor(监督器) 配置，包含策略、children(子任务集合)、fuse policy(熔断策略)、默认策略和路径前缀。
- **SupervisorPath(监督器路径)**: 稳定树路径，用于事件、指标、日志、快照和控制命令。
- **ChildId(子任务标识)**: child(子任务) 在父 supervisor(监督器) 内的稳定唯一标识。
- **TaskFactory(任务工厂)**: 每次启动或重启时构造新任务尝试的工厂。
- **TaskCtx(任务上下文)**: 传给任务尝试的上下文，包含身份、路径、代次、尝试次数、取消、心跳和事件接收点。
- **ChildRuntime(子任务运行态)**: 当前运行态记录，包含状态、代次、尝试次数、心跳、join handle(等待句柄)、取消令牌、重启计数和最近失败。
- **Registry(任务注册表)**: 当前运行时索引，保存 child(子任务) 规格和运行态。
- **SupervisionStrategy(监督策略)**: 重启范围决定，包含 `OneForOne`(一对一)、`OneForAll`(一对全部) 和 `RestForOne`(从失败处开始)。
- **RestartPolicy(重启策略)**: 退出到重启的规则，包含 `Permanent`(永久)、`Transient`(瞬时) 和 `Temporary`(临时)。
- **BackoffPolicy(退避策略)**: 重启延迟规则，包含指数增长、最大延迟、抖动和稳定后重置。
- **MeltdownPolicy(熔断策略)**: child-level(子任务级) 和 supervisor-level(监督器级) 熔断阈值及重置窗口。
- **HealthPolicy(健康策略)**: heartbeat interval(心跳间隔) 和 stale-after threshold(过期阈值)，用于检测不健康任务。
- **ReadinessPolicy(就绪策略)**: 定义 child(子任务) 何时可以从 running(运行中) 进入 ready(已就绪)，并支持默认立即就绪和显式就绪两种策略。
- **ShutdownPolicy(关闭策略)**: graceful timeout(优雅关闭超时) 和 abort-after-timeout(超时后强制终止) 行为。
- **TaskKind(任务类型)**: 区分 async worker(异步工作任务)、blocking worker(阻塞工作任务) 和 supervisor(监督器)，并决定关闭和升级边界。
- **TaskExit(任务退出)**: 退出分类，例如已完成、已失败、已取消、已超时、不健康或已恐慌。
- **TaskFailureKind(任务失败类别)**: 策略引擎使用的类型化错误类别。
- **RestartDecision(重启决策)**: 策略结果，例如不重启、延迟后重启、隔离、向父级升级或关闭整棵树。
- **SupervisorHandle(监督器句柄)**: 运行时控制平面，用于命令、关闭、快照和事件订阅。
- **ControlCommand(控制命令)**: 可审计的运行时命令，包含请求者、原因、目标路径和结果。
- **SupervisorSnapshot(监督器快照)**: 最新状态视图，包含树、children(子任务集合)、健康状态、计数器和终态。
- **SupervisorEvent(监督器事件)**: 完整生命周期记录，携带 `When`(何时)、`Where`(何处)、`What`(发生内容)、策略决定、序号和 correlation id(关联标识)。
- **EventJournal(事件日志缓冲区)**: 固定容量生命周期事件缓冲区，用于 meltdown(熔断)、关闭超时和父级升级后的诊断回放。
- **RunSummary(运行摘要)**: 运行结束或故障升级时产生的摘要，包含开始时间、结束时间、关闭原因、重启次数、失败列表、最近事件和最终状态。
- **Service(服务特征)**: 建立在 `TaskFactory`(任务工厂) 之上的可选人体工学适配层，用于让调用者以服务对象或 `service_fn`(函数适配器) 形式接入监督器。

## Constitution Alignment(宪章对齐) *(mandatory(必填))*

### Supervision Contract(监督契约)

- **Lifecycle impact(生命周期影响)**: 本功能定义声明、启动、运行、就绪、暂停、恢复、健康检查、失败、重启、隔离、升级、关闭、强制终止、状态对账和报告 child(子任务) 工作的生命周期治理。
- **Failure behavior(失败行为)**: 失败必须类型化，必须关联 child path(子任务路径) 和 attempt(尝试次数)，必须经过重启、退避、熔断、关键程度和策略评估，并必须作为可查询事件发送。
- **Shutdown behavior(关闭行为)**: 关闭是一等 shutdown protocol(关闭协议)。父取消必须传播到 child token(子令牌)，系统必须等待 graceful timeout(优雅关闭超时)，只有超时后才 abort(强制终止)，root shutdown(根关闭) 必须证明没有 orphan task(孤儿任务)，并且必须在 request stop(请求停止)、graceful drain(优雅排空)、abort stragglers(强制终止拖尾任务) 和 reconcile(状态对账) 四个内部阶段后完成。

### Rust Boundary and Observability Requirements(Rust 边界和可观察性需求)

- **Module ownership(模块所有权)**: 计划必须把声明式规格、身份、任务工厂和上下文、运行时绑定、child runner(子任务运行器)、树编排、策略引擎、健康、控制平面、注册表、事件模型、快照存储、可观察性、关闭、错误类型和测试支持拆成独立所有权边界。
- **Compatibility exports(兼容导出)**: None(无)。
- **Diagnostics(诊断)**: 每次生命周期迁移都必须能通过快照状态、事件流、event journal(事件日志缓冲区)、`RunSummary`(运行摘要)、tracing span(追踪范围)、tracing event(追踪事件)、指标更新和命令审计记录解释。
- **Dependency impact(依赖影响)**: 计划确认 Tokio(异步运行时) 运行时原语、取消、tracing(结构化追踪)、metrics(指标) 和事件 fan-out(扇出) 支持生命周期契约后，可以使用它们。actor framework(参与者框架) 和复制第三方 supervisor(监督器) API(接口) 不在范围内。

### Chinese Writing(中文写作)

- **Writing language(写作语言)**: 本规格使用中文写作。
- **Term format(术语格式)**: 英文术语以 `English(中文说明)` 形式出现。
- **Forbidden style(禁止风格)**: 本规格不使用非中文正文、片段式语言、生僻词或方言。

## Success Criteria(成功标准) *(mandatory(必填))*

### Measurable Outcomes(可衡量结果)

- **SC-001**: 维护者可以在 15 分钟内通过一个示例声明包含身份、策略、健康、关闭、依赖、标签和关键程度的 child(子任务)。
- **SC-002**: 任意 child(子任务) panic(恐慌) 后，系统必须记录 `ChildPanicked`、`BackoffScheduled` 和 `ChildRestarting` 事件，并且必须在重启后的 child(子任务) 运行前递增 attempt(尝试次数)。
- **SC-003**: 同一个 child(子任务) 在 60 秒内第 11 次重启时，系统必须把该 child(子任务) 放入 `Quarantined`(已隔离)，并阻止后续自动重启。
- **SC-004**: 同一个 supervisor(监督器) 范围在 60 秒内第 31 次 child(子任务) 失败时，系统必须发送 `Meltdown`(熔断) 并向父 supervisor(监督器) 升级。
- **SC-005**: root shutdown(根关闭) 必须取消所有 child token(子令牌)，并且在任务集合为空时完成，不留下 orphan task(孤儿任务)。
- **SC-006**: 在 `OneForAll`(一对全部) 下，任意 child(子任务) 失败后，系统必须先停止所有 sibling(同级任务)，再按定义顺序重启整组。
- **SC-007**: 在 `RestForOne`(从失败处开始) 下，失败 child(子任务) 之前定义的 child(子任务) 不得重启，失败 child(子任务) 和之后的 child(子任务) 必须重启。
- **SC-008**: 每次状态迁移都必须产生一条包含 `When`(何时)、`Where`(何处) 和 `What`(发生内容) 字段的事件。
- **SC-009**: 状态快照必须返回每个 child(子任务) 的当前状态、健康状态、generation(代次)、attempt(尝试次数)、restart count(重启次数)、last failure(最近失败) 和 path(路径)。
- **SC-010**: 所有 backoff(退避)、timeout(超时)、heartbeat(心跳) 和 meltdown(熔断) 测试都必须使用确定的 test time(测试时间)，不得依赖真实 sleep(睡眠)。
- **SC-011**: 100% 控制命令审计日志必须说明请求者、原因、目标路径、接受时间、command id(命令标识) 和结果。
- **SC-012**: 公开模型必须包含 supervisor tree(监督树)、child spec(子任务规格)、task factory(任务工厂)、policy(策略)、health(健康)、shutdown(关闭)、event(事件)、snapshot(快照)、metrics(指标)、audit(审计) 和 handle(句柄) 概念，并且不得出现 actor-model(参与者模型) 术语。
- **SC-013**: root shutdown(根关闭) 必须按声明顺序的逆序关闭 child(子任务)，并在完成后证明 registry(注册表)、snapshot(快照)、metrics(指标) 和 event journal(事件日志缓冲区) 的最终状态一致。
- **SC-014**: 需要显式 readiness(就绪) 的 child(子任务) 在报告 ready(已就绪) 前，不得在 snapshot(快照) 或 event(事件) 中显示为 ready(已就绪)。
- **SC-015**: blocking task(阻塞任务) 在关闭超时后必须产生说明不可立即终止边界的事件和策略决定，并且必须按升级策略处理。
- **SC-016**: 指标导出检查必须验证所有 metrics label(指标标签) 均为低基数值，并拒绝错误全文、用户输入和无界动态值。
- **SC-017**: meltdown(熔断)、关闭超时或父级升级发生时，系统必须生成 `RunSummary`(运行摘要)，并包含最近 event journal(事件日志缓冲区) 中的关键事件。

## Assumptions(假设)

- 默认网络连接类 worker(工作任务) 使用 `Transient`(瞬时)、`OneForOne`(一对一)、初始 backoff(退避) 100ms(毫秒)、最大 backoff(退避) 5s(秒)、10% jitter(抖动)、reset_after(重置时间) 60s(秒)、heartbeat interval(心跳间隔) 1s(秒)、stale_after(过期时间) 3s(秒)、graceful shutdown timeout(优雅关闭超时) 5s(秒) 和 abort wait(强制终止等待) 1s(秒)。
- 默认核心协调类 worker(工作任务) 使用 `Permanent`(永久)，但 meltdown(熔断) 后必须向父 supervisor(监督器) 升级，不能无限重启。
- `Permanent`(永久) 表示正常退出或异常退出后都重启；`Transient`(瞬时) 表示仅在异常退出、panic(恐慌)、timeout(超时) 或 unhealthy(不健康) 后重启；`Temporary`(临时) 表示永不重启。
- `OneForOne`(一对一) 只重启失败 child(子任务)；`OneForAll`(一对全部) 停止并重启 supervisor(监督器) 范围内所有 child(子任务)；`RestForOne`(从失败处开始) 重启失败 child(子任务) 和其后按定义顺序排列的 child(子任务)。
- child-level fuse(子任务级熔断) 和 supervisor-level fuse(监督器级熔断) 都存在；quarantine(隔离) 是 child-level(子任务级) 终态治理状态，meltdown(熔断) 是升级信号。
- 状态快照和生命周期事件是两种不同产物：快照回答当前状态，事件回答历史顺序。
- `When`(何时)、`Where`(何处) 和 `What`(发生内容) 是标准事件词汇，不能被模糊日志术语替代。
- 默认 child(子任务) 可以采用 immediate readiness(立即就绪)，但任何需要预热、建连或恢复订阅的 child(子任务) 必须选择 explicit readiness(显式就绪)。
- `TaskFactory`(任务工厂) 是监督器内核入口；`Service trait`(服务特征) 和 `service_fn`(函数适配器) 只是可选适配层。
- 参考 crate(库) 只提供概念来源，不需要也不允许第三方 API(接口) compatibility surface(兼容表面)。
- 第一版实现面向一个进程和一个 Tokio(异步运行时)。distributed supervision(分布式监督)、cross-process messaging(跨进程消息) 和 remote control(远程控制) 不在本功能范围内。
