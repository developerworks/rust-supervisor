# Research(研究结论): 子任务运行状态控制

## 决策一: `ChildRuntimeState(子任务运行状态记录)` 归属 `src/runtime/child_runtime_state.rs`

**Decision(决定)**: 新增 `src/runtime/child_runtime_state.rs`, 拥有 `ChildRuntimeState(子任务运行状态记录)` 类型. 该类型取代 `src/runtime/shutdown_pipeline.rs` 中现有 `ActiveChildAttempt(活动子任务尝试)`, 由 `RuntimeControlState(运行时控制状态)` 通过 `HashMap<ChildId, ChildRuntimeState>` 直接持有. `src/runtime/shutdown_pipeline.rs` 改为复用 `ChildRuntimeState(子任务运行状态记录)`, 不再独立维护一份活动尝试结构. registry(注册表) 继续保存 `ChildRuntime(子任务运行时记录)` 作为声明性事实和退出历史的源.

**Rationale(理由)**: 运行状态记录的关键字段包括 `CancellationToken(取消令牌)`, `AbortHandle(强制中止句柄)`, `completion_receiver(完成接收端)`, `watch::Receiver(观察接收端)`, 这些资源属于 runtime(运行时) 边界. registry(注册表) 模块在 `001-create-supervisor-core` 中被定义为 "声明性事实容器", 它的字段被多个模块以 `Clone(克隆)` 方式复制. 把运行时句柄塞入 registry(注册表) 会让所有持有者承担 cancel(取消) 与 abort(强制中止) 的所有权, 破坏现有边界. 把运行状态记录放在 runtime(运行时) 模块, 也与 `004-2-real-shutdown-pipeline` 把 `ActiveChildAttempt(活动子任务尝试)` 放在 runtime(运行时) 的决策一致.

**Alternatives considered(备选方案)**:

- 把 `ChildRuntimeState(子任务运行状态记录)` 放入 `src/registry/child_runtime_state.rs`, 让 registry(注册表) 同时拥有声明事实和运行时句柄. 该方案让 registry(注册表) 失去 `Clone(克隆)` 特性, 且会让 shutdown(关闭) 模块需要反向访问 registry(注册表) 才能拿到运行时句柄.
- 把 `ChildRuntimeState(子任务运行状态记录)` 拆为两半, 声明字段留在 registry(注册表), 运行时字段留在 runtime(运行时). 该方案制造两个真相, 不符合 spec.md FR-001 "运行状态记录必须真实表达声明, 代次, 当前尝试, 状态, 取消令牌, runtime_handle(运行时句柄), 最后心跳, readiness(就绪状态) 和 restart_limit(重启次数限制)" 的单一来源约束, 其中 `runtime_handle(运行时句柄)` 的字段映射见 `data-model.md` Field Mapping(字段映射) 表.

## 决策二: `RestartLimit(重启次数限制)` 由 runtime(运行时) 侧跟踪并暴露状态记录

**Decision(决定)**: 新增 `RestartLimitState(重启次数限制状态)` 公开类型, 放在 `src/control/outcome.rs`. 运行状态记录包含一个 `RestartLimitState(重启次数限制状态)` 字段, 由 runtime(运行时) 侧的 `RestartLimitTracker(重启次数限制跟踪器)` 或等价结构刷新. state(状态) 字段包括 `window(策略窗口)`, `limit(上限)`, `used(已使用)`, `remaining(剩余)` 和 `exhausted(已耗尽)`. `window(窗口)` 和 `limit(上限)` 来自既有 `RestartLimit(重启次数限制)` 配置来源, 优先级依次为 child strategy override(子任务策略覆盖), group strategy(分组策略), supervisor spec(监督器声明) 和配置层默认 `PolicyConfig.child_restart_limit / child_restart_window_ms(策略配置子任务重启上限与窗口)`. `src/policy/decision.rs` 中的 `PolicyEngine(策略引擎)` 是无状态结构, `RestartPolicy(重启策略)` 只有 `Permanent(永久)`, `Transient(临时故障重启)` 和 `Temporary(一次性)` 三个枚举值, 不含 `used / remaining(已使用与剩余)` 运行时历史字段, 因此实现不得从 `PolicyEngine(策略引擎)` 或 `RestartPolicy(重启策略)` 中直接读取重启次数历史.

