# Contract(契约): 子任务运行状态控制

本契约描述 `PauseChild(暂停子任务)`, `RemoveChild(移除子任务)`, `QuarantineChild(隔离子任务)` 三条停止类控制命令在 `ChildRuntimeState(子任务运行状态记录)` 上的可观察结果. 本契约不是网络协议, 它是 Rust public API(Rust 公开接口) 和 runtime diagnostics(运行时诊断) 的边界约定. `RestartChild(重启子任务)` 与 `ResumeChild(恢复子任务)` 是既有命令, 本规格只要求它们不破坏运行状态事实, 不在本契约中重新定义其生命周期语义. 关闭 supervisor tree(监督树) 的语义由 `004-2-real-shutdown-pipeline/contracts/shutdown-pipeline.md` 单独承担, 本契约不重定义关闭路径.

## Public API(公开接口)

### `ControlCommand(控制命令)` 变体

本功能不新增 `ControlCommand(控制命令)` 变体. 现有变体的字段形状保持不变, 但 runtime(运行时) 处理逻辑必须满足本契约规则.

```rust
ControlCommand::RemoveChild { meta: CommandMeta, child_id: ChildId }
ControlCommand::PauseChild { meta: CommandMeta, child_id: ChildId }
ControlCommand::QuarantineChild { meta: CommandMeta, child_id: ChildId }
ControlCommand::CurrentState { meta: CommandMeta }
```

### `CommandResult::ChildControl(子任务控制命令结果)`

`CommandResult::ChildState(子任务状态命令结果)` 变体被替换为 `CommandResult::ChildControl(子任务控制命令结果)`. 项目禁止 compatibility export(兼容导出), 旧变体名称必须删除, 不通过类型别名重导出.

```rust
pub enum CommandResult {
    ChildAdded { child_manifest: String },
    ChildControl { outcome: ChildControlResult },
    CurrentState { state: CurrentState },
    Shutdown { result: ShutdownResult },
}
```

Contract rules(契约规则):

- `RemoveChild(移除子任务)`, `PauseChild(暂停子任务)`, `QuarantineChild(隔离子任务)` 必须返回 `ChildControl(子任务控制命令结果)` 变体, 不得返回其他变体.
- `AddChild(添加子任务)` 仍返回 `ChildAdded(子任务已添加)`, 不在本功能范围.
- `CurrentState(当前状态)` 必须包含完整的运行状态记录, 详见下文 `CurrentState(当前状态)` 契约.

### `ChildControlResult(子任务控制结果)`

`ChildControlResult(子任务控制结果)` 必须由 `src/control/outcome.rs` 拥有. `ChildAttemptStatus(子任务尝试状态)`, `ChildControlOperation(子任务控制操作)`, `ChildStopState(子任务停止状态)` 和 `ChildControlFailurePhase(子任务控制失败阶段)` 也必须由 `src/control/outcome.rs` 拥有, 以避免 control(控制) 模块反向依赖 runtime(运行时) 模块.

```rust
pub struct ChildControlResult {
    pub child_id: ChildId,
    pub attempt: Option<Attempt>,
    pub generation: Option<Generation>,
    pub operation_before: ChildControlOperation,
    pub operation_after: ChildControlOperation,
    pub status: Option<ChildAttemptStatus>,
    pub cancel_delivered: bool,
    pub stop_state: ChildStopState,
    pub restart_limit: RestartLimitState,
    pub liveness: ChildLivenessState,
    pub idempotent: bool,
    pub failure: Option<ChildControlFailure>,
}
```

Contract rules(契约规则):

