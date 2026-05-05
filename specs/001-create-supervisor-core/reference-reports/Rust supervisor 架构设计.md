## 问:

Rust supervisor 架构设计, 要求:

1. 工业级强度的稳定性
2. 方便使用, 可读性强
3. 基于 Tokio
4. 吸收 crate.io supervision 各家所长
5. 功能丰富, 完善
6. 高度可维护性
7. 高度可观测行(When, Where, What)

你继续补充

## 答:

可以继续补充。这里的 supervisor 不应该只是“任务挂了就重启”，而应该是一个基于 Tokio 的运行时治理层：它负责启动、停止、重启、隔离、降级、熔断、状态查询、事件记录、健康检查和关闭顺序。

现有 crate 可以这样吸收，而不是照搬。`task-supervisor` 已经覆盖了 Tokio 任务保活、异常重启、运行时 add/restart/kill/status 控制、health-check interval（健康检查间隔）、restart limits（重启次数限制）、back-off（退避）和 dead-task threshold（死任务阈值）。但是它的任务需要 `Clone`，重启时会克隆原始实例，运行中对 `&mut self` 的修改不会保留，所以你的设计应该改成 `TaskFactory（任务工厂）`，让每次重启都重新构造一个新的 future（异步任务），并且把持久状态显式放进 `Arc`、数据库或状态仓库里。([Docs.rs][1])

`ractor-supervisor` 的可借鉴点是 OTP 风格的监督树。它区分 Static Supervisor（静态监督器）、Dynamic Supervisor（动态监督器）和 Task Supervisor（任务监督器），并支持 `OneForOne`、`OneForAll`、`RestForOne` 三种策略。它还提供 `Permanent`、`Transient`、`Temporary` 三种 restart policy（重启策略），以及 meltdown logic（熔断逻辑），例如在一个时间窗口内重启次数超过上限后让 supervisor 异常关闭。你的设计应该吸收这些语义。([Docs.rs][2]) ([Docs.rs][2])

`taskvisor` 的价值在可观测性骨架。它把 `Supervisor（运行时编排器）`、`Bus（事件总线）`、`AliveTracker（存活状态追踪器）`、`SubscriberSet（订阅者集合）` 和 `Registry（任务注册表）` 分开，并且在生命周期中发出 `TaskStarting`、`TaskFailed`、`BackoffScheduled`、`TaskStopped` 等事件。你的设计应该采用这个分层，因为它天然适合回答 When、Where、What。([Docs.rs][3]) ([Docs.rs][3])

`tokio-graceful-shutdown` 的可借鉴点是关闭协议。它支持子系统嵌套、SIGINT/SIGTERM/Ctrl-C 信号处理、子系统 failure（失败）或 panic（恐慌）触发自动关闭、带 timeout（超时）的清理流程、错误传播和局部关闭。你的 supervisor 必须把“关闭”设计成一等功能，而不是最后调用 `abort()`。([Docs.rs][4]) ([Docs.rs][4])

`task_scope` 的可借鉴点是 Structured Concurrency（结构化并发）。它强调子任务应该存在于父任务的作用域里，父任务结束时不应该留下孤儿任务。Tokio 的 `JoinSet（任务集合）` 也适合作为底层任务集合，因为它可以等待一组任务完成，并且 `JoinSet` 被 drop 时会 abort 其中的任务。你的设计应该把“无孤儿任务”写进验收标准。([Docs.rs][5]) ([Docs.rs][6]) ([Docs.rs][6])

`supertrees` 可以借鉴 supervision tree（监督树）、worker（工作任务）、restart policy（重启策略）和 backoff policy（退避策略）的概念，但它自己的文档明确说当前状态属于 experimental（实验性），并且缺少 monitoring（监控）、tracing（追踪）和 distributed messaging（分布式消息），所以它更适合作为参考，不适合作为工业级核心依赖。([Docs.rs][7])

建议继续补充这些硬性要求。

第 8 条，必须支持声明式 `ChildSpec（子任务规格）`。每个任务都应该通过配置声明 `id`、`name`、`kind`、`restart_policy`、`shutdown_policy`、`health_policy`、`backoff_policy`、`dependencies`、`tags` 和 `criticality`。业务代码不应该到处散落 `tokio::spawn`。

