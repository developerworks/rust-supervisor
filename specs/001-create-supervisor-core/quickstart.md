# Quickstart(快速开始): 创建监督器核心

本 quickstart(快速开始) 说明第一版实现应该怎样验证。

## 1. 验证项目基线

```bash
cargo fmt --check
cargo check
cargo test
```

## 2. 预期最小用法

本功能应该允许维护者描述一个 child(子任务)，并通过 supervisor(监督器) 运行它，而不是手写无人管理的后台 spawn(启动任务)。

```rust
use rust_supervisor::supervision::{
    BackoffPolicy, ChildSpec, Criticality, HealthPolicy, RestartPolicy,
    ShutdownPolicy, Supervisor, SupervisorSpec, SupervisionStrategy,
};

let child = ChildSpec::worker("binance_ws", "Binance WebSocket", factory)
    .restart_policy(RestartPolicy::Transient)
    .backoff_policy(BackoffPolicy::default_network())
    .health_policy(HealthPolicy::heartbeat())
    .shutdown_policy(ShutdownPolicy::two_phase())
    .criticality(Criticality::Degraded)
    .tag("market");

let spec = SupervisorSpec::root()
    .strategy(SupervisionStrategy::OneForOne)
    .child(child);

let handle = Supervisor::start(spec).await?;
let snapshot = handle.snapshot().await?;
handle.shutdown_tree("operator", "quickstart complete").await?;
```

实现期间 builder(构建器) 名称可以调整，但最终 API(接口) 必须保留这些契约：声明式 child spec(子任务规格)、tree spec(树规格)、runtime handle(运行时句柄)、snapshot query(快照查询) 和 two-phase shutdown(两阶段关闭)。

## 3. 必需验收测试

### Panic Restart(恐慌重启)

```bash
cargo test panic_records_restart_events
```

预期行为：

- 系统发送 `ChildPanicked` 事件。
- 系统发送 `BackoffScheduled` 事件。
- 系统发送 `ChildRestarting` 事件。
- attempt(尝试次数) 在重启后的 child(子任务) 运行前递增。

### Child Quarantine(子任务隔离)

```bash
cargo test child_quarantines_after_restart_window
```

预期行为：

- 60 秒内第 11 次重启会把 child(子任务) 置为 `Quarantined`(已隔离)。
- quarantine(隔离) 后不会继续自动重启。
- snapshot(快照) 报告隔离状态和最近策略决定。

### Supervisor Meltdown(监督器熔断)

```bash
cargo test supervisor_meltdown_escalates
```

预期行为：

- 一个 supervisor(监督器) 范围在 60 秒内第 31 次 child(子任务) 失败时，系统发送 `Meltdown`(熔断)。
- parent supervisor(父监督器) 收到 escalation(升级)。

### Two-Phase Shutdown(两阶段关闭)

```bash
cargo test root_shutdown_leaves_no_orphans
```

预期行为：

- 每个 child cancellation token(子任务取消令牌) 都被触发。
- 超时前退出的 child(子任务) 报告 graceful completion(优雅完成)。
- 未退出的 child(子任务) 在超时后被 abort(强制终止)。
- root shutdown(根关闭) 后，runtime task set(运行时任务集合) 为空。

### Supervision Strategies(监督策略)

```bash
cargo test one_for_all_restarts_group_in_order
cargo test rest_for_one_restarts_failed_and_later_children
```

预期行为：

- `OneForAll`(一对全部) 先停止所有 sibling(同级任务)，再按定义顺序重启。
- `RestForOne`(从失败处开始) 不重启失败 child(子任务) 之前定义的 child(子任务)。

### Event Shape(事件形状)

```bash
cargo test every_state_transition_has_when_where_what
```

预期行为：

- 每次状态迁移产生一条事件。
- 事件包含 `When`(何时)、`Where`(何处)、`What`(发生内容)、sequence(序号) 和 correlation id(关联标识)。

### Deterministic Time(确定性时间)

```bash
cargo test paused_time_drives_backoff_heartbeat_and_meltdown
```

预期行为：

- 测试不等待真实 60 秒窗口。
- jitter(抖动) 可以关闭，或者可以被确定性控制。

## 4. Observability Smoke Check(可观察性冒烟检查)

运行一个 example(示例) 或 integration test(集成测试)，并确认它发送这些内容：

- 每个 child attempt(子任务尝试) 一个 `tracing`(结构化追踪) span(追踪范围)。
- 每次状态迁移一个 `tracing`(结构化追踪) event(追踪事件)。
- 通过 metrics facade(指标门面) 发送必需指标。
- 每个 control command(控制命令) 都有 command audit event(命令审计事件)。

## 5. Completion Gate(完成关口)

进入实现前，以下文件必须存在：

```text
specs/001-create-supervisor-core/plan.md
specs/001-create-supervisor-core/research.md
specs/001-create-supervisor-core/data-model.md
specs/001-create-supervisor-core/contracts/public-api.md
specs/001-create-supervisor-core/quickstart.md
```

只有所有 quickstart(快速开始) 检查通过后，实现才算完成。
