# Feature Specification(功能规格): 真实关闭流水线

**Feature Branch(功能分支)**: `004-runtime-semantics`
**Created(创建日期)**: 2026-05-14
**Status(状态)**: Draft(草稿)
**Input(输入)**: 用户描述: "当前 ShutdownTree(关闭监督树) 只是推进阶段并标记完成. 它没有对正在运行的 child task(子任务) 发送取消信号, 没有等待 graceful drain(优雅排空), 没有超时后 abort stragglers(强制中止滞留任务), 也没有 join(等待结束) 每个任务. 应该重构成真实 shutdown pipeline(关闭流水线): 第一阶段发送 CancellationToken(取消令牌), 第二阶段按 shutdown_order(关闭顺序) 等待任务正常返回, 第三阶段对超时任务 abort(强制中止), 第四阶段清理运行时拥有的注册表, journal(日志) 和 metrics(指标) 输出, 对非运行时拥有的 socket(套接字) 记录对账状态, 最后返回每个 child(子任务) 的退出结果."

## User Scenarios & Testing(用户场景和测试) *(mandatory(必填))*

### User Story 1(用户故事一) - 请求所有任务协作关闭 (Priority(优先级): P1)

操作者请求 ShutdownTree(关闭监督树) 后, 所有运行中的 child task(子任务) 都必须收到取消信号, 以便任务可以主动释放资源并正常返回.

**Why this priority(为什么是这个优先级)**: 没有真实取消信号时, 关闭动作只是状态变化, 无法保证运行中的工作停止.

**Independent Test(独立测试)**: 启动多个长运行任务, 请求关闭监督树, 验证每个运行中任务都观察到取消信号.

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** 多个 child task(子任务) 正在运行, **When(当)** 操作者请求关闭监督树, **Then(则)** 系统必须向每个运行中任务发送取消信号.
2. **Given(假设)** 某个任务已在关闭请求前结束, **When(当)** 关闭开始, **Then(则)** 系统不得重复取消已经结束的任务.

---

### User Story 2(用户故事二) - 按关闭顺序等待任务结束 (Priority(优先级): P2)

操作者需要系统按 shutdown_order(关闭顺序) 等待任务正常返回, 并记录每个任务的退出结果.

**Why this priority(为什么是这个优先级)**: 关闭顺序影响依赖任务的资源释放, 也是 Shutdown Without Orphaned Tasks(关闭后不留下孤儿任务) 的核心证据.

**Independent Test(独立测试)**: 构造有依赖关系的监督树, 请求关闭, 验证等待顺序符合 shutdown_order(关闭顺序), 并且结果包含每个任务的退出分类.

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** 监督树中存在声明顺序和依赖关系, **When(当)** 系统进入 graceful drain(优雅排空), **Then(则)** 系统必须按 shutdown_order(关闭顺序) 等待任务结束.
2. **Given(假设)** 任务在超时前正常返回, **When(当)** 关闭结果生成, **Then(则)** 该任务必须被记录为 graceful(优雅完成).

---

### User Story 3(用户故事三) - 强制中止滞留任务并完成对账 (Priority(优先级): P3)

操作者需要系统在优雅排空超时后强制中止滞留任务, 再对注册表, runtime handles(运行时句柄), journal(日志), metrics(指标) 和 socket(套接字) 做最终对账. 核心 runtime(运行时) 不直接拥有 dashboard IPC socket(仪表盘进程间通信套接字) 时, 系统必须把 socket(套接字) 记录为 NotOwned(非运行时拥有), 而不是伪造清理动作.

**Why this priority(为什么是这个优先级)**: 工业系统不能让关闭无限等待, 也不能在关闭后留下未知状态.

**Independent Test(独立测试)**: 构造一个忽略取消信号的任务, 请求关闭, 验证该任务在超时后被强制中止, 最终结果说明它不是优雅退出.

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** 某个任务忽略取消信号并超过关闭预算, **When(当)** 系统进入 abort stragglers(强制中止滞留任务), **Then(则)** 系统必须强制中止该任务并记录原因.
2. **Given(假设)** 所有任务已经结束或被强制中止, **When(当)** 系统进入 reconcile(状态对账), **Then(则)** 系统必须清理运行时拥有的注册表和句柄资源, 记录 journal(日志), metrics(指标) 和 socket(套接字) 的对账状态, 并返回每个 child(子任务) 的最终结果.