**Rationale(理由)**: 当前源码已经存在 `src/spec/supervisor.rs` 的 `RestartLimit(重启次数限制)` 以及 `src/config/configurable.rs` 的 `PolicyConfig.child_restart_limit / child_restart_window_ms(策略配置子任务重启上限与窗口)`. 这些来源提供窗口和上限, 但 `PolicyEngine(策略引擎)` 不持有运行时历史. 所以本功能需要在 runtime(运行时) 边界新增最小重启次数限制跟踪状态: `PolicyEngine(策略引擎)` 仍只负责把一次 task exit(任务退出) 和 `RestartPolicy(重启策略)` 转成 `RestartDecision(重启决策)`, runtime(运行时) 则在 `handle_child_exit(处理子任务退出)` 周围维护每个 child(子任务) 的窗口, 已使用次数, 剩余次数和耗尽标志, 并写入 `ChildRuntimeState.restart_limit(子任务运行状态记录重启次数限制)`.

**Alternatives considered(备选方案)**:

- 在 `ChildRuntime(子任务运行时记录)` 上保存剩余次数并由 registry(注册表) 暴露. 该方案让 registry(注册表) 字段语义变重, 还会让控制结果中的剩余次数与 runtime(运行时) 实际 attempt(尝试) 历史分离.
- 在 control 命令路径上现场计算剩余次数. 该方案让只读控制命令承担历史统计职责, 并且会让重复读取 `CurrentState(当前状态)` 改变或重算重启次数限制语义.
- 把重启次数限制字段加入 `PolicyEngine(策略引擎)` 或 `RestartPolicy(重启策略)`. 该方案会把当前无状态策略引擎改成有状态对象, 扩大变更面, 不符合 Small Increment(小增量) 原则.

## 决策三: 控制命令路径不阻塞等待 `child future(子任务 future)` 终止

**Decision(决定)**: `PauseChild(暂停子任务)`, `RemoveChild(移除子任务)`, `QuarantineChild(隔离子任务)` 在 control loop(控制循环) 单跳内完成"按幂等规则发起取消, 更新 operation(操作) 状态, 返回 `ChildControlResult(子任务控制结果)`". 真实退出仍通过 `ChildAttemptMessage::Exited(子任务退出消息)` 回到 control loop(控制循环), 由 exit handler(退出处理) 推进 `stop_state(停止状态)` 字段并发出 `ChildControlStopCompleted(子任务控制停止完成)` 事件. 停止失败由 `reconcile_stop_deadlines(调和停止截止时间)` 在后续 control loop(控制循环) 轮次推进. 该函数在处理控制命令, `CurrentState(当前状态)` 和 child exit(子任务退出) 收尾前运行, 当运行状态记录已经超过停止截止时间且 child(子任务) 仍未退出时, 它把 `stop_state(停止状态)` 推进为 `Failed(停止失败)`, 保存 `ChildControlFailure(子任务控制失败原因)`, 并发出 `ChildControlStopFailed(子任务控制停止失败)` 事件. `stop_deadline_at_unix_nanos(停止截止时间)` 使用控制命令取消送达时刻加当前 supervisor runtime(监督器运行时) 已生效的 `ShutdownPolicy.graceful_timeout(关闭策略优雅等待时间)` 计算. 本决策采用 lazy-only(惰性触发) 语义, 不新增 timer(定时器) 或内部唤醒消息. 控制结果中 `stop_state(停止状态)` 字段分为 `NoActiveAttempt(无活动尝试)`, `CancelDelivered(已送达取消)`, `Completed(已停止)`, `Failed(停止失败)` 四值, 第一次返回时通常为 `CancelDelivered(已送达取消)`.

