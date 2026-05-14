# Feature Specification(功能规格): 子任务槽位控制

**Feature Branch(功能分支)**: `004-runtime-semantics`
**Created(创建日期)**: 2026-05-14
**Updated(更新日期)**: 2026-05-15
**Status(状态)**: Draft(草稿)
**Input(输入)**: 用户描述: "当前 TaskContext(任务上下文) 有 CancellationToken(取消令牌), 但是 ChildRunner(子任务运行器) 创建的取消令牌没有被 runtime registry(运行时注册表) 保存, 因此控制命令无法真正取消任务. PauseChild(暂停子任务), RemoveChild(移除子任务), QuarantineChild(隔离子任务) 基本只是改 ManagedChildState(受管子任务状态). 工业级版本需要把每个 child slot(子任务槽位) 设计为: spec(声明) + generation(代数) + attempt(尝试次数) + status(状态) + cancellation_token(取消令牌) + join_handle(任务句柄) + last_heartbeat(最后心跳) + ready_state(就绪状态) + restart_budget(重启预算). 所有控制命令必须作用在这个真实状态上."

## User Scenarios & Testing(用户场景和测试) *(mandatory(必填))*

### User Story 1(用户故事一) - 查看真实子任务槽位状态 (Priority(优先级): P1)

操作者需要从单一来源读取 child slot(子任务槽位) 的真实运行信息, 包括当前活动尝试, 代数, 健康信号, 就绪状态和重启预算余量, 而不是只看到一个被动写入的状态枚举.

**Why this priority(为什么是这个优先级)**: 控制命令和监督决策必须以真实槽位事实为输入, 否则后续动作都会建立在过期或伪造的状态之上.

**Independent Test(独立测试)**: 启动一个会上报 heartbeat(心跳) 和 readiness(就绪状态) 的任务, 单次读取当前状态, 验证槽位显示最新心跳, 就绪状态, 当前尝试代数和重启预算余量.

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** 子任务已经启动并周期上报 heartbeat(心跳), **When(当)** 操作者读取当前状态, **Then(则)** 系统必须返回该槽位的当前活动尝试, 状态和最后心跳时刻.
2. **Given(假设)** 子任务已经上报 readiness(就绪状态), **When(当)** 操作者读取当前状态, **Then(则)** 系统必须返回 ready(已就绪) 状态以及对应的尝试标识.
3. **Given(假设)** 子任务还没有上报 heartbeat(心跳), **When(当)** 操作者读取当前状态, **Then(则)** 系统必须明确区分 "未收到心跳" 和 "心跳超时", 不得伪造心跳值.

---

### User Story 2(用户故事二) - 控制命令停止真实运行任务 (Priority(优先级): P2)

操作者执行 PauseChild(暂停子任务), RemoveChild(移除子任务) 或 QuarantineChild(隔离子任务) 时, 系统必须对槽位当前的活动尝试发出真实的取消或等待动作, 不得只更新对外状态枚举.

**Why this priority(为什么是这个优先级)**: 只改状态而不停止活动尝试时, 任务会继续消费消息, 写入外部系统或持有锁, 与控制命令意图相反.

**Independent Test(独立测试)**: 启动一个长运行任务, 分别执行 PauseChild(暂停子任务), RemoveChild(移除子任务) 和 QuarantineChild(隔离子任务), 验证任务真实收到取消信号, 并且控制结果说明当前活动尝试的停止状态.

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** 子任务正在运行, **When(当)** 操作者执行 PauseChild(暂停子任务), **Then(则)** 系统必须暂停自动治理并向当前活动尝试发送取消信号, 直到该尝试进入终止状态.
2. **Given(假设)** 子任务正在运行, **When(当)** 操作者执行 RemoveChild(移除子任务), **Then(则)** 系统必须等待或强制中止当前活动尝试后再移除该槽位, 不得在仍有活动尝试时丢弃槽位记录.
3. **Given(假设)** 子任务正在运行, **When(当)** 操作者执行 QuarantineChild(隔离子任务), **Then(则)** 系统必须停止当前活动尝试并阻止 supervision strategy(监督策略) 触发新的自动重启.

