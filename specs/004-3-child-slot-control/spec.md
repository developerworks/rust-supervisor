# Feature Specification(功能规格): 子任务槽位控制

**Feature Branch(功能分支)**: `004-runtime-semantics`
**Created(创建日期)**: 2026-05-14
**Status(状态)**: Draft(草稿)
**Input(输入)**: 用户描述: "当前 TaskContext(任务上下文) 有 CancellationToken(取消令牌), 但是 ChildRunner(子任务运行器) 创建的取消令牌没有被 runtime registry(运行时注册表) 保存, 因此控制命令无法真正取消任务. PauseChild(暂停子任务), RemoveChild(移除子任务), QuarantineChild(隔离子任务) 基本只是改 ManagedChildState(受管子任务状态). 工业级版本需要把每个 child slot(子任务槽位) 设计为: spec(声明) + generation(代数) + attempt(尝试次数) + status(状态) + cancellation_token(取消令牌) + join_handle(任务句柄) + last_heartbeat(最后心跳) + ready_state(就绪状态) + restart_budget(重启预算). 所有控制命令必须作用在这个真实状态上."

## User Scenarios & Testing(用户场景和测试) _(mandatory(必填))_

### User Story 1(用户故事一) - 查看真实子任务槽位状态 (Priority(优先级): P1)

操作者需要看到 child slot(子任务槽位) 的真实运行信息, 包括运行尝试, 健康信号, 就绪状态和重启预算, 不能只看到一个被写入的状态枚举.

**Why this priority(为什么是这个优先级)**: 控制命令只有依赖真实槽位状态, 才能避免误判任务是否仍在运行.

**Independent Test(独立测试)**: 启动一个会发送 heartbeat(心跳) 和 readiness(就绪状态) 的任务, 读取当前状态, 验证槽位显示最新心跳, 就绪状态和 attempt(尝试次数).

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** 子任务已经启动并发送 heartbeat(心跳), **When(当)** 操作者读取当前状态, **Then(则)** 系统必须显示该槽位的运行状态和最后心跳.
2. **Given(假设)** 子任务已经报告 readiness(就绪状态), **When(当)** 操作者读取当前状态, **Then(则)** 系统必须显示 ready(已就绪) 状态.

---

### User Story 2(用户故事二) - 控制命令停止真实运行任务 (Priority(优先级): P2)

操作者执行 PauseChild(暂停子任务), RemoveChild(移除子任务) 或 QuarantineChild(隔离子任务) 时, 系统必须对真实运行任务发出取消或等待动作, 不能只改状态.

**Why this priority(为什么是这个优先级)**: 只改状态不会停止任务继续消费消息, 写入外部系统或持有锁.

**Independent Test(独立测试)**: 启动一个长运行任务, 执行 PauseChild(暂停子任务), RemoveChild(移除子任务) 和 QuarantineChild(隔离子任务), 验证任务收到取消信号并且控制结果说明真实停止状态.

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** 子任务正在运行, **When(当)** 操作者执行 PauseChild(暂停子任务), **Then(则)** 系统必须暂停自动治理并让当前运行尝试进入停止流程.
2. **Given(假设)** 子任务正在运行, **When(当)** 操作者执行 RemoveChild(移除子任务), **Then(则)** 系统必须等待或中止当前运行尝试后再移除该槽位.
3. **Given(假设)** 子任务正在运行, **When(当)** 操作者执行 QuarantineChild(隔离子任务), **Then(则)** 系统必须停止当前运行尝试并阻止自动重启.

---

### User Story 3(用户故事三) - 让控制结果反映槽位事实 (Priority(优先级): P3)

操作者需要控制命令返回真实槽位变化, 包括取消是否送达, 是否等待完成, 是否仍有运行任务.

**Why this priority(为什么是这个优先级)**: 调用方需要根据命令结果判断下一步动作, 不能依赖只表示意图的状态.