**Rationale(理由)**: control loop(控制循环) 是单线程消息循环, 单条控制命令处理时间影响其他命令的延迟. spec.md Edge Case(边界情况) 明确控制命令与自动重启同刻发生时子任务控制操作优先, 这意味着控制命令本身不应阻塞太久. spec.md SC-003 要求"对已停止或从未启动的运行状态记录重复执行停止类命令 10 次, 每次幂等返回", 也意味着停止类命令的语义可以是"幂等触发", 不要求每次都同步等待终止.

**Scope boundary(范围边界)**: `ChildRuntimeState.abort`(强制中止) 仅在 `ShutdownPipeline`(关闭流水线)(`004-2-real-shutdown-pipeline`) 关闭 `supervisor tree`(监督树) 时调用. 本功能范围内的控制命令 (`PauseChild`/`RemoveChild`/`QuarantineChild`) 只使用 `runtime_state.cancel`(软取消), 不调用 `runtime_state.abort`(强制中止). 控制命令超时后通过 `stop_state = Failed`(停止失败) 标记失败, 不自动升级为强制中止. 理由: 强制中止属于 supervision tree(监督树) 级别的安全措施, 单条控制命令不应跳过树的关闭协调.

**Clarification(澄清)**: `abort_after_timeout(超时后强制中止)` 策略标志是 `ShutdownPipeline` 的配置, 控制命令路径忽略该标志, 始终只使用 `runtime_state.cancel()`(软取消) 和超时标记 `Failed`(停止失败). 控制命令路径的等待窗口只读取 `ShutdownPolicy.graceful_timeout(关闭策略优雅等待时间)`, 不读取 `abort_wait(强制中止等待)` 且不新增控制命令专用配置. 控制命令路径的 `ChildControlFailure.phase(子任务控制失败阶段)` 只能使用 `ChildControlFailurePhase::WaitCompletion(等待完成)`, 因为本功能不进入 `abort_wait(强制中止等待)` 阶段. 初次停止命令不得为了等待失败结果阻塞 control loop(控制循环), 后续 `CurrentState(当前状态)` 或重复停止命令会先运行 `reconcile_stop_deadlines(调和停止截止时间)` 并观察到失败.

**Alternatives considered(备选方案)**:

- 命令路径同步 `await` child future(子任务 future). 该方案在长时间运行任务上阻塞 control loop(控制循环), 让 `CurrentState(当前状态)` 和其他命令延迟. 还会让取消失败的任务卡住整个控制面.
- 引入独立的"停止等待"任务. 该方案新增异步单元, 不必要, 因为现有 `ChildAttemptMessage::Exited(子任务退出消息)` 路径已经能携带终止事实.

## 决策四: heartbeat(心跳) 与 readiness(就绪状态) 通过 `watch::Receiver(观察接收端)` 暴露

**Decision(决定)**: 修改 `src/readiness/signal.rs`, `src/task/context.rs` 和 `src/child_runner/runner.rs`, 让 readiness(就绪状态) 从 `watch::Receiver<bool>` 升级为 `watch::Receiver<ReadinessState>`. `ReadinessState(就绪状态)` 取值为 `Unreported(未上报)`, `Ready(已就绪)` 和 `NotReady(未就绪)`. `ChildRunHandle(子任务运行句柄)` 在原有 `cancellation_token(取消令牌)`, `abort_handle(强制中止句柄)`, `completion_receiver(完成接收端)` 之外, 新增 `heartbeat_receiver: watch::Receiver<Option<Instant>>` 和 `readiness_receiver: watch::Receiver<ReadinessState>`. `ChildRuntimeState(子任务运行状态记录)` 在构造时保存这两个 receiver, control loop(控制循环) 在处理 `CurrentState(当前状态)` 时通过 `Receiver::borrow(借用)` 读取最新值.

