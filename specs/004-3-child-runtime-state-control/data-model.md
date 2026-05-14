# Data Model(数据模型): 子任务运行状态控制

## Entity(实体): `ChildRuntimeState(子任务运行状态记录)`

`ChildRuntimeState(子任务运行状态记录)` 是 runtime(运行时) 为每个声明 `child(子任务)` 维护的声明事实和活动事实容器. 它取代 `004-2-real-shutdown-pipeline` 中 `ActiveChildAttempt(活动子任务尝试)`, 由 `RuntimeControlState(运行时控制状态)` 直接持有, 关联文件为 `src/runtime/child_runtime_state.rs`. 当 child(子任务) 已经声明但尚无活动 attempt(尝试) 时, 运行状态记录仍然存在, 但活动尝试标识和运行时句柄字段必须为 `None(无值)`.

### Field Mapping(字段映射)

spec.md FR-001 字面要求 ChildRuntimeState 携带 "声明, 代次, 尝试次数, 状态, 取消令牌, runtime_handle(运行时句柄), 最后心跳, readiness(就绪状态), 重启次数限制" 等字段. 在 runtime(运行时) 边界上, 这些抽象字段映射为下表中的具体字段, 顺序差异属于实现细节, 含义保持一致:

| spec.md FR-001 字段 | data-model.md ChildRuntimeState 字段                                                                            | 实现说明                                                                                                                                                                                                                                                                                                                                              |
| ------------------- | ------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 声明                | `child_id` + `path`                                                                                     | 运行状态记录通过 `child_id(子任务标识)` 关联 registry(注册表) 中的 `ChildRuntime(子任务运行时记录)`, 不重复持有 `ChildSpec(子任务声明)`, 避免双份事实.                                                                                                                                                                                                        |
| 代次                | `generation`                                                                                            | 有活动 attempt(尝试) 时为 `Some(有值)`, 无活动 attempt(尝试) 时为 `None(无值)`.                                                                                                                                                                                                                                                                       |
| 尝试次数            | `attempt`                                                                                               | 有活动 attempt(尝试) 时为 `Some(有值)`, 无活动 attempt(尝试) 时为 `None(无值)`.                                                                                                                                                                                                                                                                       |
| 状态                | `status` (`ChildAttemptStatus(子任务尝试状态)`) + `operation` (`ChildControlOperation(子任务控制操作)`) | `status(状态)` 有活动 attempt(尝试) 时为 `Some(有值)`, 无活动 attempt(尝试) 时为 `None(无值)`. `operation(操作)` 始终存在, 它表达控制面要求运行状态记录执行的生命周期操作.                                                                                                                              |
| 取消令牌            | `cancellation_token`                                                                                    | 有活动 attempt(尝试) 时与 `TaskContext(任务上下文)` 共享同一 `CancellationToken(取消令牌)` 克隆, 无活动 attempt(尝试) 时为 `None(无值)`.                                                                                                                                                                                                              |
| runtime_handle(运行时句柄) | `abort_handle` + `completion_receiver`                                                                  | 规格层只要求运行状态记录暴露运行时句柄语义. 在 runtime(运行时) 边界上, `abort_handle(强制中止句柄)` 用于强制中止真实 child future(子任务 future), `completion_receiver(完成接收端)` 用于读取退出报告. 无活动 attempt(尝试) 时两个字段都为 `None(无值)`. |
| 最后心跳            | `heartbeat_receiver` + `last_observed_heartbeat_at_unix_nanos`                                          | receiver(接收端) 保存 watch channel(观察通道), 字段保存 control loop(控制循环) 最近一次读取的状态记录值. 无活动 attempt(尝试) 时 receiver(接收端) 为 `None(无值)`.                                                                                                                       |
| readiness(就绪状态) | `readiness_receiver` + `last_observed_readiness`                                                        | 与最后心跳同构. `ReadinessState(就绪状态)` 明确区分 `Unreported(未上报)`, `Ready(已就绪)` 和 `NotReady(未就绪)`.                                                                                                                                                                          |
| 重启次数限制            | `restart_limit` (`RestartLimitState(重启次数限制状态)`)                                                | 由 runtime(运行时) 侧 `RestartLimitTracker(重启次数限制跟踪器)` 在 child exit(子任务退出) 处理期间刷新, 详见 research.md 决策二.                                                                                                                                                                                                                |