**Independent Test(独立测试)**: 对不同状态的槽位执行控制命令, 验证命令结果准确说明是否已停止, 是否幂等, 是否还有活动任务.

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** 子任务已经停止, **When(当)** 操作者重复暂停或隔离, **Then(则)** 系统必须返回幂等结果并说明没有活动任务.
2. **Given(假设)** 子任务停止失败, **When(当)** 控制命令返回, **Then(则)** 系统必须指出失败阶段和子任务标识.

### Edge Cases(边界情况)

- 槽位没有活动任务时, 停止类命令必须幂等返回.
- 任务上报 heartbeat(心跳) 后立即退出时, 当前状态必须以最终退出结果为准.
- 控制命令和自动重启同时触发时, 系统必须以槽位中当前治理状态为准.

## Requirements(需求) _(mandatory(必填))_

### Functional Requirements(功能需求)

- **FR-001**: 系统必须为每个 child(子任务) 维护 child slot(子任务槽位), 该槽位必须表达声明, 代数, 尝试次数, 状态, 取消令牌, 任务句柄, 最后心跳, 就绪状态和重启预算.
- **FR-002**: PauseChild(暂停子任务), RemoveChild(移除子任务) 和 QuarantineChild(隔离子任务) 必须作用于 child slot(子任务槽位) 的真实活动任务, 而不是只写入 ManagedChildState(受管子任务状态).
- **FR-003**: 控制命令结果和当前状态必须反映 child slot(子任务槽位) 的真实运行事实, 包括活动尝试, 停止结果, 心跳, 就绪状态和重启预算.

### Key Entities(关键实体)

- **ChildSlot(子任务槽位)**: 表示一个 child(子任务) 的声明, 活动尝试和运行时治理状态.
- **ChildControlResult(子任务控制结果)**: 表示控制命令对槽位产生的真实停止, 等待, 幂等或失败结果.
- **RestartBudget(重启预算)**: 表示当前 child(子任务) 在策略窗口内还可以使用的重启额度.

## Constitution Alignment(宪章对齐) _(mandatory(必填))_

### Supervision Contract(监督契约)

- **Lifecycle impact(生命周期影响)**: 本规格改变暂停, 移除, 隔离和状态读取语义.
- **Failure behavior(失败行为)**: 控制命令失败必须指出 child(子任务), 当前槽位状态, 阶段和真实原因.
- **Shutdown behavior(关闭行为)**: 停止类控制命令必须使用与关闭流水线一致的取消和等待语义.

### Rust Boundary and Observability Requirements(Rust 边界和可观察性需求)

- **Module ownership(模块所有权)**: registry(注册表) 或 runtime(运行时) 模块拥有 child slot(子任务槽位) 状态, control(控制) 模块只暴露命令接口.
- **Compatibility exports(兼容导出)**: None(无)
- **Diagnostics(诊断)**: 必须记录槽位状态变化, 取消送达, 控制命令结果, 心跳更新和就绪状态更新.
- **Dependency impact(依赖影响)**: 不预设新增 crate(库). 如果实现阶段需要新增依赖, plan(计划) 必须说明理由.

### Chinese Writing(中文写作)

- **Writing language(写作语言)**: 本文档必须使用中文.
- **Term format(术语格式)**: 英文术语必须写成 `English(中文说明)`.
- **Forbidden style(禁止风格)**: 禁止非中文写作, 片段式语言, 生僻词和方言.

## Success Criteria(成功标准) _(mandatory(必填))_

### Measurable Outcomes(可衡量结果)

- **SC-001**: 100% 的运行中 child(子任务) 都能在当前状态中显示活动尝试和最后心跳字段.
- **SC-002**: 对运行中任务执行停止类控制命令时, 100% 的测试场景都能观察到取消送达或停止失败原因.
- **SC-003**: 对已经停止的槽位重复执行停止类控制命令 10 次, 每次都必须返回幂等结果.
- **SC-004**: 控制命令结果中 100% 包含目标 child(子任务) 和槽位最终状态.

## Assumptions(假设)

- 本规格依赖 `004-2-real-shutdown-pipeline` 的取消和等待语义.
- 本规格不要求新增动态子任务声明格式.
- 当前 ManagedChildState(受管子任务状态) 可以继续作为可见状态之一, 但不能作为唯一事实来源.