**Note(注)**: `abort_handle`(强制中止句柄) 的指向由 `spawn_once`(派生一次) 的现有 `AbortHandle` 保存逻辑保证, 本功能不改变其语义. `AbortHandle` 在 `spawn_once` 中指向 `ChildRunner`(子任务运行器) 内部真实 `child future`(子任务 future), 而非 `observer task`(观察任务). data-model.md 中 "`abort_handle` 必须指向真实 child future, 不能只指向上报任务" 的校验规则由现有 child runner 架构保证, 本功能仅将已存在的 `abort_handle` 通过 `ChildRunHandle` 传递给 `ChildRuntimeState`, 不改变其指向.

**Rationale(理由)**: `TaskContext(任务上下文)` 已经基于 `tokio::sync::watch(观察通道)` 发布 heartbeat 和 readiness. 现有 `ReadySignal::new(新建就绪信号)` 以 `false(否)` 初始化, 这会把"未上报"和"已上报但未就绪"混在一起. 把通道值改为 `ReadinessState(就绪状态)` 可以在不新增 channel(通道) 类型的前提下修复语义. `Receiver::borrow(借用)` 是非阻塞读取, 与 control loop(控制循环) 的单线程模型兼容.

**Alternatives considered(备选方案)**:

- 让 task(任务) 把 heartbeat 与 readiness 走 `RuntimeLoopMessage(运行时循环消息)` 通过 mpsc(多生产者单消费者) 上报. 该方案在高频心跳下消耗 control loop(控制循环) 时间, 还需要为每个心跳分配消息.
- 新增 `crossbeam_channel(无锁通道)` 之类的依赖. 与 Small Increment(小增量) 原则冲突.

## 决策五: `CommandResult(命令结果)` 替换 `ChildState(子任务状态)` 变体, 并删除兼容形状

**Decision(决定)**: 在 `src/control/command.rs` 中把 `CommandResult::ChildState(子任务状态命令结果)` 变体替换为 `CommandResult::ChildControl(子任务控制命令结果)`, 携带 `ChildControlResult(子任务控制结果)`. 直接删除 `ChildState(子任务状态)` 旧变体, 也不通过任何 type alias(类型别名) 把旧名称重导出. 调用者必须使用新的真实类型路径.

**Rationale(理由)**: 项目宪章 "Compatibility exports(兼容导出)" 明确禁止 placeholder adapter(占位适配器) 和兼容层. 旧 `ChildState(子任务状态)` 变体只携带 `state(状态)` 和 `idempotent(幂等)` 两个字段, 不能表达 spec.md FR-003 所要求的 `child_id(子任务标识)`, `attempt(尝试)`, `cancel_delivered(取消已送达)`, `stop_state(停止状态)`, `restart_limit(重启次数限制)` 剩余次数, 失败 phase(阶段) 和原因. 直接替换是最小且最清晰的变更.

**Alternatives considered(备选方案)**:

- 在 `ChildState(子任务状态)` 之外新增字段. 该方案让单个变体承担两种语义, 既要表达原本"操作变化", 又要表达"控制结果", 字段语义混乱.
- 使用旧 `ChildState(子任务状态)` 作为类型别名指向新结构. 项目禁止此类兼容导出.

## 决策六: 控制命令与自动重启的优先级由 `ChildControlOperation(子任务控制操作)` 决定