`attempt_cancel_delivered(尝试取消已送达)`, `abort_requested(已请求强制中止)`, `stop_state(停止状态)` 三个字段不在 spec.md FR-001 字面列表中, 但属于 spec.md FR-003 要求的"控制结果必须反映运行状态记录真实事实"所必须的派生字段, 详见下文 Fields 表与 contracts/child-runtime-state-control.md. `attempt_cancel_delivered(尝试取消已送达)` 是运行状态记录上的历史事实. `ChildControlResult.cancel_delivered(子任务控制结果取消已送达)` 是本次命令是否新发送取消的结果字段. 两者不得混用.

### Fields(字段)

| Field(字段)                             | Type(类型)                                                         | Description(说明)                                                      |
| --------------------------------------- | ------------------------------------------------------------------ | ---------------------------------------------------------------------- |
| `child_id`                              | `ChildId`                                                          | 稳定子任务标识.                                                        |
| `path`                                  | `SupervisorPath`                                                   | 子任务在 supervisor tree(监督树) 中的路径.                             |
| `generation`                            | `Option<Generation>`                                               | 当前活动尝试所属代次, 无活动 attempt(尝试) 时为 `None(无值)`.           |
| `attempt`                               | `Option<Attempt>`                                                  | 当前活动尝试编号, 无活动 attempt(尝试) 时为 `None(无值)`.               |
| `status`                                | `Option<ChildAttemptStatus>`                                          | 运行时状态, 无活动 attempt(尝试) 时为 `None(无值)`.                     |
| `operation`                            | `ChildControlOperation`                                              | 控制面要求运行状态记录执行的生命周期操作, 取值见下文.                                              |
| `cancellation_token`                    | `Option<CancellationToken>`                                        | 与 `TaskContext(任务上下文)` 共享的取消令牌, 无活动 attempt(尝试) 时为 `None(无值)`. |
| `abort_handle`                          | `Option<AbortHandle>`                                              | 指向真实 child future(子任务 future) 的强制中止句柄.                   |
| `completion_receiver`                   | `Option<watch::Receiver<Option<Result<ChildRunReport, SupervisorError>>>>` | 等待 child(子任务) 完成报告的接收端.                                   |
| `heartbeat_receiver`                    | `Option<watch::Receiver<Option<Instant>>>`                         | 来自 `TaskContext(任务上下文)` 的心跳观察接收端.                       |
| `readiness_receiver`                    | `Option<watch::Receiver<ReadinessState>>`                          | 来自 `TaskContext(任务上下文)` 的就绪观察接收端.                       |
| `last_observed_heartbeat_at_unix_nanos` | `Option<u128>`                                                     | 最近一次 control loop(控制循环) 读取到的心跳时间, 缺省为 `None(无值)`. |
| `last_observed_readiness`               | `ReadinessState`                                                   | 最近一次 control loop(控制循环) 读取到的就绪状态, 缺省为 `Unreported(未上报)`. |
| `restart_limit`                        | `RestartLimitState`                                            | runtime(运行时) 侧重启次数限制跟踪器刷新的剩余重启次数.                             |
| `attempt_cancel_delivered`              | `bool`                                                             | 当前 attempt(尝试) 是否已经收到过运行时取消.                            |
| `abort_requested`                       | `bool`                                                             | 当前 attempt(尝试) 是否已经被请求强制中止.                             |
| `stop_state`                            | `ChildStopState`                                                   | 当前 attempt(尝试) 的停止进度.                                         |
| `stop_deadline_at_unix_nanos`           | `Option<u128>`                                                     | 取消送达时刻加当前有效 `ShutdownPolicy.graceful_timeout(关闭策略优雅等待时间)` 后得到的停止截止时间, 没有等待中的停止命令时为 `None(无值)`. |
| `last_control_failure`                  | `Option<ChildControlFailure>`                                      | 最近一次停止失败原因, 仅在停止失败后为 `Some(有值)`.                   |

### Validation Rules(校验规则)

