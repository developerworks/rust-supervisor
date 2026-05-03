# Public API Contract(公开接口契约): 监督器核心

本契约描述本功能预期提供的 Rust library(库) 表面。名称必须由本项目拥有，不得成为参考 crate(库) 的 compatibility exports(兼容导出)。

## Module Boundaries(模块边界)

```text
rust_supervisor::supervision
├── spec
├── id
├── task
├── policy
├── health
├── shutdown
├── control
├── event
├── snapshot
└── error
```

`supervision::mod.rs` 只能 re-export(再导出) 一等项目类型。它不得 re-export(再导出) 第三方 supervisor(监督器) crate(库) 类型。

## Task Definition Contract(任务定义契约)

```rust
pub type BoxTaskFuture =
    Pin<Box<dyn Future<Output = TaskResult> + Send + 'static>>;

pub trait TaskFactory: Send + Sync + 'static {
    fn build(&self, ctx: TaskContext) -> BoxTaskFuture;
}
```

**Rules(规则)**:

- `build` 必须为每次 attempt(尝试) 创建 fresh future(新异步任务)。
- 跨重启持久状态必须放在 supervisor runtime(监督器运行时) 外部。
- `TaskResult`(任务结果) 必须区分成功、取消和类型化失败。

## Declarative Spec Contract(声明式规格契约)

`ChildSpec`(子任务规格) 必须包含：

- `id`
- `name`
- `kind`
- `factory` 或嵌套 supervisor spec(监督器规格)
- `restart_policy`
- `shutdown_policy`
- `health_policy`
- `backoff_policy`
- `dependencies`
- `tags`
- `criticality`

`SupervisorSpec`(监督器规格) 必须包含：

- `path`
- `strategy`
- `children`
- supervisor-level fuse policy(监督器级熔断策略)
- restart(重启)、backoff(退避)、health(健康) 和 shutdown(关闭) 的默认值。

## Policy Contract(策略契约)

核心枚举如下：

- `SupervisionStrategy`(监督策略): `OneForOne`(一对一)、`OneForAll`(一对全部)、`RestForOne`(从失败处开始)
- `RestartPolicy`(重启策略): `Permanent`(永久)、`Transient`(瞬时)、`Temporary`(临时)
- `RestartDecision`(重启决策): `DoNotRestart`(不重启)、`RestartAfter`(延迟后重启)、`Quarantine`(隔离)、`EscalateToParent`(升级到父级)、`ShutdownTree`(关闭整棵树)
- `TaskFailureKind`(任务失败类别): `Recoverable`(可恢复)、`FatalConfig`(致命配置错误)、`FatalBug`(致命代码错误)、`ExternalDependency`(外部依赖错误)、`Timeout`(超时)、`Panic`(恐慌)、`Cancelled`(已取消)

policy engine(策略引擎) 必须接收 typed exit(类型化退出) 和策略，并返回一个明确 decision(决定)。它不得从字符串推断决定。

## Runtime Control Contract(运行时控制契约)

`SupervisorHandle`(监督器句柄) 必须提供异步命令：

- `add_child`
- `remove_child`
- `restart_child`
- `pause_child`
- `resume_child`
- `quarantine_child`
- `shutdown_tree`
- `snapshot`
- `subscribe_events`

**Idempotency(幂等性)**:

- shutdown(关闭) 后重复 `shutdown_tree` 必须返回当前关闭结果。
- 对已暂停 child(子任务) 重复 `pause_child` 必须返回暂停状态。
- 对运行中 child(子任务) 重复 `resume_child` 必须返回运行状态。
- 对已隔离 child(子任务) 重复 `quarantine_child` 必须返回隔离状态。

## Snapshot Contract(快照契约)

`snapshot` 必须返回最新树状态，并包含：

- root path(根路径)
- generated sequence(生成序号)
- child path(子任务路径)
- child id(子任务标识) 和 name(名称)
- state(状态)
- health(健康状态)
- generation(代次)
- attempt(尝试次数)
- restart count(重启次数)
- last failure(最近失败)
- last policy decision(最近策略决定)

snapshot(快照) 回答“现在真实状态是什么”，不得被当作 lifecycle event history(生命周期事件历史)。

## Event Contract(事件契约)

每个 `SupervisorEvent`(监督器事件) 必须包含：

- `when`: wall time(墙钟时间)、monotonic time(单调时间)、supervisor uptime(监督器运行时长)、generation(代次) 和 attempt(尝试次数)。
- `where`: supervisor path(监督器路径)、parent id(父标识)、child id(子任务标识)、child name(子任务名称)、可用时的 task id(任务标识)、host(主机)、pid(进程标识)、thread name(线程名称) 和 registration location(注册位置)。
- `what`: event payload(事件内容)、state transition(状态迁移)、exit reason(退出原因)、failure category(失败类别)、restart decision(重启决策)、backoff(退避)、health(健康) 或 triggering command(触发命令)。
- `sequence`: monotonic event sequence(单调事件序号)。
- `correlation_id`: command(命令) 或 attempt(尝试) 的 correlation id(关联标识)。

每次状态迁移必须只发送一条 lifecycle event(生命周期事件)。

## Audit Contract(审计契约)

每个 control command(控制命令) 必须发送 command audit data(命令审计数据)：

- `command_id`
- `requested_by`
- `reason`
- `target_path`
- `accepted_at`
- `result`

audit event(审计事件) 必须可以通过 event stream(事件流) 读取。

## Shutdown Contract(关闭契约)

root shutdown(根关闭) 必须完成这些步骤：

1. 触发 parent cancellation(父取消)。
2. 把取消传播到 child token(子令牌)。
3. 等待 graceful timeout(优雅关闭超时)。
4. 超时后 abort(强制终止) 剩余 child(子任务)。
5. drain task ownership(排空任务所有权)，直到 supervisor(监督器) 不再拥有任何 child task(子任务)。
6. 发布 shutdown completed event(关闭完成事件)，并更新 snapshot(快照)。

## Metrics Contract(指标契约)

核心必须通过 metrics facade(指标门面) 发送这些指标名：

- `supervisor_restart_total`
- `supervisor_child_state`
- `supervisor_child_uptime_seconds`
- `supervisor_backoff_seconds`
- `supervisor_healthcheck_latency_seconds`
- `supervisor_meltdown_total`
- `supervisor_shutdown_duration_seconds`
- `supervisor_event_lag_total`

metric label(指标标签) 必须使用低基数值：supervisor path(监督器路径)、child id(子任务标识)、state(状态)、decision(决定) 和 failure category(失败类别)。标签不得包含无限增长的错误消息。

## Test Support Contract(测试支持契约)

`test_support` 必须提供这些帮助能力：

- paused time setup(暂停时间设置)
- fake task factory(假任务工厂)
- heartbeat control(心跳控制)
- event collection(事件收集)
- snapshot assertion(快照断言)
- no-orphan shutdown assertion(无孤儿任务关闭断言)
- deterministic jitter(确定性抖动)