**Decision(决定)**: `ChildRuntimeState(子任务运行状态记录)` 新增 `operation: ChildControlOperation(子任务控制操作)` 字段, 取值集合 `Active(活跃)`, `Paused(已暂停)`, `Quarantined(已隔离)`, `Removed(已移除)`. `ChildControlOperation(子任务控制操作)` 和 `ChildAttemptStatus(子任务尝试状态)` 的公开定义都放在 `src/control/outcome.rs`, runtime(运行时) 模块只维护字段值和内部转换. 当 `PauseChild(暂停子任务)`, `QuarantineChild(隔离子任务)` 或 `RemoveChild(移除子任务)` 命中运行状态记录时, 控制命令先更新 operation 字段, 再按活动 attempt(尝试) 是否存在决定是否发起取消. exit handler(退出处理) 决定是否触发自动重启时, 必须先读 operation: `Active(活跃)` 允许常规策略评估, `Paused(已暂停)` 阻止自动重启且本规格不定义恢复路径, `Quarantined(已隔离)` 阻止任何自动重启但允许后续 `RemoveChild(移除子任务)`, `Removed(已移除)` 在活动 attempt(尝试) 退出后会从 `child_runtime_states(子任务运行状态记录集合)` 中清除运行状态记录. 如果 `RemoveChild(移除子任务)` 命中无活动 attempt(尝试) 的运行状态记录, runtime(运行时) 必须在结果构造后同轮删除运行状态记录.

**Rationale(理由)**: `ManagedChildState(受管子任务状态)` 在 `001-create-supervisor-core` 中已经定义为对外可见的简化状态. spec.md Assumption(假设) 明确该类型可继续作为对外简化状态展示. `ChildControlOperation(子任务控制操作)` 与 `ManagedChildState(受管子任务状态)` 一一对应, 作为运行状态记录的"操作"事实. 公开 enum(枚举) 归属 control outcome(控制结果) 边界, 可以避免 control(控制) 模块反向依赖 runtime(运行时) 模块. 这样控制命令路径只需要更新运行状态字段, 不需要访问 `RuntimeControlState.children`. exit handler(退出处理) 也只读运行状态记录, 不读旧 `children: HashMap<ChildId, ManagedChildState>` 字段, 旧字段被替换.

**Alternatives considered(备选方案)**:

- 沿用 `RuntimeControlState.children` 作为"操作"事实. 该方案让运行状态记录和 children 两份字段语义重叠, 维护两份同步状态.
- 用 boolean 标志(`paused: bool`, `quarantined: bool`) 表达操作. 该方案表达能力弱, 也不能直接序列化到 audit log(审计日志).

## 决策七: `heartbeat_timeout(心跳超时)` 阈值在本切片中使用默认 5 秒常量

**Decision(决定)**: `ChildRuntimeState::observe_liveness(观察存活)` 判断 `heartbeat_stale(心跳陈旧)` 时使用 `src/runtime/child_runtime_state.rs` 内部常量 `DEFAULT_HEARTBEAT_TIMEOUT_SECS = 5`. 本切片不新增 `SupervisorSpec.heartbeat_timeout(监督器声明心跳超时)` 字段, 也不修改配置映射. 阈值含义是 "从 `last_observed_heartbeat_at_unix_nanos(最后观察心跳时间)` 起经过该时长后, 仍未收到新心跳即视为陈旧".

**Rationale(理由)**: spec.md Edge Case(边界情况) 明确要求 "区分未收到心跳与心跳超时", 但未约束具体阈值. 当前 `SupervisorSpec(监督器声明)` 源码没有 `heartbeat_timeout(心跳超时)` 字段. 若在本切片新增配置字段, 需要同时修改配置 schema(模式), 映射, 文档和兼容数据, 会把运行状态记录控制切片扩大为配置切片. 5 秒默认值与 `004-1-runtime-lifecycle-guard` 中 watchdog(看门狗) 默认观察周期保持同量级, 足以支持本切片的可观察性目标.

**Alternatives considered(备选方案)**:

- 在本切片为 `SupervisorSpec(监督器声明)` 新增 `heartbeat_timeout(心跳超时)` 字段. 该方案需要扩大配置模型和 schema(模式) 变更面, 不符合 Small Increment(小增量) 原则. 如果后续需要可配置阈值, 应由单独配置切片处理.
- 让每个 child(子任务) 在 `TaskContext(任务上下文)` 上自报阈值. 该方案让 child 任务承担监督参数, 突破了 child 与 supervisor 的责任边界.