- `generation(代次)` 和 `attempt(尝试)` 同时为 `Some(有值)` 时, `(child_id, generation, attempt)` 必须唯一标识一个活动尝试, 同一运行状态记录在 attempt(尝试) 切换前必须先收到旧 attempt 的退出消息.
- 当 `attempt(尝试)` 为 `None(无值)` 时, `generation(代次)`, `status(状态)`, `cancellation_token(取消令牌)`, `abort_handle(强制中止句柄)`, `completion_receiver(完成接收端)`, `heartbeat_receiver(心跳接收端)` 和 `readiness_receiver(就绪接收端)` 必须同时为空.
- `cancellation_token(取消令牌)` 为 `Some(有值)` 时必须与 `TaskContext(任务上下文)` 中的 token(令牌) 是同一克隆, 运行状态记录 cancel(取消) 后任务必须能观察到取消.
- `abort_handle(强制中止句柄)` 为 `Some(有值)` 时必须指向真实 child future(子任务 future), 不能只指向上报任务.
- `last_observed_heartbeat_at_unix_nanos(最后观察心跳时间)` 必须由 `heartbeat_receiver(心跳接收端)` 的最新值或后续读取确认, 不得伪造为零值.
- `restart_limit(重启次数限制)` 在 `operation(操作)` 为 `Quarantined(已隔离)` 或 `Removed(已移除)` 时, 仍可读取最近一次状态, 但 runtime(运行时) 侧重启次数限制跟踪器不得继续消耗剩余次数.
- `stop_state(停止状态)` 在活动尝试路径上只能按 `Idle(空闲)` -> `CancelDelivered(已送达取消)` -> `Completed(已停止)` 或 `Failed(停止失败)` 顺序推进. 当运行状态记录当前无活动尝试时, 停止类命令直接得到 `NoActiveAttempt(无活动尝试)`, 不进入上述 `Idle` 链. `NoActiveAttempt(无活动尝试)` 不自动代表 `idempotent(幂等)`; 只有操作未变化且没有删除动作时才是幂等返回.
- `stop_state(停止状态)` 为 `CancelDelivered(已送达取消)` 时, `stop_deadline_at_unix_nanos(停止截止时间)` 必须为 `Some(有值)`, 且该值必须等于取消送达时刻加当前有效 `ShutdownPolicy.graceful_timeout(关闭策略优雅等待时间)`. 本功能不新增单独的控制命令等待窗口配置. `reconcile_stop_deadlines(调和停止截止时间)` 只能在该截止时间已经经过且 attempt(尝试) 仍未退出时推进到 `Failed(停止失败)`.
- `stop_state(停止状态)` 为 `Failed(停止失败)` 时, `last_control_failure(最近控制失败原因)` 必须为 `Some(有值)`, 并且 `stop_deadline_at_unix_nanos(停止截止时间)` 必须记录触发失败判断的截止时间.
- `attempt_cancel_delivered(尝试取消已送达)` 为 `true(是)` 且 `operation(操作)` 已经等于当前停止类命令的目标操作时, 重复命令必须复用既有取消状态, 不得再次调用 `CancellationToken::cancel(取消)`, `idempotent(幂等)` 必须为 `true(是)`.
- `RuntimeTimeBase(运行时时间基准)` 不属于单个 `ChildRuntimeState(子任务运行状态记录)` 的字段. `RuntimeControlState(运行时控制状态)` 必须持有唯一 `RuntimeTimeBase(运行时时间基准)`, 并在 `observe_liveness(观察存活)`, `to_record(生成记录)` 和 `update_restart_limit(更新重启次数限制)` 需要生成公开时间戳时以只读引用传入.

### State Transitions(状态转换)

```text
status:        Starting -> Running -> Ready
                   \-----> Cancelling -> Stopped
operation:    Active  -> Paused  | Quarantined | Removed
                Paused  -> Quarantined
                Paused  -> Removed
                Active  -> Removed
                Active  -> Quarantined
                Quarantined -> Removed
stop_state:    NoActiveAttempt(无活动尝试)   # 占位但无活动尝试, 或幂等命令立即返回, 不经 Idle 链
                Idle -> CancelDelivered -> Completed
                                     \-> Failed
```

注: `NoActiveAttempt(无活动尝试)` 表示命令路径上的停止进度状态记录, 与 `Idle(空闲)` 不得在同一解释路径上并存, 详见上文 `ChildStopState(子任务停止状态)` 校验规则.

注: 本规格不定义从 `Paused(已暂停)` 恢复到 `Active(活跃)` 的转换. `ResumeChild(恢复子任务)` 与 `RestartChild(重启子任务)` 是既有命令, 它们在 contracts/child-runtime-state-control.md 中被显式排除, 由后续切片定义恢复路径.

