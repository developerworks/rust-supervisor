# Data Model(数据模型): 真实关闭流水线

## Entity(实体): `ShutdownPipeline(关闭流水线)`

`ShutdownPipeline(关闭流水线)` 表示一次 `ShutdownTree(关闭监督树)` 的执行过程. 它属于 `src/runtime/shutdown_pipeline.rs`, 并由 `RuntimeControlState(运行时控制状态)` 调用.

### Fields(字段)

| Field(字段) | Type(类型) | Description(说明) |
|-------------|------------|-------------------|
| `cause` | `ShutdownCause` | 第一次关闭请求记录的调用者和原因. |
| `policy` | `ShutdownPolicy` | 优雅等待和强制中止等待的时间预算. |
| `phase` | `ShutdownPhase` | 当前关闭阶段, 取值来自 `src/shutdown/stage.rs`. |
| `started_at_unix_nanos` | `u128` | 关闭流水线开始时间. |
| `completed_at_unix_nanos` | `Option<u128>` | 关闭流水线完成时间. |
| `wait_order` | `Vec<ChildId>` | 按 `shutdown_order(关闭顺序)` 计算出的等待顺序. |
| `outcomes` | `BTreeMap<ChildId, ChildShutdownOutcome>` | 每个 child(子任务) 的最终关闭结果. |
| `cached_report` | `Option<ShutdownPipelineReport>` | 已完成关闭的缓存报告, 类型属于 `src/shutdown/report.rs`, 用于重复请求. |

### Validation Rules(校验规则)

- `cause(原因)` 必须来自第一个通过校验的 `ShutdownTree(关闭监督树)` 请求.
- `wait_order(等待顺序)` 必须只包含 supervisor tree(监督树) 中声明的 child(子任务).
- `outcomes(结果集合)` 在完成阶段必须覆盖全部声明 child(子任务).
- `completed_at_unix_nanos(完成时间)` 必须大于或等于 `started_at_unix_nanos(开始时间)`.

### State Transitions(状态转换)

```text
Idle -> RequestStop -> GracefulDrain -> AbortStragglers -> Reconcile -> Completed
```

每次转换必须产生 `ShutdownPhaseChanged(关闭阶段变化)` 事件或等价诊断. `Completed(已完成)` 后不能再次启动新流水线.

## Entity(实体): `RunningChildAttempt(运行中子任务尝试)`

`RunningChildAttempt(运行中子任务尝试)` 表示当前 control loop(控制循环) 认为仍然运行中的一个 child attempt(子任务尝试).

### Fields(字段)

| Field(字段) | Type(类型) | Description(说明) |
|-------------|------------|-------------------|
| `child_id` | `ChildId` | 稳定 child(子任务) 标识. |
| `path` | `SupervisorPath` | child(子任务) 在 supervisor tree(监督树) 中的路径. |
| `generation` | `Generation` | 当前 runtime slot(运行时槽位) 的代际. |
| `attempt` | `Attempt` | 当前运行尝试编号. |
| `cancellation_token` | `CancellationToken` | 运行时保存的取消令牌 clone(克隆). |
| `abort_handle` | `AbortHandle` | 能中止真实 child future(子任务 future) 的句柄. |
| `completion` | `oneshot::Receiver<ChildRunReport>` 或等价类型 | child attempt(子任务尝试) 完成后返回结果的接收端. |
| `cancel_delivered` | `bool` | 运行时是否已经向该尝试发送取消. |
| `abort_requested` | `bool` | 运行时是否已经请求强制中止. |

### Validation Rules(校验规则)

- `(child_id, generation, attempt)` 必须唯一标识一个运行中尝试.
- `abort_handle(强制中止句柄)` 必须指向真实 child future(子任务 future), 不能只指向上报任务.
- `completion(完成接收端)` 只能被关闭流水线或正常 child exit(子任务退出) 处理一次.

## Entity(实体): `ChildShutdownOutcome(子任务关闭结果)`

`ChildShutdownOutcome(子任务关闭结果)` 是调用者和观测系统读取的 per-child(逐子任务) 关闭事实. 该公开报告类型属于 `src/shutdown/report.rs`, runtime(运行时) 只负责生成它.

### Fields(字段)

| Field(字段) | Type(类型) | Description(说明) |
|-------------|------------|-------------------|
| `child_id` | `ChildId` | 结果所属 child(子任务). |
| `path` | `SupervisorPath` | child(子任务) 路径. |
| `generation` | `Generation` | 关闭时关联的代际. |
| `attempt` | `Attempt` | 关闭时关联的尝试编号. |
| `status` | `ChildShutdownStatus` | 最终关闭分类. |
| `cancel_delivered` | `bool` | 是否发送了取消. |
| `exit` | `Option<TaskExit>` | 正常完成或被中止后可获得的退出分类. |
| `phase` | `ShutdownPhase` | 该结果在哪个阶段形成. |
| `reason` | `String` | 可读原因, 用于诊断. |