- `attempt(尝试)` 与 `generation(代次)` 必须指向命令到达时运行状态记录上活动尝试的标识. 命令到达后即使触发自动重启, 本字段必须记录命令实际作用的 attempt(尝试) 标识, 不得替换为新 attempt(尝试).
- `operation_before(命令前操作)` 必须反映命令到达时的操作, `operation_after(命令后操作)` 必须反映命令处理后的操作.
- `cancel_delivered(取消已送达)` 仅在本次命令真正向当前活动尝试发送了 `CancellationToken::cancel(取消)` 时为 `true(是)`. 复用已存在的取消状态时必须为 `false(否)`.
- `stop_state(停止状态)` 必须使用 `ChildStopState(子任务停止状态)` 取值, 不得使用字符串描述.
- `idempotent(幂等)` 为 `true(是)` 时, `operation_before(命令前操作)` 必须等于 `operation_after(命令后操作)`, 且 `cancel_delivered(取消已送达)` 必须为 `false(否)`.
- `restart_limit(重启次数限制)` 必须始终携带 `RestartLimitState(重启次数限制状态)`, 即使该运行状态记录 operation(操作) 是 `Quarantined(已隔离)`.
- `liveness(存活状态)` 必须由 control loop(控制循环) 在命令处理时基于 `heartbeat_receiver(心跳接收端)` 与 `readiness_receiver(就绪接收端)` 的最新值构造, 不得提前缓存. `readiness_receiver(就绪接收端)` 必须使用 `ReadinessState(就绪状态)` 区分 `Unreported(未上报)`, `Ready(已就绪)` 和 `NotReady(未就绪)`.
- 初次接受的停止命令通常只返回 `stop_state = CancelDelivered(已送达取消)` 与 `failure = None(无值)`, 不同步等待失败结果. 后续 `CurrentState(当前状态)` 或重复停止命令必须先运行 `reconcile_stop_deadlines(调和停止截止时间)`, 然后把已经超时的 `last_control_failure(最近控制失败原因)` 写入 `failure(失败原因)`.
- 当运行状态记录没有活动 attempt(尝试) 时, 结果必须返回 `attempt = None(无值)`, `generation = None(无值)`, `status = None(无值)`, `cancel_delivered = false(否)`, `stop_state = NoActiveAttempt(无活动尝试)`.
- 当运行状态记录没有活动 attempt(尝试) 时, `idempotent(幂等)` 仍必须按操作和删除动作判定. 如果本次命令改变 `operation(操作)` 或触发 `RemoveChild(移除子任务)` 的物理删除, `idempotent(幂等)` 必须为 `false(否)`.
- 当运行状态记录存在活动 attempt(尝试), 但 `operation_before(命令前操作)` 已经等于本次命令目标操作且 `attempt_cancel_delivered(尝试取消已送达)` 已经是 `true(是)` 时, 本次命令必须返回 `idempotent = true(幂等是)`, `cancel_delivered = false(否)`, 不得再次调用 `CancellationToken::cancel(取消)`, 不得再次发布 `ChildControlCancelDelivered(子任务控制取消已送达)` 事件.

## Operation Mapping(操作映射)

`ChildControlOperation(子任务控制操作)` 是公开结果枚举, 由 runtime(运行时) 维护具体字段值. `Operation(操作)` 表示控制面要求运行状态记录执行的生命周期操作, 它不是实际运行状态, 也不是 `PolicyEngine(策略引擎)` 的策略决策结果. `ManagedChildState(受管子任务状态)` 是对外简化视图. 两者一一对应, dashboard(仪表盘) 与 audit(审计) 输出中允许并存, 但 `ChildRuntimeState(子任务运行状态记录)` 字段始终是唯一事实来源.

| `ChildControlOperation(子任务控制操作)` | `ManagedChildState(受管子任务状态)` | 含义                                                                     |
| ----------------------------------------- | ----------------------------------- | ------------------------------------------------------------------------ |
| `Active(活跃)`                            | `Running(运行中)`                   | 运行状态记录正常运行, supervision strategy(监督策略) 可以触发自动重启.           |
| `Paused(已暂停)`                          | `Paused(已暂停)`                    | 运行状态记录被显式暂停, 自动重启暂停, 本规格不定义恢复语义. |
| `Quarantined(已隔离)`                     | `Quarantined(已隔离)`               | 运行状态记录被隔离, 自动重启被阻止, 即使 attempt(尝试) 退出也不重启; 操作者仍可继续执行 `RemoveChild(移除子任务)`. |
| `Removed(已移除)`                         | `Removed(已移除)`                   | 运行状态记录待删除, 当前 attempt(尝试) 退出后从 `child_runtime_states(子任务运行状态记录集合)` 中物理移除; 无活动 attempt(尝试) 时在命令结果构造后同轮物理移除. |