`Removed(已移除)` 是终态, exit handler(退出处理) 在 child(子任务) 退出后从 control loop(控制循环) 的 `child_runtime_states(子任务运行状态记录集合)` 中物理删除运行状态记录. 如果 `RemoveChild(移除子任务)` 命中无活动 attempt(尝试) 的运行状态记录, control loop(控制循环) 必须先构造 `ChildControlResult(子任务控制结果)`, 再在同一轮命令处理末尾物理删除运行状态记录. `Quarantined(已隔离)` 是隔离保持态, 阻止自动重启, 但运行状态记录仍存在于 `child_runtime_states(子任务运行状态记录集合)` 中以便操作者观察; 操作者仍可对隔离运行状态记录执行 `RemoveChild(移除子任务)`.

## Entity(实体): `ChildAttemptStatus(子任务尝试状态)`

`ChildAttemptStatus(子任务尝试状态)` 表示运行状态记录上活动尝试的运行时阶段. 与 `ChildRuntimeStatus(子任务运行时状态)` 一一对应, 但加上 `Cancelling(取消中)` 表达"已发送取消, 尚未确认退出". 该 enum(枚举) 属于 `src/control/outcome.rs`, 因为它是公开结果和 `CurrentState(当前状态)` 记录的字段. `src/runtime/child_runtime_state.rs` 可以直接使用该公开 enum(枚举), 但 `src/control/outcome.rs` 不得依赖 runtime(运行时) 模块.

### Values(取值)

- `Starting(启动中)`: 当前 attempt(尝试) 已经构造, 但尚未真正开始执行任务体.
- `Running(运行中)`: 当前 attempt(尝试) 正在执行任务体.
- `Ready(已就绪)`: 当前 attempt(尝试) 上报了 readiness(就绪状态).
- `Cancelling(取消中)`: runtime(运行时) 已经送达取消, 等待任务结束.
- `Stopped(已停止)`: 当前 attempt(尝试) 已经返回退出报告, 运行状态记录等待操作决策或被删除.

### Validation Rules(校验规则)

- `Ready(已就绪)` 必须有 `last_observed_readiness = ReadinessState::Ready(就绪状态为已就绪)`.
- `Cancelling(取消中)` 必须有 `attempt_cancel_delivered = true(尝试取消已送达)`.
- `Stopped(已停止)` 必须能从 `completion_receiver(完成接收端)` 中读取到 `Some(Ok)(有结果)`, 或者 `stop_state(停止状态)` 为 `Failed(停止失败)`.

## Entity(实体): `ChildControlOperation(子任务控制操作)`

`ChildControlOperation(子任务控制操作)` 表示控制面要求运行状态记录执行的生命周期操作, 与 `ManagedChildState(受管子任务状态)` 一一对应. 它不是 child(子任务) 当前是否运行的事实状态, 运行事实由 `ChildAttemptStatus(子任务尝试状态)` 表达. 该 enum(枚举) 属于 `src/control/outcome.rs`, 因为控制结果, dashboard model(仪表盘模型) 和 audit(审计) 都需要稳定序列化该操作. `src/runtime/child_runtime_state.rs` 负责维护字段值, 但不拥有公开 enum(枚举) 定义.

### Values(取值)

- `Active(活跃)`: 运行状态记录正常运行, supervision strategy(监督策略) 可以触发自动重启.
- `Paused(已暂停)`: 运行状态记录上 attempt(尝试) 必须被取消, 自动重启暂停. 本规格不定义从暂停恢复到活跃的命令语义.
- `Quarantined(已隔离)`: 运行状态记录被隔离并保持可观察, 自动重启被阻止, 即使 attempt(尝试) 退出也不重启; 操作者仍可执行 `RemoveChild(移除子任务)`.
- `Removed(已移除)`: 运行状态记录待删除, 当前 attempt(尝试) 退出后从 `child_runtime_states(子任务运行状态记录集合)` 中物理移除.

### Validation Rules(校验规则)

- 同一运行状态记录在同一时刻 operation(操作) 只能是上面四值之一.
- 从 `Quarantined(已隔离)` 或 `Removed(已移除)` 不得直接回到 `Active(活跃)`. `Quarantined(已隔离)` 只能继续保持隔离或通过 `RemoveChild(移除子任务)` 转为 `Removed(已移除)`. 本规格不定义隔离或移除后的恢复命令语义.

## Entity(实体): `ChildStopState(子任务停止状态)`

`ChildStopState(子任务停止状态)` 表示停止类控制命令的进度. 该 enum(枚举) 属于 `src/control/outcome.rs`, 因为它是控制结果的字段.

### Values(取值)

