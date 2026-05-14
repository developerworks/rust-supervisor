# Data Model(数据模型): 运行时生命周期守卫

## RuntimeControlPlane(运行时控制面)

**Purpose(目的)**: 表示一个 `SupervisorHandle`(监督器控制句柄) 关联的控制循环生命周期.

**Fields(字段)**:

- `state`: `RuntimeControlPlaneState(运行时控制面状态)`, 表示 starting(启动中), alive(存活), shutting_down(正在关闭), completed(已完成) 或 failed(失败).
- `started_at_unix_nanos`: `u128`, 表示控制面启动时间.
- `last_observed_at_unix_nanos`: `u128`, 表示最近一次 watchdog(看门狗) 或健康查询观测时间.
- `exit_report`: `Option<RuntimeExitReport>`, 表示最终退出结果. 只有 completed(已完成) 或 failed(失败) 状态可以存在该值.
- `shutdown_requested_by`: `Option<String>`, 表示主动关闭控制面的请求者.
- `shutdown_reason`: `Option<String>`, 表示主动关闭控制面的原因.

**Validation Rules(校验规则)**:

- alive(存活) 状态不得包含 `exit_report`.
- completed(已完成) 和 failed(失败) 状态必须包含 `exit_report`.
- `shutdown_requested_by` 和 `shutdown_reason` 如果存在, 必须是非空文本.
- `last_observed_at_unix_nanos` 必须大于或等于 `started_at_unix_nanos`.

**State Transitions(状态转换)**:

```text
starting -> alive
alive -> shutting_down
alive -> failed
shutting_down -> completed
shutting_down -> failed
completed -> completed
failed -> failed
```

completed(已完成) 和 failed(失败) 是最终态. 重复 `join` 必须返回同一个最终态.

## RuntimeControlPlaneState(运行时控制面状态)

**Purpose(目的)**: 给健康查询和诊断事件提供稳定低基数字段.

**Values(取值)**:

- `Starting(启动中)`: 控制循环任务已经创建, 但尚未确认开始接收消息.
- `Alive(存活)`: 控制循环正在接收 runtime command(运行时命令).
- `ShuttingDown(正在关闭)`: 控制面已经收到显式 shutdown(关闭) 请求.
- `Completed(已完成)`: 控制循环已经正常返回.
- `Failed(失败)`: 控制循环异常退出或 panic(恐慌).

## RuntimeHealthReport(运行时健康报告)

**Purpose(目的)**: 表示调用者从 `SupervisorHandle::health` 读取到的控制面健康状态.

**Fields(字段)**:

- `alive`: `bool`, 表示控制循环是否仍可接收命令.
- `state`: `RuntimeControlPlaneState(运行时控制面状态)`.
- `started_at_unix_nanos`: `u128`, 表示控制面启动时间.
- `last_observed_at_unix_nanos`: `u128`, 表示最近观测时间.
- `failure`: `Option<RuntimeFailureReason>`, 表示非存活状态下的结构化失败原因.
- `exit_report`: `Option<RuntimeExitReport>`, 表示最终态报告.

**Validation Rules(校验规则)**:

- 当 `alive=true` 时, `state` 必须是 alive(存活), 并且 `failure` 必须为空.
- 当 `alive=false` 且 `state=failed(失败)` 时, `failure` 必须存在.
- 当 `state` 是 completed(已完成) 或 failed(失败) 时, `exit_report` 必须存在.

## RuntimeExitReport(运行时退出报告)

**Purpose(目的)**: 表示 `join` 和 watchdog(看门狗) 观察到的最终控制循环结果.

**Fields(字段)**:

- `state`: `RuntimeControlPlaneState(运行时控制面状态)`, 只能是 completed(已完成) 或 failed(失败).
- `phase`: `String`, 表示退出发生阶段, 例如 `startup`, `message_loop`, `shutdown` 或 `watchdog`.
- `reason`: `String`, 表示人可读原因.
- `recoverable`: `bool`, 表示调用方是否可以通过重新创建 Supervisor(监督器) 恢复.
- `completed_at_unix_nanos`: `u128`, 表示最终态写入时间.

**Validation Rules(校验规则)**:

- `phase` 和 `reason` 必须是非空文本.
- failed(失败) 状态必须保留真实原因, 不得只写 generic failure(泛化失败).
- completed(已完成) 状态的 `recoverable` 必须为 false(否), 因为它不是故障恢复语义.

## RuntimeFailureReason(运行时失败原因)

**Purpose(目的)**: 表示健康报告中的结构化失败.

**Fields(字段)**:

- `phase`: `String`, 表示失败阶段.
- `reason`: `String`, 表示失败原因.
- `panic`: `bool`, 表示是否来自 panic(恐慌).
- `recoverable`: `bool`, 表示是否可通过重新创建 Supervisor(监督器) 恢复.

## RuntimeWatchdog(运行时看门狗)

**Purpose(目的)**: 观察控制循环 `JoinHandle(任务句柄)`, 并把一次性退出结果写入可重复读取的生命周期状态.

**Fields(字段)**:

- `control_loop_join`: `JoinHandle(任务句柄)`, 表示正在被观察的控制循环任务.
- `control_plane`: `RuntimeControlPlane(运行时控制面)` 的共享句柄.
- `event_sink`: typed event(类型化事件) 发布入口.
- `metrics_sink`: metrics(指标) 发布入口.
- `audit_sink`: audit log(审计日志) 发布入口.

**Validation Rules(校验规则)**:

- watchdog(看门狗) 必须只写入一次最终退出报告.
- watchdog(看门狗) 即使事件发布失败, 也必须保留健康状态中的失败原因.
- watchdog(看门狗) 不得自动重启控制循环.