Contract rules(契约规则):

- 任何对外字段 (例如 audit log(审计日志) 或 dashboard model(仪表盘模型)) 同时显示 `operation(操作)` 与 `ManagedChildState(受管子任务状态)` 时, 两者必须保持一致, 不得任一字段单独漂移.
- `Active(活跃)` 与 `Running(运行中)` 是同一操作, 命名差异仅为对外表达的简化; 实现端不得维护两份不同枚举值.

## Command Semantics(命令语义)

停止类命令真正向活动 attempt(尝试) 送达取消时, runtime(运行时) 必须在 `ChildRuntimeState(子任务运行状态记录)` 上记录 `stop_deadline_at_unix_nanos(停止截止时间)`. 该时间必须由取消送达时刻加当前 supervisor runtime(监督器运行时) 已生效的 `ShutdownPolicy.graceful_timeout(关闭策略优雅等待时间)` 计算得到. 本功能不新增单独的控制命令等待窗口配置, 也不读取 `abort_wait(强制中止等待)`. `reconcile_stop_deadlines(调和停止截止时间)` 是唯一负责把等待中的停止命令推进到 `Failed(停止失败)` 的路径.

### `PauseChild(暂停子任务)`

- runtime(运行时) 必须把 `ChildRuntimeState.operation(子任务控制操作)` 设为 `Paused(已暂停)`.
- 如果运行状态记录存在活动 attempt(尝试), 且 `operation_before(命令前操作)` 不是 `Paused(已暂停)` 或既有 `attempt_cancel_delivered(尝试取消已送达)` 为 `false(否)`, runtime(运行时) 必须调用一次 `ChildRuntimeState::cancel(运行状态记录取消)`, 把 `stop_state(停止状态)` 推进到 `CancelDelivered(已送达取消)`, 把 `status(状态)` 推进到 `Cancelling(取消中)`.
- 如果运行状态记录存在活动 attempt(尝试), 且 `operation_before(命令前操作)` 已经是 `Paused(已暂停)` 并且既有 `attempt_cancel_delivered(尝试取消已送达)` 为 `true(是)`, runtime(运行时) 必须按幂等返回处理, 不得重复取消.
- 如果运行状态记录没有活动 attempt(尝试), runtime(运行时) 必须把 `stop_state(停止状态)` 设为 `NoActiveAttempt(无活动尝试)`, `cancel_delivered(取消已送达)` 设为 `false(否)`. 如果 `operation_before(命令前操作)` 已经是 `Paused(已暂停)`, `idempotent(幂等)` 必须为 `true(是)`; 否则本次命令是操作变化, `idempotent(幂等)` 必须为 `false(否)`, 并且只发布 `ChildControlOperationChanged(子任务控制操作变化)` 事件.
- 暂停期间, supervision strategy(监督策略) 必须不能针对该运行状态记录触发自动重启.

### `RemoveChild(移除子任务)`

- runtime(运行时) 必须把 `operation(操作)` 设为 `Removed(已移除)`.
- 如果运行状态记录存在活动 attempt(尝试), 且 `operation_before(命令前操作)` 不是 `Removed(已移除)` 或既有 `attempt_cancel_delivered(尝试取消已送达)` 为 `false(否)`, runtime(运行时) 必须调用一次 `ChildRuntimeState::cancel(运行状态记录取消)`, 推进 `stop_state(停止状态)` 与 `status(状态)`.
- 如果运行状态记录存在活动 attempt(尝试), 且 `operation_before(命令前操作)` 已经是 `Removed(已移除)` 并且既有 `attempt_cancel_delivered(尝试取消已送达)` 为 `true(是)`, runtime(运行时) 必须按幂等返回处理, 不得重复取消.
- 当前 attempt(尝试) 退出后, exit handler(退出处理) 必须从 `RuntimeControlState.child_runtime_states` 中物理删除运行状态记录, 并发出 `ChildRuntimeStateRemoved(子任务运行状态记录已移除)` 事件.
- 如果运行状态记录没有活动 attempt(尝试), runtime(运行时) 必须构造 `stop_state = NoActiveAttempt(无活动尝试)`, `attempt = None(无值)`, `generation = None(无值)`, `status = None(无值)`, `cancel_delivered = false(否)` 的结果. 当 `operation_before(命令前操作)` 不是 `Removed(已移除)` 时, 该结果的 `idempotent(幂等)` 必须为 `false(否)`, 并且 runtime(运行时) 必须在结果构造和事件发布后从 `RuntimeControlState.child_runtime_states` 中物理删除运行状态记录. `ChildRuntimeStateRemoved(子任务运行状态记录已移除)` 事件的 `final_status(最终子任务尝试状态)` 必须为 `None(无值)`.
- 同一运行状态记录在仍存在且 `operation(操作)` 已经是 `Removed(已移除)` 时重复 `RemoveChild(移除子任务)` 必须返回 `idempotent = true(幂等是)`, 不得重复发送取消或操作变化事件. 运行状态记录已经物理删除后的再次命令使用既有 unknown child(未知子任务) 处理路径, 不属于本契约的运行状态记录级幂等返回.

