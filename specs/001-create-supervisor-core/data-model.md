# Data Model(数据模型): 创建监督器核心

## Entity Overview(实体概览)

模型分为 declarative configuration(声明式配置)、runtime state(运行时状态)、policy decision(策略决定)、control-plane command(控制平面命令)、state snapshot(状态快照) 和 lifecycle event(生命周期事件)。

## Declarative Entities(声明式实体)

### SupervisorSpec(监督器规格)

- `id`: 稳定 supervisor(监督器) 标识。
- `path`: supervisor(监督器) 范围的稳定 `SupervisorPath`(监督器路径)。
- `strategy`: `SupervisionStrategy`(监督策略)。
- `children`: 有序 child(子任务) 定义。
- `meltdown_policy`: supervisor-level fuse(监督器级熔断)。
- `default_restart_policy`: child(子任务) 默认重启策略。
- `default_backoff_policy`: child(子任务) 默认退避策略。
- `default_health_policy`: 默认 heartbeat/stale(心跳和过期) 策略。
- `default_shutdown_policy`: 默认 two-phase shutdown(两阶段关闭) 策略。

**Validation rules(校验规则)**:

- `path` 必须从 `/root` 开始。
- 同一个 supervisor(监督器) 内的 child id(子任务标识) 必须唯一。
- child(子任务) 顺序必须稳定，因为 `RestForOne`(从失败处开始) 依赖定义顺序。

### ChildSpec(子任务规格)

- `id`: 父级内唯一的稳定 `ChildId`(子任务标识)。
- `name`: 便于阅读的 child(子任务) 名称。
- `kind`: `Worker`(工作任务) 或 `Supervisor`(监督器)。
- `factory`: worker(工作任务) 的 task factory(任务工厂)，或嵌套 supervisor spec(监督器规格)。
- `restart_policy`: `Permanent`(永久)、`Transient`(瞬时) 或 `Temporary`(临时)。
- `shutdown_policy`: graceful timeout(优雅关闭超时) 和 abort wait(强制终止等待)。
- `health_policy`: heartbeat interval(心跳间隔) 和 stale-after threshold(过期阈值)。
- `backoff_policy`: base delay(基础延迟)、max delay(最大延迟)、jitter(抖动) 和 reset-after(稳定后重置)。
- `dependencies`: 必须先启动的路径或标识。
- `tags`: 低基数标签，用于筛选。
- `criticality`: `Critical`(关键) 或 `Degraded`(可降级)。

**Validation rules(校验规则)**:

- `id` 必须可以用于路径，并且不能为空。
- `name` 必须不能为空。
- `dependencies` 必须引用同一棵树中可用的 sibling(同级) 或 ancestor(祖先)。
- `jitter`(抖动) 在测试中必须可以关闭或确定。

### TaskFactory(任务工厂)

- 每个 generation/attempt(代次和尝试) 构建一个新的 task future(任务异步值)。
- 接收 `TaskContext`(任务上下文)。
- 不把跨重启持久任务状态存进 supervisor(监督器)。

### TaskContext(任务上下文)

- `child_id`
- `path`
- `generation`
- `attempt`
- `cancel`
- `events`
- `heartbeat`

**Validation rules(校验规则)**:

- 新 attempt(尝试) 必须获得新的 `TaskContext`(任务上下文)。
- parent cancellation(父取消) 必须取消 child token(子令牌)。
- child cancellation(子取消) 不得取消 parent token(父令牌)。

## Runtime Entities(运行时实体)

### ChildRuntime(子任务运行态)

- `spec`: 关联的 `ChildSpec`(子任务规格)。
- `state`: 当前 `ChildState`(子任务状态)。
- `generation`: 重启 generation(代次)。
- `attempt`: 当前 generation(代次) 内的 attempt(尝试次数)。
- `heartbeat`: 最新 heartbeat(心跳) 记录。
- `join_handle`: 运行中任务的所有权句柄。
- `cancel_token`: child cancellation token(子任务取消令牌)。
- `restart_count`: 当前 child window(子任务窗口) 内的重启次数。
- `recent_failures`: 有序失败记录。
- `last_failure`: 最近一次失败，可为空。
- `last_policy_decision`: 最近一次策略决定，可为空。

### Registry(注册表)

- 把 `SupervisorPath`(监督器路径) 映射到 child spec(子任务规格) 和 runtime(运行态)。
- 维护定义顺序。
- 支持 add(添加)、remove(移除)、query(查询)、pause(暂停)、resume(恢复)、quarantine(隔离) 和 shutdown(关闭) 查找。

### SupervisorRuntime(监督器运行时)

- 在一个 supervisor(监督器) 范围内拥有 registry(注册表)、control loop(控制循环)、child runner(子任务运行器)、policy engine(策略引擎)、event bus(事件总线)、snapshot store(快照存储) 和 shutdown coordinator(关闭协调器)。

## Policy Entities(策略实体)

### SupervisionStrategy(监督策略)

- `OneForOne`: 只重启失败 child(子任务)。
- `OneForAll`: 停止范围内所有 child(子任务)，再按定义顺序重启。
- `RestForOne`: 停止失败 child(子任务) 和其后 child(子任务)，再按定义顺序重启。

### RestartPolicy(重启策略)

- `Permanent`: 正常退出或异常退出后都重启。
- `Transient`: 异常退出、panic(恐慌)、timeout(超时) 或 unhealthy(不健康) 后重启。
- `Temporary`: 永不重启。

### BackoffPolicy(退避策略)

