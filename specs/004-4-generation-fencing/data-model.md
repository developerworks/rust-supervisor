# Data Model(数据模型): 代次隔离重启

## Entity(实体): `GenerationFenceState(代次隔离状态)`

`GenerationFenceState(代次隔离状态)` 表示一个 `ChildRuntimeState(子任务运行状态记录)` 当前是否允许启动新 attempt(尝试). 它不是时间戳, 也不是 `UNIX_EPOCH(Unix 纪元常量)`. 它只表达同一个 child(子任务) 跨重启的新旧代次边界.

### Fields(字段)

| Field(字段) | Type(类型) | Description(说明) |
|-------------|------------|-------------------|
| `phase` | `GenerationFencePhase` | 当前隔离阶段. |
| `active_generation` | `Option<Generation>` | 当前活动尝试所属 generation(代次), 无活动尝试时为 `None(无值)`. |
| `active_attempt` | `Option<ChildStartCount>` | 当前活动 attempt(尝试), 无活动尝试时为 `None(无值)`. |
| `pending_restart` | `Option<PendingRestart>` | 已接受但尚未完成的待重启请求. |
| `last_stale_report` | `Option<StaleAttemptReport>` | 最近一次过期报告, 用于诊断和 dashboard(仪表盘) 投影. |

### Values(取值): `GenerationFencePhase(代次隔离阶段)`

- `Open(开放)`: 没有活动尝试或活动尝试允许正常运行. 自动重启和手动重启仍需通过启动门禁.
- `WaitingForOldStop(等待旧尝试停止)`: 重启请求已经发送取消, 等待旧 attempt(尝试) 完成.
- `AbortingOld(正在中止旧尝试)`: 旧 attempt(尝试) 超过优雅等待时间, runtime(运行时) 已经请求强制中止.
- `ReadyToStart(可以启动新尝试)`: 旧 attempt(尝试) 已经确认结束, 新 generation(代次) 可以启动.
- `Closed(关闭)`: 运行状态记录已经 removed(已移除) 或 supervisor tree(监督树) 正在关闭, 重启不得继续启动.

### Validation Rules(校验规则)

- `phase = WaitingForOldStop(等待旧尝试停止)` 或 `phase = AbortingOld(正在中止旧尝试)` 时, `pending_restart(待重启请求)` 必须为 `Some(有值)`.
- `pending_restart.old_generation(待重启旧代次)` 必须等于发起重启时的 `active_generation(活动代次)`.
- `pending_restart.old_attempt(待重启旧尝试)` 必须等于发起重启时的 `active_attempt(活动尝试)`.
- `phase = ReadyToStart(可以启动新尝试)` 时, 当前 `ChildRuntimeState(子任务运行状态记录)` 不得持有旧 attempt(尝试) 的 cancellation token(取消令牌), abort handle(强制中止句柄) 或 completion receiver(完成接收端).
- `phase = Closed(关闭)` 时, `spawn_child_start(派生子任务启动)` 不得启动新 attempt(尝试).

## Entity(实体): `PendingRestart(待重启请求)`

`PendingRestart(待重启请求)` 表示已经接受的 `RestartChild(重启子任务)` 命令. 它保存重启意图和旧 attempt(尝试) 的隔离身份, 直到旧 attempt(尝试) 完成或中止完成.

### Fields(字段)

| Field(字段) | Type(类型) | Description(说明) |
|-------------|------------|-------------------|
| `command_id` | `CommandId` | 原始重启命令标识. |
| `requested_by` | `String` | 发起重启的操作者. |
| `reason` | `String` | 发起重启的原因. |
| `old_generation` | `Generation` | 重启命令锁定的旧 generation(代次). |
| `old_attempt` | `ChildStartCount` | 重启命令锁定的旧 attempt(尝试). |
| `target_generation` | `Generation` | 旧 attempt(尝试) 完成后要启动的新 generation(代次). |
| `requested_at_unix_nanos` | `u128` | 重启请求被接受的时间. |
| `stop_deadline_at_unix_nanos` | `u128` | 取消送达后允许旧 attempt(尝试) 优雅退出的截止时间. |
| `abort_requested` | `bool` | runtime(运行时) 是否已经请求强制中止旧 attempt(尝试). |
| `duplicate_request_count` | `u32` | 已合并的重复重启请求数量. |

### Validation Rules(校验规则)

- `target_generation(目标代次)` 必须大于 `old_generation(旧代次)`.
- `stop_deadline_at_unix_nanos(停止截止时间)` 必须等于取消送达时间加当前有效 `ShutdownPolicy.graceful_timeout(关闭策略优雅等待时间)`.
- `duplicate_request_count(重复请求数量)` 只在重复 `RestartChild(重启子任务)` 被合并时增加, 不得导致新的 generation(代次) 分配.
- `abort_requested(已请求强制中止)` 从 `false(否)` 只能变为 `true(是)`, 不得回退.

