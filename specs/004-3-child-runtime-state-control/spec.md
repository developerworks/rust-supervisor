# Feature Specification(功能规格): 子任务运行状态控制

**Feature Branch(功能分支)**: `004-runtime-semantics`
**Created(创建日期)**: 2026-05-14
**Updated(更新日期)**: 2026-05-15
**Status(状态)**: Draft(草稿)
**Input(输入)**: 用户描述整理后为: "当前 TaskContext(任务上下文) 有 CancellationToken(取消令牌), 但是 ChildRunner(子任务运行器) 创建的取消令牌没有被 runtime registry(运行时注册表) 保存, 因此控制命令无法真正取消任务. PauseChild(暂停子任务), RemoveChild(移除子任务), QuarantineChild(隔离子任务) 基本只是改 ManagedChildState(受管子任务状态). 工业级版本需要把每个 child runtime state(子任务运行状态记录) 设计为: spec(声明) + generation(代次) + attempt(尝试次数) + status(状态) + cancellation_token(取消令牌) + runtime_handle(运行时句柄) + last_heartbeat(最后心跳) + readiness(就绪状态) + restart_limit(重启次数限制). 所有控制命令必须作用在这个真实状态上."

## User Scenarios & Testing(用户场景和测试) *(mandatory(必填))*

### User Story 1(用户故事一) - 查看真实子任务尝试状态 (Priority(优先级): P1)

操作者需要从单一来源读取 child runtime state(子任务运行状态记录) 的真实运行信息, 包括当前活动尝试, 代次, 健康信号, 就绪状态和剩余重启次数, 而不是只看到一个被动写入的状态枚举.

**Why this priority(为什么是这个优先级)**: 控制命令和监督决策必须以真实运行状态事实为输入, 否则后续动作都会建立在过期或伪造的状态之上.

**Independent Test(独立测试)**: 启动一个会上报 heartbeat(心跳) 和 readiness(就绪状态) 的任务, 单次读取当前状态, 验证运行状态记录显示最新心跳, 就绪状态, 当前 generation(代次), attempt(尝试) 和剩余重启次数. 同一测试必须连续 20 次构造 `CurrentState(当前状态)` 调用结果, 并验证每次构造耗时都低于 1 毫秒.

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** 子任务已经启动并周期上报 heartbeat(心跳), **When(当)** 操作者读取当前状态, **Then(则)** 系统必须返回该运行状态记录的当前活动尝试, 状态和最后心跳时刻.
2. **Given(假设)** 子任务已经上报 readiness(就绪状态), **When(当)** 操作者读取当前状态, **Then(则)** 系统必须返回 ready(已就绪) 状态以及对应的尝试标识.
3. **Given(假设)** 子任务还没有上报 heartbeat(心跳), **When(当)** 操作者读取当前状态, **Then(则)** 系统必须明确区分 "未收到心跳" 和 "心跳超时", 不得伪造心跳值.

---

### User Story 2(用户故事二) - 控制命令停止真实运行任务 (Priority(优先级): P2)

操作者执行 PauseChild(暂停子任务), RemoveChild(移除子任务) 或 QuarantineChild(隔离子任务) 时, 系统必须对运行状态记录当前的活动尝试发出真实的取消或等待动作, 不得只更新对外状态枚举.

**Why this priority(为什么是这个优先级)**: 只改状态而不停止活动尝试时, 任务会继续消费消息, 写入外部系统或持有锁, 与控制命令要求执行的操作相反.

**Independent Test(独立测试)**: 启动一个长运行任务, 分别执行 PauseChild(暂停子任务), RemoveChild(移除子任务) 和 QuarantineChild(隔离子任务), 验证任务真实收到取消信号, 并且控制结果说明当前活动尝试的停止状态.

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** 子任务正在运行, **When(当)** 操作者执行 PauseChild(暂停子任务), **Then(则)** 系统必须暂停自动重启并向当前活动尝试发送取消信号, 直到该尝试进入终止状态.
2. **Given(假设)** 子任务正在运行, **When(当)** 操作者执行 RemoveChild(移除子任务), **Then(则)** 系统必须等待当前活动尝试终止后再移除该运行状态记录, 不得在仍有活动尝试时丢弃运行状态记录. 如果任务忽略取消信号, 系统必须在停止等待窗口超时后标记停止失败, 但运行状态记录必须继续存在, 直到 child(子任务) 真正退出.
3. **Given(假设)** 子任务正在运行, **When(当)** 操作者执行 QuarantineChild(隔离子任务), **Then(则)** 系统必须停止当前活动尝试并阻止 supervision strategy(监督策略) 触发新的自动重启.