- `Idle(空闲)`: 运行状态记录没有发起过停止类命令, 也没有自动重启动作.
- `NoActiveAttempt(无活动尝试)`: 运行状态记录当前没有活动尝试, 停止类命令不发送取消, 并直接返回无活动尝试结果.
- `CancelDelivered(已送达取消)`: runtime(运行时) 已经向当前 attempt(尝试) 发送取消, 但尚未观察到退出.
- `Completed(已停止)`: 当前 attempt(尝试) 已经退出, 运行状态记录完成停止动作.
- `Failed(停止失败)`: 当前 attempt(尝试) 在停止截止时间经过后仍未退出.

### Validation Rules(校验规则)

- `Completed(已停止)` 必须由 exit handler(退出处理) 在收到 `ChildAttemptMessage::Exited(子任务退出消息)` 后写入.
- `Failed(停止失败)` 必须携带失败 phase(阶段) 和原因. 运行状态记录的 `last_control_failure(最近控制失败原因)` 与 `ChildControlResult.failure(控制结果失败原因)` 字段都必须为 `Some(有值)`.
- `Idle(空闲)` 与 `NoActiveAttempt(无活动尝试)` 不得共存, 运行状态记录无活动尝试时优先记 `NoActiveAttempt(无活动尝试)`.

## Entity(实体): `ChildControlResult(子任务控制结果)`

`ChildControlResult(子任务控制结果)` 是公开类型, 由 `CommandResult::ChildControl(子任务控制命令结果)` 携带. 该类型属于 `src/control/outcome.rs`.

### Fields(字段)

| Field(字段)         | Type(类型)                    | Description(说明)                                                     |
| ------------------- | ----------------------------- | --------------------------------------------------------------------- |
| `child_id`          | `ChildId`                     | 控制目标子任务.                                                       |
| `attempt`           | `Option<Attempt>`             | 控制命令实际作用的 attempt(尝试) 编号. 没有活动尝试时为 `None(无值)`. |
| `generation`        | `Option<Generation>`          | 控制命令实际作用的 generation(代次). 没有活动尝试时为 `None(无值)`.   |
| `operation_before` | `ChildControlOperation`         | 命令到达前的操作.                                                 |
| `operation_after`  | `ChildControlOperation`         | 命令处理后的操作.                                                 |
| `status`            | `Option<ChildAttemptStatus>`     | 当前 attempt(尝试) 的运行时状态, 无活动尝试时为 `None(无值)`.         |
| `cancel_delivered`  | `bool`                        | 本命令是否触发了取消送达. 已停止任务的幂等返回必须为 `false(否)`.     |
| `stop_state`        | `ChildStopState`              | 命令处理后运行状态记录停止进度.                                               |
| `restart_limit`    | `RestartLimitState`       | 运行状态记录最近一次 runtime(运行时) 重启次数限制跟踪写入的剩余状态.                   |
| `liveness`          | `ChildLivenessState`       | 控制结果同步的 heartbeat 与 readiness 状态.                            |
| `idempotent`        | `bool`                        | 本次命令是否复用了已有操作, 复用时为 `true(是)`.                  |
| `failure`           | `Option<ChildControlFailure>` | 停止失败时携带的失败 phase(阶段) 和原因.                              |

### Validation Rules(校验规则)

- `attempt(尝试)` 为 `None(无值)` 时 `cancel_delivered(取消已送达)` 必须为 `false(否)`, `stop_state(停止状态)` 必须为 `NoActiveAttempt(无活动尝试)`.
- `idempotent(幂等)` 为 `true(是)` 时 `operation_before(命令前操作)` 必须等于 `operation_after(命令后操作)`, 且 `cancel_delivered(取消已送达)` 必须为 `false(否)`.
- `attempt(尝试)` 为 `None(无值)` 时不自动允许 `idempotent(幂等)` 为 `true(是)`. 如果命令改变 `operation(操作)` 或触发物理删除, `idempotent(幂等)` 必须为 `false(否)`.
- `stop_state(停止状态)` 为 `Failed(停止失败)` 时 `failure(失败原因)` 必须为 `Some(有值)`.
- `failure(失败原因)` 必须反映运行状态记录 `last_control_failure(最近控制失败原因)`, 不得在 outcome(结果) 构造时临时伪造不同原因.
- `restart_limit(重启次数限制)` 必须始终携带最近一次状态记录, 即使运行状态记录 operation(操作) 是 `Quarantined(已隔离)`.

## Entity(实体): `RestartLimitState(重启次数限制状态)`