### `QuarantineChild(隔离子任务)`

- runtime(运行时) 必须把 `operation(操作)` 设为 `Quarantined(已隔离)`.
- 如果运行状态记录存在活动 attempt(尝试), 且 `operation_before(命令前操作)` 不是 `Quarantined(已隔离)` 或既有 `attempt_cancel_delivered(尝试取消已送达)` 为 `false(否)`, runtime(运行时) 必须 `cancel(取消)` 该 attempt(尝试). 运行状态记录继续存在于 `RuntimeControlState.child_runtime_states` 中, 但 supervision strategy(监督策略) 必须不再触发自动重启.
- 如果运行状态记录存在活动 attempt(尝试), 且 `operation_before(命令前操作)` 已经是 `Quarantined(已隔离)` 并且既有 `attempt_cancel_delivered(尝试取消已送达)` 为 `true(是)`, runtime(运行时) 必须按幂等返回处理, 不得重复取消.
- 如果运行状态记录没有活动 attempt(尝试), runtime(运行时) 必须返回 `stop_state = NoActiveAttempt(无活动尝试)` 且 `cancel_delivered = false(否)`. 当 `operation_before(命令前操作)` 已经是 `Quarantined(已隔离)` 时, `idempotent(幂等)` 必须为 `true(是)`; 否则本次命令是操作变化, `idempotent(幂等)` 必须为 `false(否)`.
- 同一运行状态记录重复 `QuarantineChild(隔离子任务)` 必须返回 `idempotent = true(幂等是)`.

### `CurrentState(当前状态)`

- runtime(运行时) 在构造 `CurrentState(当前状态)` 前必须先调用 `reconcile_stop_deadlines(调和停止截止时间)`, 以便长期忽略取消的 child(子任务) 能在只读状态读取中暴露停止失败.
- runtime(运行时) 必须返回包含 `child_runtime_records(子任务运行状态记录集合)` 的 `CurrentState(当前状态)`.
- 每个 `ChildRuntimeRecord(子任务运行状态记录)` 必须按声明顺序排列.
- 每个 `ChildRuntimeRecord(子任务运行状态记录)` 必须包含 `failure(失败原因)` 字段. 当 `stop_state = Failed(停止失败)` 时, `failure(失败原因)` 必须为 `Some(有值)`.
- 关闭流水线进行中或已经完成时, `CurrentState(当前状态)` 必须仍能返回最近一次运行状态记录.
- `CurrentState(当前状态)` 构造路径必须只执行非阻塞记录读取和线性排序, 不得等待 child future(子任务 future), 不得执行额外 I/O(输入输出). 代表性测试场景中连续 20 次构造调用结果时, 每次构造耗时都必须低于 1 毫秒.

## Shutdown vs Operation(关闭与操作)

`ShutdownPipeline(关闭流水线)` 与本功能的 `ChildRuntimeState.operation(子任务控制操作)` 字段在同一运行状态记录上可能并存. 关闭路径必须优先, 操作不得阻塞 supervisor tree(监督树) 级别的关闭.

Contract rules(契约规则):

