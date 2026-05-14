# Contract(契约): 真实关闭流水线

本契约描述 `ShutdownTree(关闭监督树)` 完成后调用者, dashboard(仪表盘), journal(日志) 和 metrics(指标) 可以观察到的结构化结果. 本契约不是网络协议, 它是 Rust public API(Rust 公开接口) 和 runtime diagnostics(运行时诊断) 的边界约定.

## Public API(公开接口)

### `ControlCommand::ShutdownTree(关闭监督树命令)`

`ControlCommand(控制命令)` 不新增变体, 也不改变调用语义.

```rust
ControlCommand::ShutdownTree {
    meta: CommandMeta,
}
```

### `CommandResult::Shutdown(关闭命令结果)`

`CommandResult::Shutdown(关闭命令结果)` 继续返回 `ShutdownResult(关闭结果)`. `ShutdownResult(关闭结果)` 必须包含真实关闭流水线的可选报告.

```rust
pub enum CommandResult {
    Shutdown {
        result: ShutdownResult,
    },
}
```

### `ShutdownResult(关闭结果)`

```rust
pub struct ShutdownResult {
    pub phase: ShutdownPhase,
    pub cause: ShutdownCause,
    pub idempotent: bool,
    pub report: Option<ShutdownPipelineReport>,
}
```

Contract rules(契约规则):

- 当 `phase` 是 `Completed(已完成)` 时, `report` 必须是 `Some(有值)`.
- 当请求只是创建或复用进行中的关闭流水线时, `report` 可以是 `None(无值)`.
- 重复请求返回已完成结果时, `idempotent` 必须为 `true`.

## New Runtime Model(新增运行时模型)

这些类型属于 `src/runtime/shutdown_pipeline.rs`.

```rust
pub struct ShutdownPipelineReport {
    pub cause: ShutdownCause,
    pub started_at_unix_nanos: u128,
    pub completed_at_unix_nanos: u128,
    pub phase: ShutdownPhase,
    pub outcomes: Vec<ChildShutdownOutcome>,
    pub reconcile: ShutdownReconcileReport,
    pub idempotent: bool,
}

pub struct ChildShutdownOutcome {
    pub child_id: ChildId,
    pub path: SupervisorPath,
    pub generation: Generation,
    pub attempt: Attempt,
    pub status: ChildShutdownStatus,
    pub cancel_delivered: bool,
    pub exit: Option<TaskExit>,
    pub phase: ShutdownPhase,
    pub reason: String,
}

pub enum ChildShutdownStatus {
    AlreadyExited,
    Graceful,
    Aborted,
    AbortFailed,
    LateReport,
}

pub struct ShutdownReconcileReport {
    pub registry_status: ResourceReconcileStatus,
    pub runtime_handle_status: ResourceReconcileStatus,
    pub journal_status: ResourceReconcileStatus,
    pub metrics_status: ResourceReconcileStatus,
    pub socket_status: ResourceReconcileStatus,
    pub warnings: Vec<String>,
}

pub enum ResourceReconcileStatus {
    Cleaned,
    Recorded,
    NotOwned,
    Failed,
}
```

Contract rules(契约规则):

- `outcomes(结果集合)` 必须覆盖 supervisor tree(监督树) 的全部声明 child(子任务).
- 同一个 child(子任务) 在 `outcomes(结果集合)` 中最多出现一次.
- `status = Graceful(优雅完成)` 时, `cancel_delivered` 必须是 `true`.
- `status = AlreadyExited(已经退出)` 时, 运行时不得再次发送取消.
- `status = Aborted(已强制中止)` 时, 运行时必须已经请求 `abort(强制中止)`.
- `socket_status(套接字状态)` 在核心 runtime(运行时) 中可以是 `NotOwned(非运行时拥有)`, 因为 dashboard IPC socket(仪表盘进程间通信套接字) 不由核心 runtime(运行时) 直接持有.

## Runtime Message Contract(运行时消息契约)

本功能不改变 `RuntimeLoopMessage(运行时循环消息)` 的公开入口. 子任务退出仍然通过 `RuntimeLoopMessage::ChildAttempt(子任务尝试消息)` 进入 control loop(控制循环).

Shutdown pipeline(关闭流水线) 需要保证:

- 正常 child exit(子任务退出) 和关闭等待不能重复消费同一个 completion(完成结果).
- 关闭期间到达的 child exit(子任务退出) 必须归并到对应 `ChildShutdownOutcome(子任务关闭结果)`.
- 关闭完成后的迟到报告必须记录为 `LateReport(迟到报告)` 或被忽略前产生可观察诊断.

## Event Contract(事件契约)

`src/event/payload.rs` 必须保留已有 shutdown(关闭) 事件, 并补齐 per-child(逐子任务) 关闭事实.

Required events(必需事件):

- `ShutdownRequested(关闭已请求)` 必须包含 `cause(原因)`.
- `ShutdownPhaseChanged(关闭阶段变化)` 必须包含 `from(原阶段)` 和 `to(新阶段)`.
- `ShutdownCompleted(关闭完成)` 必须包含完整摘要或摘要标识.
- `ChildShutdownCancelDelivered(子任务取消已送达)` 必须包含 `child_id(子任务标识)`, `generation(代际)` 和 `attempt(尝试)`.
- `ChildShutdownGraceful(子任务优雅完成)` 必须包含 child(子任务) 和 exit(退出) 分类.
- `ChildShutdownAborted(子任务已强制中止)` 必须包含 child(子任务), phase(阶段) 和 reason(原因).
- `ChildShutdownLateReport(子任务迟到报告)` 必须包含 child(子任务) 和原始 exit(退出) 分类.

## Metrics Contract(指标契约)

`src/observe/metrics.rs` 必须暴露 low-cardinality(低基数) 指标, 并避免把 `child_id(子任务标识)` 放入不受控 label(标签).

Required metrics(必需指标):

- `shutdown_duration_seconds(关闭耗时秒数)` 必须记录完整流水线耗时.
- `shutdown_child_outcomes_total(子任务关闭结果总数)` 必须按 `status(状态)` 和 `phase(阶段)` 计数.
- `shutdown_abort_total(关闭强制中止总数)` 必须按 `reason(原因分类)` 计数.
- `shutdown_late_reports_total(关闭迟到报告总数)` 必须按 `phase(阶段)` 计数.

## Audit Contract(审计契约)

`src/observe/pipeline.rs` 或相邻观测边界必须记录下列 audit(审计) 事实:

- 关闭请求被接受的时间, caller(调用者) 和 reason(原因).
- 每个 child(子任务) 的取消送达结果.
- graceful drain(优雅排空) 的等待顺序.
- abort stragglers(强制中止滞留任务) 的任务集合和结果.
- reconcile(对账) 后每类资源的状态.

## Dashboard Contract(仪表盘契约)

dashboard protocol(仪表盘协议) 的控制命令形状不改变. `dashboard_protocol_shape_test` 必须继续证明 `ShutdownTree(关闭监督树)` 的请求字段没有漂移.

dashboard(仪表盘) 可以通过现有 command result(命令结果), runtime state(运行时状态) 或 event stream(事件流) 观察以下信息:

- shutdown phase(关闭阶段) 当前值.
- 每个 child(子任务) 的关闭状态.
- 关闭完成时的 reconcile report(对账报告).

本功能不要求新增 dashboard route(仪表盘路由), 但是不允许破坏已有控制协议.

## Compatibility(兼容性)

本功能禁止 compatibility exports(兼容导出). 不新增旧类型别名. 不重新导出内部类型. 调用者必须使用新增的真实类型路径.