`RestartLimitState(重启次数限制状态)` 是公开类型, 属于 `src/control/outcome.rs`. 它表达 runtime(运行时) 侧重启次数限制跟踪器在策略窗口内记录的已使用次数与剩余重启尝试次数. `window(窗口)` 和 `limit(上限)` 来自既有 `RestartLimit(重启次数限制)` 配置来源, 优先级依次为 child strategy override(子任务策略覆盖), group strategy(分组策略), supervisor spec(监督器声明) 和配置层默认 `PolicyConfig.child_restart_limit / child_restart_window_ms(策略配置子任务重启上限与窗口)`. 当前 `src/policy/decision.rs` 中 `PolicyEngine(策略引擎)` 是无状态结构, `RestartPolicy(重启策略)` 只有 `Permanent(永久)`, `Transient(临时故障重启)` 和 `Temporary(一次性)` 三个枚举值, 不提供 `used / remaining(已使用与剩余)` 运行时历史字段.

### Fields(字段)

| Field(字段)             | Type(类型) | Description(说明)                                                |
| ----------------------- | ---------- | ---------------------------------------------------------------- |
| `window`                | `Duration` | runtime(运行时) 侧重启次数限制跟踪器使用的策略窗口长度.                  |
| `limit`                 | `u32`      | 策略窗口内允许的最大失败尝试数.                                  |
| `used`                  | `u32`      | 当前已经计入窗口的失败尝试数.                                    |
| `remaining`             | `u32`      | 当前剩余的重启尝试次数, 等于 `limit.saturating_sub(used)`(饱和相减) 的结果. |
| `exhausted`             | `bool`     | 当前重启次数限制是否已耗尽.                                              |
| `updated_at_unix_nanos` | `u128`     | 最近一次 runtime(运行时) 侧重启次数限制跟踪器写入状态的时间, 使用 `RuntimeTimeBase(运行时时间基准)` 生成. |

### Validation Rules(校验规则)

- `remaining(剩余)` 必须等于 `limit.saturating_sub(used)`(上限对已使用次数做饱和相减). 当 `used(已使用)` 大于 `limit(上限)` 时, `remaining(剩余)` 必须为 `0(零)`.
- `exhausted(已耗尽)` 必须等于 `remaining == 0(剩余等于零)`.
- `updated_at_unix_nanos(更新时间)` 必须单调递增. runtime(运行时) 侧必须先用 `RuntimeTimeBase(运行时时间基准)` 生成当前纳秒时间戳, 再与同一运行状态记录前一次 `updated_at_unix_nanos(更新时间)` 比较; 如果当前值小于或等于前一次值, 必须写入 `previous + 1(前值加一)` 防止系统时间回拨或同纳秒刷新造成非递增.

## Entity(实体): `ChildLivenessState(子任务存活状态)`

`ChildLivenessState(子任务存活状态)` 是公开类型, 属于 `src/control/outcome.rs`. 它是 heartbeat(心跳) 与 readiness(就绪状态) 在控制结果中的不可变状态记录.

### Fields(字段)

| Field(字段)                    | Type(类型)     | Description(说明)                                                                                                                    |
| ------------------------------ | -------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| `last_heartbeat_at_unix_nanos` | `Option<u128>` | 最近一次心跳时间, 从 `tokio::time::Instant(单调时刻)` 换算到 `SystemTime(系统时间)` 后转为纳秒时间戳. 没有收到心跳时为 `None(无值)`. |
| `heartbeat_stale`              | `bool`           | 是否被识别为心跳陈旧. 当前 attempt(尝试) 未上报心跳时为 `false(否)`, 上报后超过 `heartbeat_timeout(心跳超时)` 时为 `true(是)`.       |
| `readiness`                    | `ReadinessState` | 最近一次 readiness(就绪状态). 未上报时为 `Unreported(未上报)`, 上报为就绪时为 `Ready(已就绪)`, 上报为非就绪时为 `NotReady(未就绪)`. |

### Validation Rules(校验规则)