- `ShutdownPipeline(关闭流水线)` 在 `RequestStop(请求停止)` 阶段必须对 `child_runtime_states(子任务运行状态记录集合)` 中**全部**运行状态记录发起 `cancel(取消)`, 不论该运行状态记录 `operation(操作)` 是 `Active(活跃)`, `Paused(已暂停)` 还是 `Quarantined(已隔离)`. `CancellationToken::cancel(取消令牌取消)` 对已经取消的 token 是 no-op, 因此对 `Paused(已暂停)` 或 `Quarantined(已隔离)` 运行状态记录重复调用不会产生副作用.
- `operation = Removed(已移除)` 的运行状态记录在 `RequestStop(请求停止)` 阶段已经从 `child_runtime_states` 中被 exit handler(退出处理) 物理删除时, `ShutdownPipeline(关闭流水线)` 必须跳过该 child(子任务) 的 `cancel(取消)` 路径, 同时把该 child(子任务) 在 `ShutdownPipelineReport.outcomes(关闭流水线报告结果集合)` 中标记为 `AlreadyExited(已经退出)`, 与 `004-2-real-shutdown-pipeline/contracts/shutdown-pipeline.md` 中没有运行中任务时的处理保持一致.
- 关闭期间, `operation(操作)` 字段保持不变; 关闭完成后, `Removed(已移除)` 与 `Quarantined(已隔离)` 运行状态记录不得被本功能的任何控制命令重新激活. 恢复路径不在本规格范围内.
- 关闭路径产生的 `ChildShutdownOutcome(子任务关闭结果)` 与本功能产生的 `ChildControlResult(子任务控制结果)` 是两套独立报告: 关闭路径的 outcome 反映 supervisor tree(监督树) 关闭事实, 控制路径的 outcome 反映单条控制命令事实. 实现端必须区分两套 outcome 的归属, 不得共用同一 outcome 对象.

## Runtime Internal Message(运行时内部消息)

本功能不新增 `RuntimeLoopMessage(运行时循环消息)` 变体. 子任务退出仍通过 `RuntimeLoopMessage::ChildInstance(子任务尝试消息)` 进入 control loop(控制循环).

Runtime contract(运行时契约):

- control loop(控制循环) 每次处理 `ControlCommand(控制命令)`, `CurrentState(当前状态)` 或 child exit(子任务退出) 收尾前, 必须调用 `reconcile_stop_deadlines(调和停止截止时间)`. 该函数遍历等待中的运行状态记录, 当 `stop_deadline_at_unix_nanos(停止截止时间)` 已经过期且 `ChildAttemptMessage::Exited(子任务退出消息)` 尚未到达时, 把 `stop_state(停止状态)` 推进到 `Failed(停止失败)`, 写入 `last_control_failure(最近控制失败原因)`, 并发布 `ChildControlStopFailed(子任务控制停止失败)` 事件. 该函数不得调用 `runtime_state.abort(运行状态记录强制中止)`. 本契约采用 lazy-only(惰性触发) 语义, 不新增 timer(定时器) 或内部唤醒消息; 若没有后续控制命令, `CurrentState(当前状态)` 或 child exit(子任务退出), 停止失败事件不会单独按时钟自动发布.
- `ChildAttemptMessage::Exited(子任务退出消息)` 到达时, exit handler(退出处理) 必须读取运行状态记录 `operation(操作)`. `Active(活跃)` 允许执行常规策略评估, 并允许 runtime(运行时) 侧重启次数限制跟踪器更新 `restart_limit(重启次数限制)` 状态. `Paused(已暂停)` 必须只保存最近一次 `restart_limit(重启次数限制)` 状态, 不启动新 attempt(尝试), 也不得记录新的重启尝试. `Quarantined(已隔离)` 必须既不评估策略也不启动新 attempt(尝试). `Removed(已移除)` 必须从 `child_runtime_states(子任务运行状态记录集合)` 中删除运行状态记录.
- 控制命令路径不得直接调用 `wait_for_report(等待报告)`. 等待 child future(子任务 future) 终止仍由 spawn 时挂上的观察任务通过 `RuntimeLoopMessage::ChildInstance(子任务尝试消息)` 完成.

## Observability Emission Path(可观测事件发送路径)