- `initial`: 默认 100ms(毫秒)。
- `max`: 默认 5s(秒)。
- `jitter`: 默认 10%。
- `reset_after`: 默认 60s(秒)。
- `test_jitter`: 测试中关闭或使用确定性来源。

### MeltdownPolicy(熔断策略)

- `child_max_restarts`: 默认 10。
- `child_window`: 默认 60s(秒)。
- `supervisor_max_failures`: 默认 30。
- `supervisor_window`: 默认 60s(秒)。
- `reset_after`: 稳定运行后清除计数器。

### RestartDecision(重启决策)

- `DoNotRestart`(不重启)
- `RestartAfter(duration)`(延迟后重启)
- `Quarantine`(隔离)
- `EscalateToParent`(升级到父级)
- `ShutdownTree`(关闭整棵树)

## State Machines(状态机)

### ChildState(子任务状态)

```text
Declared
  -> Starting
  -> Running
  -> Ready
  -> Restarting
  -> Paused
  -> Quarantined
  -> ShuttingDown
  -> Stopped
  -> Failed
```

**Rules(规则)**:

- `Quarantined`(已隔离)、`Stopped`(已停止) 和 `Failed`(已失败) 对自动重启来说是 terminal(终态)。
- 手动控制命令可以把 `Paused`(已暂停) 通过 `resume`(恢复) 移回 `Running`(运行中)。
- `Restarting`(正在重启) 必须拥有策略决定和可选 backoff(退避)。

### TaskExit(任务退出)

- `Completed`(已完成)
- `Failed(TaskFailure)`(已失败)
- `Cancelled`(已取消)
- `TimedOut`(已超时)
- `Unhealthy`(不健康)
- `Panicked`(已恐慌)

### TaskFailureKind(任务失败类别)

- `Recoverable`(可恢复)
- `FatalConfig`(致命配置错误)
- `FatalBug`(致命代码错误)
- `ExternalDependency`(外部依赖错误)
- `Timeout`(超时)
- `Panic`(恐慌)
- `Cancelled`(已取消)

## Control Plane(控制平面)

### ControlCommand(控制命令)

- `command_id`
- `requested_by`
- `reason`
- `target_path`
- `accepted_at`
- `kind`
- `result`

**Command kinds(命令种类)**:

- `add_child`
- `remove_child`
- `restart_child`
- `pause_child`
- `resume_child`
- `quarantine_child`
- `shutdown_tree`
- `snapshot`
- `subscribe_events`

**Rules(规则)**:

- 命令必须幂等。
- 每个已接受命令都必须产生 audit event(审计事件)。

## State Plane(状态平面)

### SupervisorSnapshot(监督器快照)

- `root_path`
- `generated_at`
- `sequence`
- `children`: 以路径为索引的 child snapshot(子任务快照)。
- `meltdown_state`
- `shutdown_state`

### ChildSnapshot(子任务快照)

- `path`
- `id`
- `name`
- `state`
- `health`
- `generation`
- `attempt`
- `restart_count`
- `last_failure`
- `last_event_sequence`
- `last_policy_decision`

## Event Plane(事件平面)

### SupervisorEvent(监督器事件)

- `when`: `EventTime`(事件时间)。
- `where`: `EventLocation`(事件位置)。
- `what`: `EventPayload`(事件内容)。
- `policy`: 可选 `PolicyDecision`(策略决定)。
- `sequence`: monotonic event sequence(单调事件序号)。
- `correlation_id`: command(命令) 或 attempt(尝试) 的 correlation id(关联标识)。

### EventTime(事件时间)

- `unix_nanos`
- `monotonic_nanos`
- `supervisor_uptime_ms`
- `generation`
- `attempt`

### EventLocation(事件位置)

- `supervisor_path`
- `parent_id`
- `child_id`
- `child_name`
- `tokio_task_id`
- `host`
- `pid`
- `thread_name`
- `module_path`
- `source_file`
- `source_line`

### EventPayload(事件内容)

- `ChildStarting`
- `ChildRunning`
- `ChildReady`
- `ChildHeartbeat`
- `ChildFailed`
- `ChildPanicked`
- `BackoffScheduled`
- `ChildRestarting`
- `ChildRestarted`
- `ChildQuarantined`
- `ChildStopped`
- `ChildUnhealthy`
- `Meltdown`
- `ShutdownRequested`
- `ShutdownCompleted`
- `CommandAccepted`
- `CommandCompleted`
- `SubscriberLagged`

## Metrics(指标)

必需指标名如下：

- `supervisor_restart_total`
- `supervisor_child_state`
- `supervisor_child_uptime_seconds`
- `supervisor_backoff_seconds`
- `supervisor_healthcheck_latency_seconds`
- `supervisor_meltdown_total`
- `supervisor_shutdown_duration_seconds`
- `supervisor_event_lag_total`

## Relationships(关系)

- `SupervisorSpec`(监督器规格) 拥有有序 `ChildSpec`(子任务规格) 值。
- `ChildSpec`(子任务规格) 启动后变成 `ChildRuntime`(子任务运行态)。
- `TaskFactory`(任务工厂) 使用 `TaskContext`(任务上下文) 构建 attempt(尝试)。
- `ChildRuntime`(子任务运行态) 发送 `SupervisorEvent`(监督器事件)，并更新 `SupervisorSnapshot`(监督器快照)。
- `PolicyEngine`(策略引擎) 读取 `TaskExit`(任务退出)、`TaskFailureKind`(任务失败类别)、策略和计数器，并生成 `RestartDecision`(重启决策)。
- `ControlCommand`(控制命令) 产生 audit event(审计事件)，并可以改变 registry(注册表) 或 shutdown state(关闭状态)。