第 9 条，必须支持 `SupervisorTree（监督树）`。根 supervisor 可以包含子 supervisor 和 worker。worker 负责真实业务任务，supervisor 负责任务治理。树结构必须有稳定路径，例如 `/root/market/binance_ws`，这样日志、指标和事件才能明确定位。

第 10 条，必须支持三种 supervision strategy（监督策略）。`OneForOne` 表示只重启失败的子任务；`OneForAll` 表示一个失败，整组停止后重启；`RestForOne` 表示失败任务以及它后面按定义顺序启动的任务一起重启。这三种策略应该成为核心枚举，而不是写死在业务代码里。

第 11 条，必须支持三种 restart policy（重启策略）。`Permanent` 表示无论正常退出还是异常退出都重启；`Transient` 表示只有异常退出、panic、timeout 或 unhealthy 才重启；`Temporary` 表示永不重启。这个语义要和 `ExitReason（退出原因）` 分开，否则后面很难维护。

第 12 条，必须支持熔断。熔断不应该只在全局发生，而应该同时存在 child-level fuse（子任务级熔断）和 supervisor-level fuse（监督器级熔断）。例如同一个 child 在 60 秒内重启超过 10 次后进入 `Quarantined（隔离）` 状态；同一个 supervisor 在 60 秒内发生超过 30 次子任务失败后向父 supervisor 上报 `Meltdown（熔断）`。

第 13 条，必须支持退避策略。默认策略应该是 exponential backoff with jitter（带随机抖动的指数退避），例如初始 100ms，最大 5s，jitter 为 10%，`reset_after` 为 60s。测试环境必须允许关闭 jitter，否则单元测试无法稳定断言。

第 14 条,必须支持健康检查.健康检查不应该只看任务是否还在运行,因为一个任务可能活着但已经卡死.每个任务应该通过 `TaskContext` 定期发送 heartbeat(心跳),supervisor 根据 `heartbeat_interval` 和 `stale_after` 判断任务是否 unhealthy(不健康).

第 15 条，必须支持两阶段关闭。第一阶段发送 `CancellationToken（取消令牌）` 并等待 graceful timeout（优雅关闭超时）；第二阶段才调用 abort。`CancellationToken` 支持 child token（子取消令牌），父 token 取消会取消子 token，子 token 取消不会反向取消父 token，这适合监督树的关闭传播。([Docs.rs][8])

第 16 条,必须支持运行时控制平面.`SupervisorHandle` 应该提供 `add_child`,`remove_child`,`restart_child`,`pause_child`,`resume_child`,`quarantine_child`,`shutdown_tree`,`current_state` 和 `subscribe_events`.这些命令必须是幂等的,例如对已经停止的任务重复 shutdown 应该返回当前状态,而不是报不可恢复错误.

第 17 条，必须支持状态平面。`watch channel（观察通道）` 适合保存最新状态，因为 Tokio 的 `watch` 只保留最后一次发送的值，接收者可以等待新值变化；完整生命周期事件则应该使用 `broadcast` 或自定义 event bus（事件总线），因为状态快照和事件流不是同一个东西。([Docs.rs][9])

第 18 条，必须支持 typed error（类型化错误）。不要让所有任务只返回 `anyhow::Error`。supervisor 至少要区分 `Recoverable（可恢复）`、`FatalConfig（致命配置错误）`、`FatalBug（致命代码错误）`、`ExternalDependency（外部依赖错误）`、`Timeout（超时）`、`Panic（恐慌）` 和 `Cancelled（已取消）`。策略引擎必须根据错误类别做决定。

第 19 条，必须支持 `When、Where、What` 事件模型。`When` 记录 wall time（墙钟时间）、monotonic time（单调时间）、sequence（事件序号）、attempt（第几次尝试）和 generation（第几代任务）。`Where` 记录 supervisor path、child id、parent id、task name、tokio task id、host、pid、thread name 和注册位置。`What` 记录事件类型、状态变化、退出原因、错误类别、重启决策、退避时间、健康状态和触发命令。

第 20 条，必须用 `tracing（结构化追踪）` 作为基础。`tracing` 的 span（时间范围）可以表达一段工作从开始到结束，event（事件）可以表达某个时间点发生了什么，所以每个 child attempt（子任务尝试）都应该有自己的 span，每次状态迁移都应该发一个 event。([Docs.rs][10]) ([Docs.rs][10])