控制命令事件必须从 control loop(控制循环) 进入 `ObservabilityPipeline(可观测流水线)`. `src/event/payload.rs` 只定义 typed payload(类型化载荷), 不能单独视为已经发布事件.

Contract rules(契约规则):

- control loop(控制循环) 必须为 `ChildControlCancelDelivered(子任务控制取消已送达)`, `ChildControlStopCompleted(子任务控制停止完成)`, `ChildControlStopFailed(子任务控制停止失败)`, `ChildControlOperationChanged(子任务控制操作变化)`, `ChildRuntimeStateRemoved(子任务运行状态记录已移除)` 和 `ChildHeartbeatStale(子任务运行状态记录心跳陈旧)` 构造 `SupervisorEvent(监督器事件)`.
- 每个 `SupervisorEvent(监督器事件)` 必须包含 `EventSequence(事件序列)`, `CorrelationId(关联标识)`, `Where(位置)` 和 `What(事件载荷)`. `Where(位置)` 必须至少包含 supervisor path(监督器路径) 与相关 `child_id(子任务标识)`.
- control loop(控制循环) 必须调用 `ObservabilityPipeline::emit(可观测流水线发送)` 或等价的 typed event sink(类型化事件发送边界). 仅向 `broadcast::Sender<String>(广播字符串发送器)` 发送文本不得满足本契约.
- metrics(指标), audit(审计), tracing(追踪), journal(事件日志) 和 test recorder(测试记录器) 必须从同一个 `SupervisorEvent(监督器事件)` 派生, 不得由各任务分别手写不一致的事实.

## Race Determinism Contract(竞态可复现契约)

控制命令与自动重启竞态测试必须使用 `research.md` 决策九指定的测试夹具门控. 测试 child(子任务) 在准备返回失败前必须先通知测试代码, 然后等待释放信号. 测试代码必须先发送 `PauseChild(暂停子任务)` 并确认操作已经写入 `Paused(已暂停)`, 再释放 child(子任务) 返回失败. 退出消息到达后, exit handler(退出处理) 必须读取该操作并跳过自动重启.

Contract rules(契约规则):

- 本功能的竞态测试默认不使用 `tokio::time::pause(暂停时间)` 作为调度控制策略.
- 本功能不得为了该竞态测试在 `src/runtime/control_loop.rs` 增加仅测试可见的生产代码钩子.
- 可复现性必须来自测试 child(子任务) 的协作式退出门控, 而不是来自生产 control loop(控制循环) 的特殊分支.

## Event Contract(事件契约)

`src/event/payload.rs` 必须新增下列事件, 与 `004-2-real-shutdown-pipeline` 的事件风格保持一致.