### `ChildShutdownStatus(子任务关闭状态)` Values(取值)

- `AlreadyExited(已经退出)` 表示关闭请求前该 child(子任务) 已经不在 active attempt(活动尝试) 集合中. 没有运行中任务时, 每个声明 child(子任务) 都必须使用该状态进入最终报告.
- `Graceful(优雅完成)` 表示运行时发送取消后, 该 child(子任务) 在 `graceful_timeout(优雅超时)` 前返回.
- `Aborted(已强制中止)` 表示该 child(子任务) 超时后被 `abort(强制中止)` 并完成.
- `AbortFailed(强制中止失败)` 表示运行时请求强制中止后仍无法在 `abort_wait(强制中止等待)` 内完成.
- `LateReport(迟到报告)` 表示该 child(子任务) 在对账期间或完成后才上报退出.

### Validation Rules(校验规则)

- `Graceful(优雅完成)` 必须有 `cancel_delivered = true`.
- `AlreadyExited(已经退出)` 不得再次发送取消.
- `Aborted(已强制中止)` 必须有 `abort_requested = true` 的内部证据.
- `AbortFailed(强制中止失败)` 必须记录失败阶段和原因.

## Entity(实体): `ShutdownPipelineReport(关闭流水线报告)`

`ShutdownPipelineReport(关闭流水线报告)` 是 `ShutdownResult(关闭结果)` 携带的完整摘要. 该公开报告类型属于 `src/shutdown/report.rs`.

### Fields(字段)

| Field(字段) | Type(类型) | Description(说明) |
|-------------|------------|-------------------|
| `cause` | `ShutdownCause` | 关闭原因. |
| `started_at_unix_nanos` | `u128` | 关闭开始时间. |
| `completed_at_unix_nanos` | `u128` | 关闭完成时间. |
| `phase` | `ShutdownPhase` | 关闭完成时的阶段, 必须是 `Completed(已完成)`. |
| `outcomes` | `Vec<ChildShutdownOutcome>` | 每个 child(子任务) 的关闭结果. |
| `reconcile` | `ShutdownReconcileReport` | 运行时资源对账摘要. |
| `idempotent` | `bool` | 该报告是否来自重复关闭请求. |

### Validation Rules(校验规则)

- `phase(阶段)` 必须是 `Completed(已完成)`.
- `outcomes(结果)` 必须覆盖全部声明 child(子任务), 并且同一 `child_id(子任务标识)` 只能出现一次.
- `reconcile(对账报告)` 必须列出 registry(注册表), runtime handles(运行时句柄), journal(日志), metrics(指标) 和 socket(套接字) 的状态.

## Entity(实体): `ShutdownReconcileReport(关闭对账报告)`

`ShutdownReconcileReport(关闭对账报告)` 表示关闭流水线完成后的资源状态. 该公开报告类型属于 `src/shutdown/report.rs`.

### Fields(字段)

| Field(字段) | Type(类型) | Description(说明) |
|-------------|------------|-------------------|
| `registry_status` | `ResourceReconcileStatus` | registry(注册表) 是否已经标记退出或清理. |
| `runtime_handle_status` | `ResourceReconcileStatus` | active attempt(活动尝试) 句柄是否已经移除. |
| `journal_status` | `ResourceReconcileStatus` | journal(日志) 是否已经收到关闭摘要事件. |
| `metrics_status` | `ResourceReconcileStatus` | metrics(指标) 是否已经记录关闭摘要. |
| `socket_status` | `ResourceReconcileStatus` | socket(套接字) 资源状态. 核心 runtime(运行时) 不直接拥有 dashboard IPC socket(仪表盘进程间通信套接字) 时必须是 `NotOwned(非运行时拥有)`. |
| `warnings` | `Vec<String>` | 对账期间发现的非致命问题. |

### `ResourceReconcileStatus(资源对账状态)` Values(取值)

- `Cleaned(已清理)` 表示运行时已经清理或关闭该资源.
- `Recorded(已记录)` 表示资源由观测系统保留, 并且已经写入关闭事实.
- `NotOwned(非运行时拥有)` 表示资源不归核心 runtime(运行时) 所有.
- `Failed(失败)` 表示资源对账失败, 需要在 `warnings(警告)` 中说明原因.

## Entity(实体): `ShutdownResult(关闭结果)` Extension(扩展)

`ShutdownResult(关闭结果)` 当前位于 `src/shutdown/coordinator.rs`. 本功能保留现有 `phase(阶段)`, `cause(原因)` 和 `idempotent(幂等)` 字段, 并增加 `report: Option<ShutdownPipelineReport>`.

### Validation Rules(校验规则)

- `RequestStop(请求停止)` 或进行中阶段可以返回 `report = None`.
- `Completed(已完成)` 阶段必须返回 `report = Some(...)`.
- 重复请求如果返回已完成结果, `report.idempotent(报告幂等)` 和 `ShutdownResult.idempotent(关闭结果幂等)` 必须都为 `true`.