- `heartbeat_stale(心跳陈旧)` 为 `true(是)` 必须有 `last_heartbeat_at_unix_nanos = Some(有值)`. 未上报心跳的运行状态记录不得被标为陈旧.
- `readiness(就绪状态)` 必须区分"未上报"和"已上报但非就绪", 不得用 `Unreported(未上报)` 表达"已上报但非就绪".
- `last_heartbeat_at_unix_nanos(最后心跳纳秒时间戳)` 必须使用近似 `UTC(协调世界时)` 纳秒时间戳, 也就是自 `UNIX_EPOCH(Unix 纪元常量)` 起经过的纳秒数. supervisor runtime(监督器运行时) 初始化时必须创建唯一 `RuntimeTimeBase(运行时时间基准)`, 该基准包含 `base_instant = tokio::time::Instant::now()` 和 `base_unix_nanos = SystemTime::now().duration_since(UNIX_EPOCH)` 换算出的纳秒值. 将心跳 `Instant(单调时刻)` 换算为 `u128` 时必须使用公式: `base_unix_nanos + (heartbeat_instant - base_instant)`; 如果心跳时刻早于 `base_instant(基准单调时刻)`, 必须使用 `base_unix_nanos.saturating_sub(base_instant - heartbeat_instant)` 防止下溢. 实现不得直接使用 `SystemTime::UNIX_EPOCH.elapsed()` 代表某个历史心跳时刻, 也不得把 `tokio::time::Instant` 的相对值直接写入公开结果.
- `Generation(代次)` 和 `UNIX_EPOCH(Unix 纪元常量)` 是两个不同概念. `Generation(代次)` 表示同一个 child(子任务) 跨重启的新旧运行实例编号. `Attempt(尝试)` 表示某次实际启动出来的任务尝试. `UNIX_EPOCH(Unix 纪元常量)` 只表示时间戳起点, 不能用于表示任务运行代次.

## Entity(实体): `ReadinessState(就绪状态)`

`ReadinessState(就绪状态)` 是公开就绪观测枚举, 属于 `src/readiness/signal.rs`. `ReadySignal(就绪信号)` 必须通过 `watch::Receiver<ReadinessState>` 发布该枚举, 以便 control loop(控制循环) 能够区分未上报和已上报但未就绪.

### Values(取值)

- `Unreported(未上报)`: child(子任务) 尚未上报 readiness(就绪状态).
- `Ready(已就绪)`: child(子任务) 已经上报 readiness(就绪状态) 为就绪.
- `NotReady(未就绪)`: child(子任务) 已经上报 readiness(就绪状态) 为未就绪或退化.

### Validation Rules(校验规则)

- `ReadySignal::new(新建就绪信号)` 必须以 `Unreported(未上报)` 初始化 receiver(接收端), 不得用 `false(否)` 伪造未上报状态.
- `TaskContext(任务上下文)` 必须继续提供 `mark_ready(标记就绪)` 能力, 并新增 `set_readiness(设置就绪状态)` 能力. `mark_ready(标记就绪)` 必须等价于调用 `set_readiness(ReadinessState::Ready)`(设置就绪状态为已就绪).

## Entity(实体): `ChildControlFailurePhase(子任务控制失败阶段)`

`ChildControlFailurePhase(子任务控制失败阶段)` 是公开类型, 属于 `src/control/outcome.rs`. 它替代自由字符串阶段, 让控制失败原因和事件字段使用稳定枚举.

### Values(取值)

- `WaitCompletion(等待完成)`: 等待子任务退出阶段超过停止截止时间.

### Validation Rules(校验规则)

- 序列化时必须使用稳定 `snake_case(蛇形命名)` 字符串, 例如 `wait_completion`.
- 本功能的控制命令路径只能产生 `WaitCompletion(等待完成)` 阶段. 新增阶段必须先更新 contracts(契约), data-model(数据模型), tasks(任务) 和事件测试, 不得直接使用自由字符串绕过枚举.

## Entity(实体): `ChildControlFailure(子任务控制失败原因)`

`ChildControlFailure(子任务控制失败原因)` 是公开类型, 属于 `src/control/outcome.rs`. 它表达停止类控制命令失败时的结构化原因.

### Fields(字段)

| Field(字段)   | Type(类型) | Description(说明)                                                |
| ------------- | ---------- | ---------------------------------------------------------------- |
| `phase`       | `ChildControlFailurePhase` | 失败阶段, 取值见上文.                                             |
| `reason`      | `String`   | 人类可读原因, 必须非空, 不得使用 generic failure(泛化失败) 描述. |
| `recoverable` | `bool`     | 调用方是否可以通过重发命令恢复.                                  |

### Validation Rules(校验规则)

- `phase(阶段)` 必须使用 `ChildControlFailurePhase(子任务控制失败阶段)` 枚举, 不得使用自由字符串.
- `reason(原因)` 必须非空且不得只写"failed"或"error".

## Entity(实体): `CommandResult::ChildControl(子任务控制命令结果)` (替换)