---

### User Story 3(用户故事三) - 让控制结果反映运行状态事实 (Priority(优先级): P3)

操作者需要控制命令返回真实的运行状态记录变化, 包括目标子任务, 目标尝试, 取消是否送达, 是否等待完成, 是否仍有活动任务, 当前剩余重启次数, 以及失败时的具体阶段和原因.

**Why this priority(为什么是这个优先级)**: 调用方需要根据命令结果判断下一步动作, 不能依靠只表示操作模式的状态枚举.

**Independent Test(独立测试)**: 对处于不同状态的运行状态记录执行控制命令, 验证命令结果包含目标子任务标识, 目标尝试标识, 是否真实停止, 是否幂等, 以及是否仍有运行尝试.

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** 子任务已经停止或从未启动, **When(当)** 操作者重复执行停止类控制命令, **Then(则)** 系统必须返回幂等结果, 不得重复发送取消信号或制造新的副作用.
2. **Given(假设)** 子任务停止失败, **When(当)** 后续 `CurrentState(当前状态)` 或重复停止类控制命令返回, **Then(则)** 系统必须指出失败的阶段, 目标子任务标识, 目标 attempt(尝试), 当前子任务尝试状态和真实原因, 不得只返回成功或泛化错误.
3. **Given(假设)** 重启次数限制已经耗尽, **When(当)** 操作者读取控制结果, **Then(则)** 系统必须显示当前剩余重启次数并说明不会再触发自动重启.

### Edge Cases(边界情况)

- 运行状态记录刚刚启动, 首次 heartbeat(心跳) 还没送达时, 当前状态必须区分 "未收到心跳" 和 "心跳超时", 不得用零值伪造心跳.
- 任务上报 heartbeat(心跳) 后立即退出时, 当前状态必须以最终退出结果为主, 同时必须把最后心跳作为历史信息写入调用结果.
- 自动重启已经把活动尝试推进到新 generation(代次) 或新 attempt(尝试) 时, 控制命令必须以运行状态记录当前活动尝试为目标, 不得跨 attempt 误送取消信号.
- restart_limit(重启次数限制) 已经耗尽时, 控制结果必须显示当前剩余重启次数, 并说明系统不会再自动重启.
- readiness(就绪状态) 退化或从未上报时, 当前状态必须区分这两种情况, 不得把未上报视为退化.
- 控制命令与 supervision strategy(监督策略) 触发的自动重启在同一时刻发生时, 系统必须以运行状态记录当前操作为决策依据, 不得让两个动作互相覆盖.
- child(子任务) 已在 registry(注册表) 或 `child_runtime_states(子任务运行状态记录集合)` 中占位但尚未产生活动 attempt(尝试) 时, 停止类命令不得向不存在的任务体发送取消, 控制结果必须使用 `NoActiveAttempt(无活动尝试)` 语义, `attempt(尝试)` 为 `None(无值)`, `cancel_delivered(取消已送达)` 为 `false(否)`. 如果该命令改变 `operation(操作)` 或触发 `RemoveChild(移除子任务)` 的物理删除, `idempotent(幂等)` 必须为 `false(否)`. 只有命令到达时目标操作已经存在且没有物理删除动作时, `idempotent(幂等)` 才能为 `true(是)`. 验收用例见 `tasks.md` T036, 字段约束见 `data-model.md` 中 `ChildControlResult(子任务控制结果)` 校验规则.

## Requirements(需求) *(mandatory(必填))*

### Functional Requirements(功能需求)

