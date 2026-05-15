# 运行时控制

语言: [English](../en/runtime-control.html)

## 控制入口

`SupervisorHandle`(监督器句柄)是运行时控制入口. 它通过命令通道把请求发送给 runtime control loop(运行时控制循环), 并返回 `CommandResult`(命令结果).

## 控制命令

- `add_child`: 当 `DynamicSupervisorPolicy`(动态监督器策略) 允许新增 child(子任务) 时, 接受 dynamic child manifest(动态子任务清单文本).
- `remove_child`: 把目标 child(子任务) 的运行状态记录标记为 `Removed(已移除)`, 向活动 attempt(尝试) 发送 cancel(取消), 并在 attempt(尝试) 退出后移除运行状态记录.
- `restart_child`: 请求目标 child(子任务)重启.
- `pause_child`: 把目标 child(子任务) 的运行状态记录标记为 `Paused(已暂停)`, 向活动 attempt(尝试) 发送 cancel(取消), 并暂停自动重启.
- `resume_child`: 恢复目标 child(子任务)治理.
- `quarantine_child`: 把目标 child(子任务) 的运行状态记录标记为 `Quarantined(已隔离)`, 向活动 attempt(尝试) 发送 cancel(取消), 并阻止自动重启.
- `shutdown_tree`: 关闭整棵监督树.
- `current_state`: 返回当前 `SupervisorState`(监督器状态), 并在 `CurrentState.child_runtime_records(当前状态子任务运行状态记录集合)` 中暴露每个 child(子任务) 的运行状态事实.
- `subscribe_events`: 订阅生命周期事件.
- `is_alive`: 快速判断 runtime control loop(运行时控制循环) 是否仍可接收普通控制命令.
- `health`: 返回 `RuntimeHealthReport`(运行时健康报告), 包含控制面状态, 启动时间, 最近观测时间和最终失败原因.
- `join`: 等待 runtime control plane(运行时控制面)进入最终态, 并重复返回同一个 `RuntimeExitReport`(运行时退出报告).
- `shutdown`: 只关闭 runtime control plane(运行时控制面), 不替代 `shutdown_tree`(监督树关闭).

## 子任务运行状态控制

`PauseChild(暂停子任务)`, `RemoveChild(移除子任务)` 和 `QuarantineChild(隔离子任务)` 是本功能定义的停止类控制命令. 这 3 条命令都会返回 `CommandResult::ChildControl(子任务控制命令结果)`, 结果中包含 `ChildControlResult(子任务控制结果)`. 旧的 `CommandResult::ChildState(子任务状态命令结果)` 不再属于公开结果形状.

`PauseChild(暂停子任务)` 会把 `ChildRuntimeState.operation(子任务运行状态记录操作)` 写为 `Paused(已暂停)`. 如果当前存在活动 attempt(尝试), runtime control loop(运行时控制循环) 会向该 attempt(尝试) 发送 cancel(取消), 并把停止进度推进到 `CancelDelivered(已送达取消)`. 暂停期间, supervision strategy(监督策略) 不会针对该 child(子任务) 自动重启.

`RemoveChild(移除子任务)` 会把 `ChildRuntimeState.operation(子任务运行状态记录操作)` 写为 `Removed(已移除)`. 如果当前存在活动 attempt(尝试), runtime control loop(运行时控制循环) 会先发送 cancel(取消), 等 attempt(尝试) 退出后再从 `child_runtime_states(子任务运行状态记录集合)` 中物理删除记录. 如果当前没有活动 attempt(尝试), runtime control loop(运行时控制循环) 会返回 `NoActiveAttempt(无活动尝试)` 结果, 然后删除运行状态记录.

`QuarantineChild(隔离子任务)` 会把 `ChildRuntimeState.operation(子任务运行状态记录操作)` 写为 `Quarantined(已隔离)`. 如果当前存在活动 attempt(尝试), runtime control loop(运行时控制循环) 会发送 cancel(取消). 隔离后的运行状态记录仍然保留, 但是 supervision strategy(监督策略) 不会继续自动重启该 child(子任务). 操作者仍然可以后续执行 `RemoveChild(移除子任务)`.

这 3 条停止类控制命令不会同步等待 child future(子任务 future) 结束. 如果 child(子任务) 长时间忽略 cancel(取消), 后续 `CurrentState(当前状态)` 或重复停止类控制命令会触发 `reconcile_stop_deadlines(调和停止截止时间)`, 并通过 `ChildControlFailure(子任务控制失败原因)` 暴露停止失败.

`CurrentState(当前状态)` 会返回 `child_runtime_records(子任务运行状态记录集合)`. 每条 `ChildRuntimeRecord(子任务运行状态记录)` 都按声明顺序排列. 构造过程只做非阻塞读取, 不等待 child future(子任务 future), 不执行额外 I/O(输入输出). 该集合是查看运行状态事实的主入口.

`RestartChild(重启子任务)` 和 `ResumeChild(恢复子任务)` 仍然是既有命令. 本功能只要求它们不破坏运行状态事实, 不把它们定义为新增生命周期语义.

完整契约见 [`child-runtime-state-control.md`](../../specs/004-3-child-runtime-state-control/contracts/child-runtime-state-control.md).

## `ChildControlResult(子任务控制结果)` 字段