| Event(事件)                                          | Required fields(必需字段, 包含类型注释)                       | Purpose(目的)                                                                                                                                                                                                                                      |
| ---------------------------------------------------- | ------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ChildControlCancelDelivered(子任务控制取消已送达)`  | `child_id: ChildId(子任务标识)`, `generation: Generation(代次)`, `attempt: Attempt(尝试)`, `command: String(命令名)`, `command_id: String(命令标识)` | 控制命令真正向活动尝试发送取消时发布.                                                                                                                                                                                                              |
| `ChildControlStopCompleted(子任务控制停止完成)`      | `child_id: ChildId(子任务标识)`, `generation: Generation(代次)`, `attempt: Attempt(尝试)`, `exit_kind: ExitKind(退出分类)` | exit handler(退出处理) 在收到 `Exited(已退出)` 后, 运行状态记录 `stop_state(停止状态)` 推进到 `Completed(已停止)` 时发布.                                                                                                                                  |
| `ChildControlStopFailed(子任务控制停止失败)`         | `child_id: ChildId(子任务标识)`, `generation: Generation(代次)`, `attempt: Attempt(尝试)`, `status: ChildAttemptStatus(子任务尝试状态)`, `stop_state: ChildStopState(子任务停止状态)`, `phase: ChildControlFailurePhase(子任务控制失败阶段)`, `reason: String(原因)`, `recoverable: bool(可恢复)` | `reconcile_stop_deadlines(调和停止截止时间)` 发现停止截止时间已经经过且 child(子任务) 仍未退出时发布. |
| `ChildControlOperationChanged(子任务控制操作变化)` | `child_id: ChildId(子任务标识)`, `from: ChildControlOperation(操作原值)`, `to: ChildControlOperation(操作新值)`, `command: String(命令名)`, `command_id: String(命令标识)` | 运行状态记录 `operation(操作)` 字段变化时发布.                                                                                                                                                                                                        |
| `ChildRuntimeStateRemoved(子任务运行状态记录已移除)`                 | `child_id: ChildId(子任务标识)`, `path: SupervisorPath(路径)`, `final_status: Option<ChildAttemptStatus>(可选最终子任务尝试状态)` | 运行状态记录从 `child_runtime_states(子任务运行状态记录集合)` 中物理删除时发布. 有活动 attempt(尝试) 退出后删除时为 `Some(有值)`, 无活动 attempt(尝试) 的占位运行状态记录被删除时为 `None(无值)`. |
| `ChildHeartbeatStale(子任务运行状态记录心跳陈旧)`        | `child_id: ChildId(子任务标识)`, `attempt: Attempt(尝试)`, `since_unix_nanos: u128(最后心跳纳秒时间戳)` | control loop(控制循环) 在 `CurrentState(当前状态)` 处理过程中识别到心跳陈旧时发布. `since_unix_nanos` 是最后收到心跳的时间戳(即 `last_observed_heartbeat_at_unix_nanos`(最后观察心跳纳秒时间戳) 的副本), 不是陈旧持续时长. 抑制规则: 同一 `(child_id, attempt)` 对在该 `attempt(尝试)` 终止前最多发布一次; 后续若发生 `attempt(尝试)` 切换或 child(子任务) 恢复正常心跳, 抑制计数随之重置. |

Contract rules(契约规则):

- `command(命令)` 字段必须取自 `command_name(命令名)` 的稳定字符串, 例如 `pause_child`, `remove_child`.
- 事件必须使用 `ChildId(子任务标识)`, `Generation(代次)`, `Attempt(尝试)`, `ChildAttemptStatus(子任务尝试状态)`, `ChildStopState(子任务停止状态)` 和 `ChildControlFailurePhase(子任务控制失败阶段)` 等类型化字段, 不得使用未类型化字符串表达这些状态.
- 本功能的控制命令路径只允许 `ChildControlFailurePhase::WaitCompletion(子任务控制失败阶段为等待完成)` 出现在 `ChildControlStopFailed(子任务控制停止失败)` 事件和 `ChildControlResult.failure(子任务控制结果失败原因)` 中.
- 重复幂等返回不得发出 `ChildControlCancelDelivered(子任务控制取消已送达)` 或 `ChildControlOperationChanged(子任务控制操作变化)` 事件.

## Metrics Contract(指标契约)

`src/observe/metrics.rs` 必须暴露 low-cardinality(低基数) 指标. `child_id(子任务标识)` 在高基数集群中禁止作为 metrics(指标) 标签, 但本仓库目前 child(子任务) 数量受 supervisor tree(监督树) 声明约束, 当前阶段允许在 gauge(仪表) 中使用 `child_id(子任务标识)` 标签.

| Metric(指标)                                                                     | Type(类型)      | Labels(标签)        | Rule(规则)                                                                      |
| -------------------------------------------------------------------------------- | --------------- | ------------------- | ------------------------------------------------------------------------------- |
| `supervisor_child_control_command_total(子任务控制命令总数)`                     | counter(计数器) | `command`, `result` | 每条控制命令处理后增加 1, `result(结果)` 取 `accepted`, `idempotent`, `failed`. |
| `supervisor_child_runtime_restart_limit_remaining(子任务运行状态记录剩余重启次数)`         | gauge(仪表)     | `child_id`          | runtime(运行时) 侧重启次数限制跟踪器刷新 `RestartLimitState(重启次数限制状态)` 后写入当前 `remaining(剩余)` 值. |
| `supervisor_child_runtime_heartbeat_stale_total(子任务运行状态记录心跳陈旧总数)`            | counter(计数器) | `none(无)`          | 仅在抑制规则允许并实际发布 `ChildHeartbeatStale(子任务运行状态记录心跳陈旧)` 事件时增加 1. `child_id(子任务标识)` 只出现在事件和 audit log(审计日志), 不作为该 counter(计数器) 的标签. |
| `supervisor_child_runtime_operation_transitions_total(子任务控制操作转换总数)` | counter(计数器) | `from`, `to`        | 每次 `operation(操作)` 变化增加 1.                                         |

`result(结果)` 必须按下列规则映射:

- `accepted(已接受)`: 本次命令返回成功, `ChildControlResult.idempotent(子任务控制结果幂等)` 为 `false(否)`, 且 `ChildControlResult.failure(子任务控制结果失败原因)` 为 `None(无值)`.
- `idempotent(幂等)`: 本次命令返回成功, 且 `ChildControlResult.idempotent(子任务控制结果幂等)` 为 `true(是)`.
- `failed(失败)`: 命令处理返回错误, 或 `ChildControlResult.failure(子任务控制结果失败原因)` 为 `Some(有值)`, 或 `ChildControlResult.stop_state(子任务控制结果停止状态)` 为 `Failed(停止失败)`.

`result(结果)` 必须避免使用 `reason(原因)` 作为指标标签. `reason(原因)` 仅出现在事件和 audit log(审计日志).

## Audit Contract(审计契约)

`src/observe/pipeline.rs` 或相邻观测边界必须记录下列 audit(审计) 事实:

- 每条控制命令到达时间, `command_id(命令标识)`, `requested_by(请求者)`, `reason(原因)`, 目标 `child_id(子任务标识)`, 目标 `generation(代次)`, 目标 `attempt(尝试)` 和命令到达时 `status(状态)`.
- 每条控制命令处理结果: `operation_before(命令前操作)`, `operation_after(命令后操作)`, `cancel_delivered(取消已送达)`, `stop_state(停止状态)`, `restart_limit_remaining(剩余重启次数)`, `idempotent(幂等)`, `failure(失败原因)`.
- exit handler(退出处理) 把 `stop_state(停止状态)` 从 `CancelDelivered(已送达取消)` 推进到 `Completed(已停止)` 时, 或 `reconcile_stop_deadlines(调和停止截止时间)` 把停止状态推进到 `Failed(停止失败)` 时.
- `ChildRuntimeStateRemoved(子任务运行状态记录已移除)` 事件发布时记录最终操作和最终 exit(退出) 分类.

## Dashboard Contract(仪表盘契约)

dashboard protocol(仪表盘协议) 的控制命令请求字段不改变, `dashboard_protocol_shape_test` 必须继续证明 `PauseChild(暂停子任务)`, `RemoveChild(移除子任务)`, `QuarantineChild(隔离子任务)` 的请求字段没有漂移. 返回结果形状会按本契约有意变化: `CommandResult(命令结果)` 把旧 `ChildState(子任务状态)` 调用结果替换为 `ChildControl(子任务控制)` 调用结果, `CurrentState(当前状态)` 调用结果新增 `child_runtime_records(子任务运行状态记录集合)`.

dashboard(仪表盘) 必须通过更新后的 `CommandResult::ChildControl(子任务控制命令结果)`, 扩展后的 `CurrentState(当前状态)` 和 event stream(事件流) 观察:

- 当前每个运行状态记录的 `operation(操作)`, `status(状态)` 与最后心跳.
- `restart_limit(重启次数限制)` 剩余次数及是否耗尽.
- 控制命令的 `idempotent(幂等)` 标志和失败原因.

本功能不要求新增 dashboard route(仪表盘路由), 但必须同步 dashboard(仪表盘) 返回结果模型, 避免仍按旧 `ChildState(子任务状态)` 或旧 `CurrentState(当前状态)` 字段解析返回结果.

## Compatibility(兼容性)

本功能禁止 compatibility exports(兼容导出). `CommandResult::ChildState(子任务状态命令结果)` 旧变体必须直接删除. 调用者必须使用新的 `ChildControlResult(子任务控制结果)` 类型. `CurrentState(当前状态)` 字段升级必须直接修改原结构, 不通过包装类型继续提供旧形状.