- **FR-001**: 系统必须为每个 child(子任务) 维护一个 child runtime state(子任务运行状态记录), 该运行状态记录必须真实表达声明, generation(代次), 当前 attempt(尝试), status(状态), cancellation_token(取消令牌), runtime_handle(运行时句柄), last_heartbeat(最后心跳), readiness(就绪状态) 和 restart_limit(重启次数限制), 并且这些字段必须可以被外部读取. 当运行状态记录已经声明但尚无活动 attempt(尝试) 时, generation(代次), attempt(尝试), status(状态), cancellation_token(取消令牌), runtime_handle(运行时句柄), heartbeat(心跳) receiver(接收端) 和 readiness(就绪状态) receiver(接收端) 必须显式为 `None(无值)`, 不得使用其他空状态, 也不得伪造活动尝试.
- **FR-002**: PauseChild(暂停子任务), RemoveChild(移除子任务) 和 QuarantineChild(隔离子任务) 必须作用于 child runtime state(子任务运行状态记录) 当前活动尝试的真实生命周期, 包括 cancellation_token(取消令牌) 送达和等待结果, 而不仅仅是更新 ManagedChildState(受管子任务状态) 枚举. 此处「等待结果」指在 control loop(控制循环) 单跳返回之后, 由 `ChildAttemptMessage::Exited(子任务退出消息)` 与 `stop_state(停止状态)` 等字段表达的可观察完成或失败事实, 不是要求在控制命令处理函数内同步 `await(异步等待)` child future(子任务 future) 终止. 注意: 强制中止 (abort) 不属于单条控制命令的行为范围, 它由 `004-2-real-shutdown-pipeline` 的 `ShutdownPipeline`(关闭流水线) 在关闭 supervisor tree(监督树) 时统一处理.
- **FR-003**: 控制命令的返回结果和当前状态读取必须反映 child runtime state(子任务运行状态记录) 的真实事实, 包括目标 child id(子任务标识), 目标 attempt(尝试) 标识, cancellation_token(取消令牌) 送达情况, 等待结果, restart_limit(重启次数限制) 剩余次数, 以及失败阶段和原因. 此处「等待结果」含义与 FR-002 中相同, 均指异步可观察的停止进度, 不是控制路径上的阻塞等待.

### Key Entities(关键实体)

- **ChildRuntimeState(子任务运行状态记录)**: 表示一个 child(子任务) 的声明, 当前活动尝试和运行时子任务控制操作. 该实体的字段必须覆盖 FR-001 的运行状态事实, 具体运行时字段映射由 `data-model.md` 统一定义, 本节不重复维护字段清单.
- **ChildControlResult(子任务控制结果)**: 表示一次控制命令对运行状态记录产生的真实结果, 包括目标 child id(子任务标识), 目标 attempt(尝试), 取消送达, 等待结束, 幂等返回或失败原因.
- **Generation(代次)**: 表示同一个 child(子任务) 跨重启产生的新旧运行实例编号. 它用于识别迟到报告和当前运行实例, 不是时间戳起点. 文档必须统一使用本术语, 不得使用其他中文名.
- **Attempt(尝试)**: 表示某次实际启动出来的任务尝试. 同一个 generation(代次) 内可以存在递增的 attempt(尝试), 但是同一运行状态记录在任意时刻只能有一个活动 attempt(尝试).
- **Epoch(纪元)**: 表示时间戳起点, 例如 `UNIX_EPOCH(Unix 纪元常量)`. 它只能用于 `updated_at_unix_nanos(更新时间纳秒数)` 这类时间戳字段, 不能用于表示任务运行代次.
- **RestartLimit(重启次数限制)**: 表示当前 child(子任务) 在 supervision strategy(监督策略) 窗口内还可以使用的重启次数限制, 是 ChildRuntimeState(子任务运行状态记录) 必须暴露的字段, 也是控制结果必须引用的对象.

