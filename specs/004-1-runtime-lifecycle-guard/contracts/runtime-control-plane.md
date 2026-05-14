# Contract(契约): 运行时控制面生命周期

## Public Handle API(公共句柄接口)

本契约描述 `SupervisorHandle(监督器控制句柄)` 新增或调整的调用者可见能力. 代码标识符保持英文, 周围说明必须使用中文.

### `SupervisorHandle::is_alive`

**Purpose(目的)**: 快速判断 runtime control loop(运行时控制循环) 是否仍可接收命令.

**Input(输入)**: 无参数.

**Output(输出)**: `bool`.

**Rules(规则)**:

- 控制循环处于 alive(存活) 时返回 `true`.
- 控制循环处于 starting(启动中), shutting_down(正在关闭), completed(已完成) 或 failed(失败) 时返回 `false`.
- 本方法不得等待控制循环结束.

### `SupervisorHandle::health`

**Purpose(目的)**: 返回 `RuntimeHealthReport(运行时健康报告)`.

**Input(输入)**: 无参数.

**Output(输出)**: `RuntimeHealthReport(运行时健康报告)`.

**Rules(规则)**:

- 正常启动后必须包含 alive(存活), 控制循环状态, 启动时间和最近观测时间.
- 控制循环异常退出后必须包含 not alive(非存活), 失败阶段, 失败原因和是否可恢复.
- 本方法必须在控制循环已经结束后仍然可用.

### `SupervisorHandle::join`

**Purpose(目的)**: 等待控制面进入最终态, 并返回 `RuntimeExitReport(运行时退出报告)`.

**Input(输入)**: 无参数.

**Output(输出)**: `Result<RuntimeExitReport, SupervisorError>`.

**Rules(规则)**:

- 如果控制面仍在运行, 本方法等待最终态.
- 如果控制面已经结束, 本方法立即返回缓存的最终结果.
- 对同一个已结束运行时重复调用 10 次, 每次都必须返回相同结果.
- 本方法不得消费调用者后续读取健康状态所需的数据.

### `SupervisorHandle::shutdown`

**Purpose(目的)**: 请求 runtime control loop(运行时控制循环) 正常结束, 并返回最终结果.

**Input(输入)**:

- `requested_by`: 请求者, 必须是非空文本.
- `reason`: 原因, 必须是非空文本.

**Output(输出)**: `Result<RuntimeExitReport, SupervisorError>`.

**Rules(规则)**:

- 如果控制面正在运行, 本方法必须请求控制循环退出, 然后等待并返回最终结果.
- 如果控制面已经结束, 本方法必须返回已有最终结果, 不得挂起.
- 如果 `requested_by` 或 `reason` 为空白, 本方法必须返回结构化错误.
- 本方法只关闭控制面. 它不替代 `shutdown_tree` 的监督树关闭语义.

## Runtime Internal Message(运行时内部消息)

### `RuntimeCommand::ShutdownControlPlane`

**Purpose(目的)**: 让控制循环收到显式控制面关闭请求.

**Fields(字段)**:

- `meta`: `CommandMeta(命令元数据)`, 包含请求者和原因.
- `reply_sender`: `oneshot::Sender<Result<RuntimeExitReport, SupervisorError>>`, 返回控制面接受关闭请求的结果.

**Rules(规则)**:

- 控制循环收到该消息后必须进入 shutting_down(正在关闭) 状态.
- 控制循环必须返回 completed(已完成) 退出结果.
- 真实 child task(子任务) 关闭不在本契约范围内.

## Typed Events(类型化事件)

运行时必须通过已有 `SupervisorEvent(监督器事件)` 形状发布下列事件或语义等价事件.

| Event(事件) | Required fields(必需字段) | Purpose(目的) |
|-------------|---------------------------|---------------|
| `RuntimeControlLoopStarted` | `phase`, `started_at_unix_nanos` | 表示控制循环已经开始接收命令. |
| `RuntimeControlLoopShutdownRequested` | `requested_by`, `reason`, `command_id` | 表示操作者已经请求控制面关闭. |
| `RuntimeControlLoopCompleted` | `phase`, `reason`, `completed_at_unix_nanos` | 表示控制循环正常结束. |
| `RuntimeControlLoopFailed` | `phase`, `reason`, `panic`, `recoverable` | 表示控制循环异常退出. |
| `RuntimeControlLoopJoinCompleted` | `state`, `phase`, `reason` | 表示一次 join(等待结束) 调用已经获得最终状态. |

## Metrics(指标)

| Metric(指标) | Type(类型) | Labels(标签) | Rule(规则) |
|--------------|------------|--------------|------------|
| `supervisor_runtime_control_loop_exit_total` | counter(计数器) | `state`, `phase` | 控制循环完成或失败时增加 1. |
| `supervisor_runtime_control_plane_alive` | gauge(仪表) | `state` | 健康状态变化时写入 1 或 0. |

标签必须保持低基数. `reason` 不得作为 metrics(指标) 标签.

## Audit Log(审计日志)

控制面关闭请求和 join(等待结束) 完成必须进入 audit log(审计日志) 或语义等价的审计记录.

**Required fields(必需字段)**:

- `command_id`: 命令标识.
- `requested_by`: 请求者.
- `reason`: 原因.
- `result`: `accepted`, `completed` 或 `failed`.
- `phase`: 控制面阶段.

## Error Contract(错误契约)

- 控制循环已经结束后再发送普通控制命令, 必须返回包含已知退出原因的 `SupervisorError(监督器错误)`.
- watchdog(看门狗) 观察到 panic(恐慌) 时, 必须返回 failed(失败) 健康状态, 并在原因中保留 panic(恐慌) 类别.
- 事件发布失败不得覆盖健康状态的最终失败原因.