第 21 条，必须支持指标导出。最小指标集合应该包括 `supervisor_restart_total`、`supervisor_child_state`、`supervisor_child_uptime_seconds`、`supervisor_backoff_seconds`、`supervisor_healthcheck_latency_seconds`、`supervisor_meltdown_total`、`supervisor_shutdown_duration_seconds` 和 `supervisor_event_lag_total`。

第 22 条，必须把 supervisor 从业务热路径移开。高频交易、盘口处理和撮合逻辑不应该每条消息都经过 supervisor。supervisor 只管理生命周期、健康状态、控制命令和低频事件。否则 supervisor 本身会变成延迟源。

第 23 条，必须支持测试时间。退避、超时、心跳、熔断窗口都应该基于 Tokio time（Tokio 时间），并且测试中可以使用 paused time（暂停时间）推进。这样可以在毫秒内测试 60 秒熔断窗口。

第 24 条，必须支持审计日志。每个 control command（控制命令）都必须产生一条 command event（命令事件），并带上 `command_id`、`requested_by`、`reason`、`target_path`、`accepted_at` 和 `result`。生产环境排查事故时，必须能知道是谁在什么时候重启了哪个任务。

推荐的目录结构如下：

```text
src/supervisor/
  mod.rs
  spec.rs          # ChildSpec、SupervisorSpec、声明式配置
  id.rs            # ChildId、SupervisorPath、Generation、Attempt
  task.rs          # TaskFactory,TaskContext,TaskResult
  runtime.rs       # Tokio 运行时绑定，JoinSet、spawn、join、abort
  child_runner.rs  # 单个 child 的生命周期循环
  tree.rs          # 监督树，父子关系，启动和停止顺序
  policy.rs        # RestartPolicy、BackoffPolicy、MeltdownPolicy
  health.rs        # heartbeat、health check、stale 检测
  control.rs       # SupervisorHandle、ControlCommand
  registry.rs      # 当前运行状态，ChildRuntime
  event.rs         # SupervisorEvent，When/Where/What
  state.rs         # SupervisorState, watch 当前状态输出
  observe.rs       # tracing、metrics、subscriber
  shutdown.rs      # 两阶段关闭，grace/abort
  error.rs         # SupervisorError、TaskFailureKind
  test_support.rs  # paused time、fake task、事件断言工具
```

核心接口可以这样定型：

```rust
use std::{future::Future, pin::Pin, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;

pub type BoxTaskFuture = Pin<Box<dyn Future<Output = TaskResult> + Send + 'static>>;

pub trait TaskFactory: Send + Sync + 'static {
    fn build(&self, ctx: TaskContext) -> BoxTaskFuture;
}

#[derive(Clone)]
pub struct TaskContext {
    pub child_id: ChildId,
    pub path: SupervisorPath,
    pub generation: Generation,
    pub attempt: Attempt,
    pub cancel: CancellationToken,
    pub events: EventSink,
    pub heartbeat: Heartbeat,
}

pub struct ChildSpec {
    pub id: ChildId,
    pub name: String,
    pub factory: Arc<dyn TaskFactory>,
    pub restart: RestartPolicy,
    pub backoff: BackoffPolicy,
    pub health: HealthPolicy,
    pub shutdown: ShutdownPolicy,
    pub criticality: Criticality,
    pub tags: Vec<String>,
}

pub enum RestartPolicy {
    Permanent,
    Transient,
    Temporary,
}

pub enum SupervisionStrategy {
    OneForOne,
    OneForAll,
    RestForOne,
}

pub enum TaskExit {
    Completed,
    Failed(TaskFailure),
    Cancelled,
}

pub enum RestartDecision {
    DoNotRestart,
    RestartAfter(Duration),
    Quarantine,
    EscalateToParent,
    ShutdownTree,
}
```

事件结构建议从第一天就固定下来，因为后面日志、指标、告警、回放、测试都会依赖它。