替换 `src/control/command.rs` 中现有 `CommandResult::ChildState(子任务状态命令结果)` 变体, 改为携带 `ChildControlResult(子任务控制结果)`.

### New shape(新形状)

```rust
pub enum CommandResult {
    ChildAdded { child_manifest: String },
    ChildControl { outcome: ChildControlResult },
    CurrentState { state: CurrentState },
    Shutdown { result: ShutdownResult },
}
```

### Validation Rules(校验规则)

- `AddChild(添加子任务)` 仍返回 `ChildAdded(子任务已添加)`, 不在本功能范围内.
- `RemoveChild(移除子任务)`, `PauseChild(暂停子任务)`, `QuarantineChild(隔离子任务)` 必须返回 `ChildControl(子任务控制命令结果)` 变体.
- `CurrentState(当前状态)` 当前返回的 `CurrentState(当前状态)` 结构必须扩展, 详见 `CurrentState(当前状态)` 实体修订.

## Entity(实体): `CurrentState(当前状态)` Extension(扩展)

`CurrentState(当前状态)` 当前位于 `src/control/command.rs`, 仅有 `child_count(子任务数)` 和 `shutdown_completed(关闭完成)` 字段. 本功能要求 `CurrentState(当前状态)` 输出运行时运行状态摘要.

### Extended Fields(扩展字段)

| Field(字段)          | Type(类型)               | Description(说明)                 |
| -------------------- | ------------------------ | --------------------------------- |
| `child_count`        | `usize`                  | 沿用: 控制循环已知子任务数量.     |
| `shutdown_completed` | `bool`                   | 沿用: 关闭是否完成.               |
| `child_runtime_records`              | `Vec<ChildRuntimeRecord>` | 新增: 当前每个运行状态记录的对外可见状态记录. |

### Validation Rules(校验规则)

- `child_runtime_records(子任务运行状态记录集合)` 必须覆盖 `RuntimeControlState.child_runtime_states` 中的全部运行状态记录, 且按声明顺序排列.
- 关闭完成后 `child_runtime_records(子任务运行状态记录集合)` 仍可暴露最后一次操作, 直到运行状态记录被物理删除.

## Entity(实体): `ChildRuntimeRecord(子任务运行状态记录)`

`ChildRuntimeRecord(子任务运行状态记录)` 是公开类型, 属于 `src/control/outcome.rs`. 它是 `ChildRuntimeState(子任务运行状态记录)` 在 `CurrentState(当前状态)` 中的不可变投影.

### Fields(字段)

| Field(字段)      | Type(类型)              | Description(说明)            |
| ---------------- | ----------------------- | ---------------------------- |
| `child_id`       | `ChildId`               | 运行状态记录对应子任务.              |
| `path`           | `SupervisorPath`        | 运行状态记录路径.                    |
| `generation`     | `Option<Generation>`    | 当前活动 generation(代次), 无活动 attempt(尝试) 时为 `None(无值)`. |
| `attempt`        | `Option<Attempt>`       | 当前活动 attempt(尝试), 无活动 attempt(尝试) 时为 `None(无值)`.    |
| `status`         | `Option<ChildAttemptStatus>` | 当前运行时状态, 无活动 attempt(尝试) 时为 `None(无值)`.           |
| `operation`     | `ChildControlOperation`   | 当前操作.                |
| `liveness`       | `ChildLivenessState` | heartbeat 与 readiness 状态. |
| `restart_limit` | `RestartLimitState` | 重启次数限制状态.                |
| `stop_state`     | `ChildStopState`        | 当前停止进度.                |
| `failure`        | `Option<ChildControlFailure>` | 停止失败原因, 未失败时为 `None(无值)`. |

### Validation Rules(校验规则)

- `ChildRuntimeRecord(子任务运行状态记录)` 不得携带 Tokio 句柄, 它必须可以 `Clone(克隆)` 和 `Serialize(序列化)`.
- 状态记录必须与读取时刻的 `ChildRuntimeState(子任务运行状态记录)` 字段一致, 不得在状态记录构造期间观察到非原子状态.
- `stop_state(停止状态)` 为 `Failed(停止失败)` 时, `failure(失败原因)` 必须为 `Some(有值)`.
- `attempt(尝试)` 为 `None(无值)` 时, `generation(代次)` 与 `status(状态)` 也必须为 `None(无值)`, 且 `stop_state(停止状态)` 必须为 `NoActiveAttempt(无活动尝试)`.