### Edge Cases(边界情况)

- 没有运行中任务时, 关闭流水线必须仍然产生完整阶段结果, 并把每个声明 child(子任务) 记录为 AlreadyExited(已经退出).
- 重复 ShutdownTree(关闭监督树) 请求必须返回同一关闭结果或当前关闭进度.
- 关闭期间有任务迟到上报时, 系统必须把它归入对应 child(子任务) 的最终结果, 或标记为迟到报告.

## Requirements(需求) *(mandatory(必填))*

### Functional Requirements(功能需求)

- **FR-001**: ShutdownTree(关闭监督树) 必须向所有运行中的 child task(子任务) 发送 CancellationToken(取消令牌), 并记录取消请求已经送达的任务集合.
- **FR-002**: 系统必须按 shutdown_order(关闭顺序) 等待运行中任务完成 graceful drain(优雅排空), 并为每个 child(子任务) 记录退出结果.
- **FR-003**: 系统必须在关闭预算超时后 abort stragglers(强制中止滞留任务), 清理运行时拥有的资源, 记录非运行时拥有资源的对账状态, 并返回覆盖每个 child(子任务) 的关闭摘要.

### Key Entities(关键实体)

- **ShutdownPipeline(关闭流水线)**: 表示取消, 等待, 强制中止和对账四个关闭阶段.
- **ChildShutdownOutcome(子任务关闭结果)**: 表示每个 child(子任务) 是优雅完成, 被强制中止, 已经退出, 还是关闭失败.
- **ShutdownReconcileReport(关闭对账报告)**: 表示关闭后注册表, socket(套接字), journal(日志) 和 metrics(指标) 的最终状态.

## Constitution Alignment(宪章对齐) *(mandatory(必填))*

### Supervision Contract(监督契约)

- **Lifecycle impact(生命周期影响)**: 本规格改变 ShutdownTree(关闭监督树) 的停止, 等待, 超时和清理语义.
- **Failure behavior(失败行为)**: 关闭失败必须指出具体 child(子任务), 阶段和原因.
- **Shutdown behavior(关闭行为)**: 关闭必须真实取消运行中任务, 等待任务结束, 超时后强制中止, 并完成最终对账.

### Rust Boundary and Observability Requirements(Rust 边界和可观察性需求)

- **Module ownership(模块所有权)**: shutdown(关闭) 模块保留阶段契约, runtime(运行时) 模块拥有任务句柄, 取消令牌和关闭流水线执行.
- **Compatibility exports(兼容导出)**: None(无)
- **Diagnostics(诊断)**: 必须记录关闭阶段变化, 每个 child(子任务) 的取消送达, 等待完成, 超时和强制中止.
- **Dependency impact(依赖影响)**: 不预设新增 crate(库). 如果实现阶段需要新增依赖, plan(计划) 必须说明理由.

### Chinese Writing(中文写作)

- **Writing language(写作语言)**: 本文档必须使用中文.
- **Term format(术语格式)**: 英文术语必须写成 `English(中文说明)`.
- **Forbidden style(禁止风格)**: 禁止非中文写作, 片段式语言, 生僻词和方言.

## Success Criteria(成功标准) *(mandatory(必填))*

### Measurable Outcomes(可衡量结果)

- **SC-001**: 关闭请求发出后, 100% 的运行中任务都能在关闭结果中显示取消送达状态.
- **SC-002**: 有依赖关系的监督树在 100% 的测试场景中按 shutdown_order(关闭顺序) 记录等待顺序.
- **SC-003**: 忽略取消信号的任务在超出关闭预算后, 100% 被记录为强制中止或关闭失败.
- **SC-004**: 关闭完成后, 100% 的测试场景都能获得覆盖全部声明 child(子任务) 的关闭摘要.

## Assumptions(假设)

- 本规格依赖 `004-1-runtime-lifecycle-guard` 提供的运行时健康和等待语义.
- ShutdownCoordinator(关闭协调器) 继续作为阶段状态机, 不直接拥有任务句柄.
- 本规格不改变 supervision strategy(监督策略) 的重启决策, 只改变关闭执行语义.