## Entity(实体): `GenerationFenceOutcome(代次隔离结果)`

`GenerationFenceOutcome(代次隔离结果)` 是 `ChildControlResult(子任务控制结果)` 的重启专用扩展. 非重启命令可以把该字段设为 `None(无值)`.

### Fields(字段)

| Field(字段) | Type(类型) | Description(说明) |
|-------------|------------|-------------------|
| `decision` | `GenerationFenceDecision` | 本次命令对代次隔离的处理结论. |
| `old_generation` | `Option<Generation>` | 本次命令锁定或观察到的旧 generation(代次). |
| `old_attempt` | `Option<ChildStartCount>` | 本次命令锁定或观察到的旧 attempt(尝试). |
| `target_generation` | `Option<Generation>` | 本次命令预期启动的新 generation(代次). |
| `cancel_delivered` | `bool` | 本次命令是否新发送取消. |
| `abort_requested` | `bool` | 本次命令是否请求强制中止. |
| `conflict` | `Option<ChildControlFailure>` | 重启冲突或不可继续时的结构化原因. |

### Values(取值): `GenerationFenceDecision(代次隔离决定)`

- `StartedImmediately(立即启动)`: 运行状态记录没有活动尝试, 新 generation(代次) 已直接启动.
- `QueuedAfterStop(停止后启动)`: 运行状态记录存在活动尝试, 重启已经等待旧尝试停止.
- `AlreadyPending(已存在待重启)`: 运行状态记录已有待重启请求, 本次重复请求已合并.
- `BlockedByShutdown(被关闭阻止)`: supervisor tree(监督树) 正在关闭, 重启不得继续.
- `Rejected(已拒绝)`: 请求无法执行, 原因写入 `conflict(冲突原因)`.

### Validation Rules(校验规则)

- `decision = QueuedAfterStop(停止后启动)` 时, `old_generation(旧代次)`, `old_attempt(旧尝试)` 和 `target_generation(目标代次)` 必须都是 `Some(有值)`.
- `decision = AlreadyPending(已存在待重启)` 时, 结果必须引用原始 `PendingRestart(待重启请求)` 的目标代次, 不得分配新目标代次.
- `decision = Rejected(已拒绝)` 时, `conflict(冲突原因)` 必须为 `Some(有值)`.
- `cancel_delivered(取消已送达)` 只表示本次命令是否新发送取消, 不能复用 `PendingRestart(待重启请求)` 的历史事实.

## Entity(实体): `StaleAttemptReport(过期尝试报告)`

`StaleAttemptReport(过期尝试报告)` 表示旧 generation(代次) 的退出报告在当前 generation(代次) 已经变化后到达. 它必须可观察, 但不得覆盖当前运行状态.

### Fields(字段)

| Field(字段) | Type(类型) | Description(说明) |
|-------------|------------|-------------------|
| `child_id` | `ChildId` | 报告所属子任务. |
| `reported_generation` | `Generation` | 过期报告中的 generation(代次). |
| `reported_attempt` | `ChildStartCount` | 过期报告中的 attempt(尝试). |
| `current_generation` | `Option<Generation>` | 报告到达时运行状态记录中的当前 generation(代次). |
| `current_attempt` | `Option<ChildStartCount>` | 报告到达时运行状态记录中的当前 attempt(尝试). |
| `exit_kind` | `ExitKind` | 过期尝试的退出分类. |
| `handled_as` | `StaleReportHandling` | runtime(运行时) 对该报告的处理结果. |
| `observed_at_unix_nanos` | `u128` | 报告被识别为过期的时间. |

### Values(取值): `StaleReportHandling(过期报告处理)`

- `IgnoredForState(状态已忽略)`: 报告没有修改当前运行状态.
- `RecordedForAudit(审计已记录)`: 报告已经写入事件和 audit(审计).
- `CountedForMetrics(指标已计数)`: 报告已经增加过期报告指标.

### Validation Rules(校验规则)

- 过期报告不得修改当前 `generation(代次)`, `attempt(尝试)`, `status(状态)`, `operation(操作)` 或 `restart_limit(重启次数限制)`.
- `reported_generation(报告代次)` 小于 `current_generation(当前代次)` 时必须判定为过期.
- 报告不匹配 pending restart(待重启请求) 的 old `(generation, attempt)(代次和尝试)` 时必须判定为过期.
- 每个过期报告至少必须发布一次 `ChildAttemptStaleReport(子任务过期报告)` 事件.