```rust
pub struct SupervisorEvent {
    pub when: EventTime,
    pub where_: EventLocation,
    pub what: EventPayload,
    pub policy: Option<PolicyDecision>,
    pub sequence: u64,
    pub correlation_id: String,
}

pub struct EventTime {
    pub unix_nanos: u128,
    pub monotonic_nanos: u128,
    pub supervisor_uptime_ms: u64,
    pub generation: u64,
    pub attempt: u32,
}

pub struct EventLocation {
    pub supervisor_path: String,
    pub parent_id: Option<String>,
    pub child_id: String,
    pub child_name: String,
    pub tokio_task_id: Option<String>,
    pub module_path: Option<String>,
    pub source_file: Option<String>,
    pub source_line: Option<u32>,
}

pub enum EventPayload {
    ChildStarting,
    ChildRunning,
    ChildHeartbeat,
    ChildFailed { reason: ExitReason, error: String },
    ChildPanicked { payload_type: String },
    BackoffScheduled { delay_ms: u64 },
    ChildRestarting,
    ChildQuarantined,
    ChildStopped,
    ShutdownRequested { reason: String },
    ShutdownCompleted,
}
```

整体架构建议长这样：

```text
Application
    │
    ▼
SupervisorHandle
    │  control command: add / restart / pause / shutdown / current_state
    ▼
ControlLoop
    │
    ├── Registry
    │       └── ChildRuntime: state / generation / attempt / heartbeat / handle
    │
    ├── PolicyEngine
    │       └── restart / backoff / meltdown / escalation decision
    │
    ├── EventBus
    │       ├── tracing
    │       ├── metrics
    │       ├── audit log
    │       └── subscriber
    │
    ├── StateStore
    │       └── watch::Sender<SupervisorState>
    │
    └── ChildRunner
            ├── create TaskContext
            ├── spawn task
            ├── wait exit / panic / timeout / cancellation
            ├── classify failure
            ├── apply policy
            └── restart or stop
```

默认参数可以先定成下面这样，后面通过配置覆盖。普通网络连接类任务使用 `Transient`，`OneForOne`，初始退避 100ms，最大退避 5s，熔断窗口 60s，窗口内最多重启 10 次，心跳间隔 1s，超过 3s 没有心跳判定为 unhealthy，优雅关闭等待 5s，最后 abort 等待 1s。核心协调器类任务使用 `Permanent`，但熔断后必须向父 supervisor 升级，不应该无限重启。

验收标准也要补上。第一，任意 child panic 后，supervisor 必须记录 `ChildPanicked`、`BackoffScheduled` 和 `ChildRestarting` 三个事件，并且 attempt 必须递增。第二，同一个 child 在 60 秒内失败 11 次时必须进入 `Quarantined`，不能继续重启。第三，root shutdown 后，所有 child 的 `CancellationToken` 必须被触发，所有 `JoinSet` 必须为空。第四，`OneForAll` 策略下任意一个 child 失败时，同组所有 child 必须先停止再按定义顺序重启。第五，`RestForOne` 策略下失败 child 之前的 child 不能被重启，失败 child 以及它之后的 child 必须重启。第六，每一次状态迁移都必须产生一条包含 When、Where、What 的事件。第七，所有退避和熔断测试必须使用 Tokio paused time，不能靠真实 sleep。

最后的结论是：这个 supervisor 的核心不是“自动重启”，而是“可解释的生命周期治理”。工业级版本必须做到任务可声明、失败可分类、重启可控制、熔断可证明、关闭可等待、状态可查询、事件可回放、指标可告警、测试可复现。

[1]: https://docs.rs/task-supervisor "task_supervisor - Rust"
[2]: https://docs.rs/ractor-supervisor "ractor_supervisor - Rust"
[3]: https://docs.rs/taskvisor "taskvisor - Rust"
[4]: https://docs.rs/tokio-graceful-shutdown "tokio_graceful_shutdown - Rust"
[5]: https://docs.rs/task_scope/latest/task_scope/ "task_scope - Rust"
[6]: https://docs.rs/tokio/latest/tokio/task/struct.JoinSet.html "JoinSet in tokio::task - Rust"
[7]: https://docs.rs/supertrees "supertrees - Rust"
[8]: https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html "CancellationToken in tokio_util::sync - Rust"
[9]: https://docs.rs/tokio/latest/tokio/sync/watch/index.html "tokio::sync::watch - Rust"
[10]: https://docs.rs/tracing/latest/tracing/ "tracing - Rust"