---

### User Story 3(用户故事三) - 让控制结果反映槽位事实 (Priority(优先级): P3)

操作者需要控制命令返回真实的槽位变化, 包括目标子任务, 目标尝试, 取消是否送达, 是否等待完成, 是否仍有活动任务, 当前重启预算余量, 以及失败时的具体阶段和原因.

**Why this priority(为什么是这个优先级)**: 调用方需要根据命令结果判断下一步动作, 不能依靠只表示意图的状态枚举.

**Independent Test(独立测试)**: 对处于不同状态的槽位执行控制命令, 验证命令结果包含目标子任务标识, 目标尝试标识, 是否真实停止, 是否幂等, 以及是否仍有运行尝试.

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** 子任务已经停止或从未启动, **When(当)** 操作者重复执行停止类控制命令, **Then(则)** 系统必须返回幂等结果, 不得重复发送取消信号或制造新的副作用.
2. **Given(假设)** 子任务停止失败, **When(当)** 控制命令返回, **Then(则)** 系统必须指出失败的阶段, 目标子任务标识和真实原因, 不得只返回成功或泛化错误.
3. **Given(假设)** 重启预算已经耗尽, **When(当)** 操作者读取控制结果, **Then(则)** 系统必须显示当前预算余量并说明不会再触发自动重启.

### Edge Cases(边界情况)

- 槽位刚刚启动, 首次 heartbeat(心跳) 还没送达时, 当前状态必须区分 "未收到心跳" 和 "心跳超时", 不得用零值伪造心跳.
- 任务上报 heartbeat(心跳) 后立即退出时, 当前状态必须以最终退出结果为主, 同时保留最后心跳作为历史信息.
- 自动重启已经把活动尝试推进到新 generation(代数) 或新 attempt(尝试) 时, 控制命令必须以槽位当前活动尝试为目标, 不得跨 attempt 误送取消信号.
- restart_budget(重启预算) 已经耗尽时, 控制结果必须显示当前预算余量, 并说明系统不会再自动重启.
- ready_state(就绪状态) 退化或从未上报时, 当前状态必须区分这两种情况, 不得把未上报视为退化.
- 控制命令与 supervision strategy(监督策略) 触发的自动重启在同一时刻发生时, 系统必须以槽位当前治理状态为决策依据, 不得让两个动作互相覆盖.

## Requirements(需求) *(mandatory(必填))*

### Functional Requirements(功能需求)

- **FR-001**: 系统必须为每个 child(子任务) 维护一个 child slot(子任务槽位), 该槽位必须真实表达声明, generation(代数), 当前 attempt(尝试), status(状态), cancellation_token(取消令牌), join_handle(任务句柄), last_heartbeat(最后心跳), ready_state(就绪状态) 和 restart_budget(重启预算), 并且这些字段必须可以被外部读取.
- **FR-002**: PauseChild(暂停子任务), RemoveChild(移除子任务) 和 QuarantineChild(隔离子任务) 必须作用于 child slot(子任务槽位) 当前活动尝试的真实生命周期, 包括 cancellation_token(取消令牌) 送达, 等待或强制中止, 而不仅仅是更新 ManagedChildState(受管子任务状态) 枚举.
- **FR-003**: 控制命令的返回结果和当前状态读取必须反映 child slot(子任务槽位) 的真实事实, 包括目标 child id(子任务标识), 目标 attempt(尝试) 标识, cancellation_token(取消令牌) 送达情况, 等待结果, restart_budget(重启预算) 余量, 以及失败阶段和原因.

### Key Entities(关键实体)

- **ChildSlot(子任务槽位)**: 表示一个 child(子任务) 的声明, 当前活动尝试和运行时治理状态, 字段覆盖 generation(代数), attempt(尝试), status(状态), cancellation_token(取消令牌), join_handle(任务句柄), last_heartbeat(最后心跳), ready_state(就绪状态) 和 restart_budget(重启预算).
- **ChildControlResult(子任务控制结果)**: 表示一次控制命令对槽位产生的真实结果, 包括目标 child id(子任务标识), 目标 attempt(尝试), 取消送达, 等待结束, 幂等返回或失败原因.
- **RestartBudget(重启预算)**: 表示当前 child(子任务) 在 supervision strategy(监督策略) 窗口内还可以使用的重启额度, 是 ChildSlot(子任务槽位) 必须暴露的字段, 也是控制结果必须引用的对象.