- `child_id(子任务标识)`: 被控制的 child(子任务) 稳定标识.
- `attempt(尝试)`: 命令实际作用的活动 attempt(尝试). 没有活动 attempt(尝试) 时为 `None(无值)`.
- `generation(代次)`: 命令实际作用的 generation(代次). 没有活动 attempt(尝试) 时为 `None(无值)`.
- `operation_before(命令前操作)`: 命令到达时的 `ChildControlOperation(子任务控制操作)`.
- `operation_after(命令后操作)`: 命令处理后的 `ChildControlOperation(子任务控制操作)`.
- `status(状态)`: 当前 attempt(尝试) 的 `ChildAttemptStatus(子任务尝试状态)`. 没有活动 attempt(尝试) 时为 `None(无值)`.
- `cancel_delivered(取消已送达)`: 本次命令是否真正发送了 cancel(取消).
- `stop_state(停止状态)`: 本次命令处理后的 `ChildStopState(子任务停止状态)`.
- `restart_limit(重启次数限制)`: 当前 `RestartLimitState(重启次数限制状态)`, 包含窗口, 上限, 已使用次数, 剩余次数和耗尽标志.
- `liveness(存活状态)`: 当前 `ChildLivenessState(子任务存活状态)`, 包含最后心跳时间, 心跳是否陈旧和 readiness(就绪状态).
- `idempotent(幂等)`: 本次命令是否复用了已经存在的目标状态.
- `failure(失败原因)`: 当前控制失败原因. 没有失败时为 `None(无值)`.

## `ChildRuntimeRecord(子任务运行状态记录)` 字段

- `child_id(子任务标识)`: 运行状态记录对应的 child(子任务) 稳定标识.
- `path(路径)`: child(子任务) 在 supervisor tree(监督树) 中的路径.
- `generation(代次)`: 当前活动 generation(代次). 没有活动 attempt(尝试) 时为 `None(无值)`.
- `attempt(尝试)`: 当前活动 attempt(尝试). 没有活动 attempt(尝试) 时为 `None(无值)`.
- `status(状态)`: 当前 attempt(尝试) 的 `ChildAttemptStatus(子任务尝试状态)`.
- `operation(操作)`: 当前 `ChildControlOperation(子任务控制操作)`, 可能是 `Active(活跃)`, `Paused(已暂停)`, `Quarantined(已隔离)` 或 `Removed(已移除)`.
- `liveness(存活状态)`: 当前 `ChildLivenessState(子任务存活状态)`.
- `restart_limit(重启次数限制)`: 当前 `RestartLimitState(重启次数限制状态)`.
- `stop_state(停止状态)`: 当前 `ChildStopState(子任务停止状态)`.
- `failure(失败原因)`: 最近一次 `ChildControlFailure(子任务控制失败原因)`. 当 `stop_state(停止状态)` 为 `Failed(停止失败)` 时必须为 `Some(有值)`.

## 幂等语义

重复控制命令不应该制造不可恢复错误. 已暂停的 child(子任务)再次暂停时返回当前状态. 已隔离的 child(子任务)再次隔离时返回当前状态. 已完成 shutdown(关闭)后再次关闭时返回已有关闭结果.

`join`(等待结束) 会缓存控制循环的最终 `RuntimeExitReport`(运行时退出报告). 同一个 handle(句柄) 重复调用 `join`(等待结束) 时, 每次都返回相同结果, 不会再次消费底层 `JoinHandle`(任务句柄).

`shutdown`(关闭) 只请求 runtime control loop(运行时控制循环) 正常退出. 如果控制面已经 completed(已完成) 或 failed(失败), 再次调用 `shutdown`(关闭) 会直接返回已有最终报告. `shutdown_tree`(监督树关闭) 仍然负责 child task(子任务)和整棵监督树的关闭语义.

## 运行时健康

`is_alive`(是否存活) 是低成本状态判断. 当控制面处于 alive(存活) 时, 它返回 `true`. 当控制面处于 starting(启动中), shutting_down(正在关闭), completed(已完成) 或 failed(失败) 时, 它返回 `false`.

`health`(健康报告) 返回结构化状态. 控制面异常退出后, `health`(健康报告) 仍然可以读取 failed(失败)状态, failure phase(失败阶段), reason(原因), panic(恐慌)标记和 recoverable(可恢复)标记. 普通控制命令在控制面结束后会返回包含同一退出原因的 `SupervisorError`(监督器错误).

## 动态添加

运行时会在接受 manifest(清单文本) 前执行 dynamic addition(动态添加) 治理. 当 dynamic supervision(动态监督) 被禁用, 或 declared child count(声明子任务数量) 加 dynamic child count(动态子任务数量) 已经达到配置上限时, `add_child`(添加子任务) 会被拒绝. `current_state`(当前状态) 的 `child_count`(子任务数量) 包含已经接受的 dynamic manifest(动态清单文本).

## 审计数据

每个控制命令都带有 `requested_by`(请求者), `reason`(原因), `target_path`(目标路径), `accepted_at`(接受时间)和 `command_id`(命令标识). 这些字段用于 audit event(审计事件)和问题追踪.

`requested_by`(请求者) 和 `reason`(原因) 必须提供非空文本. `SupervisorHandle`(监督器句柄) 会在命令进入 channel(通道) 前拒绝空值, runtime control loop(运行时控制循环) 也会在执行命令前再次校验. 这样做可以保证人工操作, dashboard IPC(看板进程间通信) 转发和内部控制调用都留下可追踪的审计来源.