> **Note(注)**: FR-001 的字段名为规格层的抽象描述, 运行时类型的字段映射详见 `data-model.md` 的 Field Mapping 表. `runtime_handle(运行时句柄)` 在运行时由 `abort_handle(强制中止句柄)` 与 `completion_receiver(完成接收端)` 共同实现, `restart_limit(重启次数限制)` 的具体状态字段见 `RestartLimitState(重启次数限制状态)`.

## Constitution Alignment(宪章对齐) *(mandatory(必填))*

### Supervision Contract(监督契约)

- **Lifecycle impact(生命周期影响)**: 本规格改变暂停, 移除, 隔离, 自动重启交互和当前状态读取的语义, 把这些动作绑定到 child runtime state(子任务运行状态记录) 的真实活动尝试.
- **Failure behavior(失败行为)**: 控制命令失败必须指出 child id(子任务标识), 目标 generation(代次), 目标 attempt(尝试), 当前子任务尝试状态, 停止状态, 失败阶段和真实原因.
- **Shutdown behavior(关闭行为)**: 停止类控制命令必须复用 `004-2-real-shutdown-pipeline` 中的取消和等待语义, 不得在 child runtime state(子任务运行状态记录) 层另起一套关闭路径.

### Rust Boundary and Observability Requirements(Rust 边界和可观察性需求)

- **Module ownership(模块所有权)**: runtime(运行时) 模块拥有 child runtime state(子任务运行状态记录) 的字段和句柄, control(控制) 模块拥有公开命令接口, 公开结果类型, 公开子任务尝试状态枚举和公开子任务控制操作枚举, supervision strategy(监督策略) 模块只读 restart_limit(重启次数限制) 剩余次数. control(控制) 模块不得反向依赖 runtime(运行时) 模块.
- **Compatibility exports(兼容导出)**: None(无)
- **Diagnostics(诊断)**: 必须记录子任务尝试状态变化, cancellation_token(取消令牌) 送达, 控制命令结果, heartbeat(心跳) 更新, readiness(就绪状态) 变化和 restart_limit(重启次数限制) 刷新记录.
- **Dependency impact(依赖影响)**: 不预设新增 crate(库). 如果实现阶段需要新增依赖, plan(计划) 必须说明理由.

### Chinese Writing(中文写作)

- **Writing language(写作语言)**: 本文档必须使用中文.
- **Term format(术语格式)**: 英文术语必须写成 `English(中文说明)`.
- **Forbidden style(禁止风格)**: 禁止非中文写作, 片段式语言, 生僻词和方言.

## Success Criteria(成功标准) *(mandatory(必填))*

### Measurable Outcomes(可衡量结果)

- **SC-001**: 100% 的运行中 child(子任务) 在一次状态读取中可以同时获得 attempt(尝试), last_heartbeat(最后心跳), readiness(就绪状态) 和 restart_limit(重启次数限制) 剩余次数. 代表性测试场景中, 连续 20 次构造 `CurrentState(当前状态)` 调用结果时, 每次构造耗时都必须低于 1 毫秒.
- **SC-002**: 对运行中任务执行 PauseChild(暂停子任务), RemoveChild(移除子任务) 和 QuarantineChild(隔离子任务) 时, 100% 的测试场景都能观察到 cancellation_token(取消令牌) 送达或者明确的停止失败原因.
- **SC-003**: 对已经处于目标操作且仍存在于 `child_runtime_states(子任务运行状态记录集合)` 中的运行状态记录, 重复执行同一停止类控制命令 10 次, 每次都必须返回幂等结果, 并且不得重复发送 cancellation_token(取消令牌). 本条幂等验收覆盖两类记录: 已经向活动 attempt(尝试) 送达取消且 `operation(操作)` 已经等于目标操作的记录, 以及没有活动 attempt(尝试) 且目标操作已经存在并且不会触发物理删除的记录. `RemoveChild(移除子任务)` 首次命中无活动 attempt(尝试) 的占位运行状态记录时必须物理删除运行状态记录, 该首次删除不是幂等返回; 删除后的再次命令使用既有 unknown child(未知子任务) 处理路径, 不属于本条运行状态记录级幂等验收.
- **SC-004**: 控制命令返回结果中 100% 包含目标 child id(子任务标识), `operation_after(命令后操作)`, `status(状态)`, `stop_state(停止状态)` 以及目标 attempt(尝试) 标识. 当运行状态记录没有活动 attempt(尝试) 时, 控制结果必须明确返回 `attempt = None(无值)`, `generation = None(无值)`, `status = None(无值)` 与 `stop_state = NoActiveAttempt(无活动尝试)`, 不得伪造 attempt(尝试).