## Constitution Alignment(宪章对齐) *(mandatory(必填))*

### Supervision Contract(监督契约)

- **Lifecycle impact(生命周期影响)**: 本规格改变暂停, 移除, 隔离, 自动重启交互和当前状态读取的语义, 把这些动作绑定到 child slot(子任务槽位) 的真实活动尝试.
- **Failure behavior(失败行为)**: 控制命令失败必须指出 child id(子任务标识), 目标 attempt(尝试), 当前槽位状态, 失败阶段和真实原因.
- **Shutdown behavior(关闭行为)**: 停止类控制命令必须复用 `004-2-real-shutdown-pipeline` 中的取消和等待语义, 不得在 child slot(子任务槽位) 层另起一套关闭路径.

### Rust Boundary and Observability Requirements(Rust 边界和可观察性需求)

- **Module ownership(模块所有权)**: registry(注册表) 或 runtime(运行时) 模块拥有 child slot(子任务槽位) 的字段和句柄, control(控制) 模块只暴露命令接口和结果, supervision strategy(监督策略) 模块只读 restart_budget(重启预算) 余量.
- **Compatibility exports(兼容导出)**: None(无)
- **Diagnostics(诊断)**: 必须记录槽位状态变化, cancellation_token(取消令牌) 送达, 控制命令结果, heartbeat(心跳) 更新, ready_state(就绪状态) 变化和 restart_budget(重启预算) 消耗.
- **Dependency impact(依赖影响)**: 不预设新增 crate(库). 如果实现阶段需要新增依赖, plan(计划) 必须说明理由.

### Chinese Writing(中文写作)

- **Writing language(写作语言)**: 本文档必须使用中文.
- **Term format(术语格式)**: 英文术语必须写成 `English(中文说明)`.
- **Forbidden style(禁止风格)**: 禁止非中文写作, 片段式语言, 生僻词和方言.

## Success Criteria(成功标准) *(mandatory(必填))*

### Measurable Outcomes(可衡量结果)

- **SC-001**: 100% 的运行中 child(子任务) 在一次状态读取中可以同时获得 attempt(尝试), last_heartbeat(最后心跳), ready_state(就绪状态) 和 restart_budget(重启预算) 余量.
- **SC-002**: 对运行中任务执行 PauseChild(暂停子任务), RemoveChild(移除子任务) 和 QuarantineChild(隔离子任务) 时, 100% 的测试场景都能观察到 cancellation_token(取消令牌) 送达或者明确的停止失败原因.
- **SC-003**: 对已经停止或从未启动的槽位重复执行停止类控制命令 10 次, 每次都必须返回幂等结果, 并且不得重复发送 cancellation_token(取消令牌).
- **SC-004**: 控制命令返回结果中 100% 包含目标 child id(子任务标识), 目标 attempt(尝试) 标识和槽位最终状态.

## Assumptions(假设)

- 本规格依赖 `004-2-real-shutdown-pipeline` 的取消和等待语义, 复用其 cancellation_token(取消令牌) 和 abort handle(强制中止句柄) 模型, 不另起一套关闭路径.
- 功能目录名为 `004-3-child-slot-control`, 与功能分支名 `004-runtime-semantics` 一对多: 同分支上并列 `004-1`, `004-2`, `004-4` 等其他运行时语义切片.
- ManagedChildState(受管子任务状态) 可以继续作为对外简化状态展示, 但不再是唯一事实来源, 槽位字段才是真实事实.
- 本规格不要求新增动态子任务声明格式, 也不改变 supervision strategy(监督策略) 的重启决策算法, 只把策略窗口内已消耗预算和剩余预算暴露到槽位中.
- restart_budget(重启预算) 的窗口和上限沿用现有重启策略定义, 槽位只负责暴露当前余量, 不重复计算策略.