## 决策八: 测试覆盖必须包含合作运行状态记录和非合作运行状态记录

**Decision(决定)**: 新增外部测试 `src/tests/supervisor_child_runtime_state_control_test.rs`, 覆盖下列行为:

- 启动一个会上报 heartbeat 与 readiness 的 child(子任务), 验证 `CurrentState(当前状态)` 显示真实字段.
- 启动一个尚未上报心跳的 child(子任务), 验证 `last_heartbeat(最后心跳)` 为 `None(无值)`, 与"心跳超时"区分.
- `PauseChild(暂停子任务)` 真实向活动尝试发送取消并标记 `operation = Paused(已暂停)`.
- `RemoveChild(移除子任务)` 发起取消, 然后由 exit handler(退出处理) 删除运行状态记录.
- `QuarantineChild(隔离子任务)` 发起取消并阻止自动重启.
- 对已停止或从未启动的运行状态记录重复执行停止类命令 10 次, 控制结果幂等且不重复发取消.
- 自动重启已经推进到新 `attempt(尝试)` 时, 控制命令仅作用于运行状态记录中当前 attempt, 不跨 attempt 误送取消.
- `restart_limit(重启次数限制)` 耗尽时控制结果说明 `remaining = 0(剩余为零)` 且 `exhausted = true(已耗尽)`.
- 控制命令与自动重启同刻发生时, 运行状态记录 operation 优先于新的策略决策.

**Rationale(理由)**: spec.md SC-001 至 SC-004 要求 100% 测试场景覆盖. 只测试 control_loop 的 enum 状态不能证明真实取消, 必须通过 task factory(任务工厂) 观察 token(令牌) 是否被任务感知, 也必须通过事件流验证 cancel_delivered 与 stop_completed 事件序列.

**Alternatives considered(备选方案)**:

- 仅扩展 `supervisor_control_test`. 该方案让单个测试文件同时承担旧 ChildState 兼容测试和新 ChildControlResult 测试, 关注点不集中, 出现回归时定位困难.
- 把测试放在生产模块内联. 项目宪章 "禁止内联单元测试代码, 单元测试必须放到外部目录" 明确禁止.

## 决策九: 自动重启竞态测试默认使用测试夹具门控

**Decision(决定)**: `operation_wins_over_auto_restart_race_test(操作优先于自动重启竞态测试)` 默认使用测试夹具门控来保证可复现性. 测试 child(子任务) 在准备返回失败前, 先通过测试通道通知测试代码, 然后等待释放信号. 测试代码在释放信号前先发送 `PauseChild(暂停子任务)`, 确认控制命令写入 `operation = Paused(已暂停)` 后再释放 child(子任务) 退出. 这样 `ChildAttemptMessage::Exited(子任务退出消息)` 到达时, exit handler(退出处理) 必须读取已经写入的操作, 并跳过自动重启.

**Rationale(理由)**: 该方案不需要为 `tokio(异步运行时)` 开启额外 test-util(测试工具) feature(特性), 也不需要在 `src/runtime/control_loop.rs` 增加仅测试可见的生产代码钩子. 可复现性来自测试 child(子任务) 本身的协作式退出门控, 仍然验证 control loop(控制循环) 对真实退出消息的处理顺序.

**Alternatives considered(备选方案)**:

- 使用 `tokio::time::pause(暂停时间)` 控制调度. 该方案依赖 `tokio(异步运行时)` 的 test-util(测试工具) feature(特性), 当前计划不需要为此改变依赖特性.
- 在 control loop(控制循环) 中增加仅测试可见的钩子来推迟 `ChildAttemptMessage::Exited(子任务退出消息)`. 该方案会把测试路径写进生产模块, 与小增量和模块边界要求不匹配.