**Acceptance note(验收说明)**: SC-001 是 `CurrentState(当前状态)` 性能目标的承载项. SC-002 至 SC-004 的回归测试只要通过 `CurrentState(当前状态)` 读取运行状态事实, 都必须复用 SC-001 中连续 20 次构造调用结果且每次低于 1 毫秒的目标, 不得把该目标只留在 `plan.md` 或 `tasks.md`.

## Assumptions(假设)

- 本规格依赖 `004-2-real-shutdown-pipeline` 的取消和等待语义, 复用其 cancellation_token(取消令牌) 和 abort handle(强制中止句柄) 模型, 不另起一套关闭路径.
- PauseChild(暂停子任务), RemoveChild(移除子任务) 和 QuarantineChild(隔离子任务) 只使用取消和异步等待, 不调用强制中止. `abort_handle`(强制中止句柄) 仅供 `004-2-real-shutdown-pipeline` 的 `ShutdownPipeline`(关闭流水线) 在关闭 `supervisor tree`(监督树) 时使用. 控制命令超时后通过 `stop_state = Failed`(停止失败) 标记失败, 不自动升级为强制中止.
- 控制命令的停止等待窗口来自当前 supervisor runtime(监督器运行时) 已生效的 `ShutdownPolicy.graceful_timeout(关闭策略优雅等待时间)`. `stop_deadline_at_unix_nanos(停止截止时间)` 必须等于取消送达时刻加该等待窗口. 本功能不新增单独的控制命令等待窗口配置, 并且仍然忽略 `abort_after_timeout(超时后强制中止)` 策略标志.
- 停止失败不是初次控制命令同步等待 child future(子任务 future) 的结果. control loop(控制循环) 在后续命令, `CurrentState(当前状态)` 或 child exit(子任务退出) 收尾处理前调用 `reconcile_stop_deadlines(调和停止截止时间)`, 当停止截止时间已经经过且 child(子任务) 仍未退出时, 才把运行状态记录推进到 `Failed(停止失败)` 并发布失败事件. 本规格采用 lazy-only(惰性触发) 语义, 不新增 timer(定时器) 或内部唤醒消息, 因此没有后续控制命令, `CurrentState(当前状态)` 或 child exit(子任务退出) 时, 失败事件不会单独按时钟自动发布.
- 功能目录名为 `004-3-child-runtime-state-control`, 与功能分支名 `004-runtime-semantics` 一对多: 同分支上并列 `004-1`, `004-2`, `004-4` 等其他运行时语义切片.
- ManagedChildState(受管子任务状态) 可以继续作为对外简化状态展示, 但不再是唯一事实来源, 运行状态字段才是真实事实.
- 本规格不要求新增动态子任务声明格式, 也不改变 supervision strategy(监督策略) 的重启决策算法, 只把策略窗口内已使用次数和剩余次数暴露到运行状态记录中.
- restart_limit(重启次数限制) 的窗口和上限来自既有 `RestartLimit(重启次数限制)` 配置来源, 优先级依次为 child strategy override(子任务策略覆盖), group strategy(分组策略), supervisor spec(监督器声明) 和配置层默认 `PolicyConfig.child_restart_limit / child_restart_window_ms(策略配置子任务重启上限与窗口)`. 已使用次数和剩余次数由 runtime(运行时) 侧重启次数限制跟踪结构维护. 当前 `PolicyEngine(策略引擎)` 是无状态结构, `RestartPolicy(重启策略)` 不提供 `used / remaining(已使用与剩余)` 运行时历史字段, 运行状态记录只负责暴露 runtime(运行时) 写入的当前剩余次数状态.
