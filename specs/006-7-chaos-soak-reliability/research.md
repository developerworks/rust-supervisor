# Research(研究): 混沌测试与浸泡测试技术研究

**Branch(分支)**: `006-7-chaos-soak-reliability` | **Date(日期)**: 2026-05-19 | **Spec(规格)**: `specs/006-7-chaos-soak-reliability/spec.md`

## 1. 研究范围

本文件是 Phase 0(研究阶段) 输出, 解决 plan.md 中标记为 NEEDS CLARIFICATION 的技术未知点, 并记录技术选型的关键研究结论.

## 2. 关键研究结论

### 2.1 Supervisor 启动模式

**结论**: 使用 `Supervisor::start_with_policy(spec, shutdown_policy)` 作为混沌场景的 supervisor 启动标准模式.

**详细分析**:

- `Supervisor::start(spec)` 内部调用 `start_with_policy(spec, shutdown_policy_from_spec(&spec))`, 直接从 `SupervisorSpec` 派生关停策略.
- `Supervisor::start_from_config_file(path)` 从 YAML 文件加载配置, 包含 dashboard IPC 启动逻辑, 不适用于混沌测试(需避免 IPC 干扰).
- 混沌场景不需要 dashboard IPC, 因此使用 `Supervisor::start_with_policy` 传入 `ShutdownPolicy`, 避免不必要的 dashboard 初始化.

**SupervisorSpec 构造**: 现有测试模式使用 `SupervisorSpec::root(children)` 构造根 supervisor, 其中 `children: Vec<ChildSpec>`. 混沌场景需要为每个 scenario 构造不同的 child spec 集合.

### 2.2 ShutdownPolicy 与超时配置

**结论**: `ShutdownPolicy::new(graceful: Duration, abort: Duration, finish_remaining: bool)` 是关停策略的标准构造函数, 各场景应使用不同的超时值.

**详细分析**:

- `ShutdownPhase` 枚举: `Idle` -> `RequestStop` -> `GracefulDrain` -> `AbortStragglers` -> `Reconcile` -> `Completed`
- `ShutdownCoordinator` 追踪当前 phase 和 cause.
- 混沌场景如 `child_block_forever` 和 `child_ignore_cancel` 需要验证关停阶段是否在 `graceful_timeout + abort_wait` 时间内完成, 因此需要较短的 graceful 超时(如 500ms)以便在测试窗口内触发 abort 路径.

### 2.3 事件系统 API

**结论**: 使用 `broadcast::channel(spec.event_channel_capacity)` 创建事件通道, 通过 `SupervisorHandle` 获取事件接收器.

**详细分析**:

- `What` 枚举包含 20+ 生命周期事件变体, 包括 `ChildPanicked`, `ChildFailed`, `BackpressureAlert`, `ShutdownPhaseChanged`, `RuntimeStarved` 等.
- 事件通过 `tokio::sync::broadcast` 通道分发, 支持多订阅者.
- 混沌场景需要订阅事件以验证故障注入是否产生了预期事件(如 `ChildPanicked` 事件计数).
- `CorrelationId` 和 `EventSequence` 可用于事件链追踪.

### 2.4 ChildSlot 与 AdmissionSet 生命周期

**结论**: 使用 `ChildSlot::new(id, path, heartbeat_timeout)` 创建子任务槽位, 使用 `AdmissionSet` 管理并发重启准入.

**详细分析**:

- `ChildSlot::new(child_id, path, Duration::from_secs(60))` 创建槽位.
- `ChildSlot` 通过 `ChildRunHandle` 与运行中的任务交互, 包含 `CancellationToken`, `AbortHandle`, 完成通知通道等.
- `AdmissionSet::try_admit(child_id, generation, start_count)` 检查并发重启冲突, 返回 `AdmissionConflict` 或允许准入.
- 混沌场景的 `rapid_failure_10k` 场景需要验证 `AdmissionSet` 在高频失败下是否按预期拒绝过多并发.

### 2.5 RestartBudget API

**结论**: 使用 `RestartBudgetTracker::try_consume(now_unix_nanos)` 检查重启预算, 返回 `BudgetVerdict::Granted | Exhausted { retry_after_ns }`.

**详细分析**:

- `RestartBudgetConfig::new(window: Duration, max_burst: u32, recovery_rate_per_sec: f64)` 配置窗口/爆发/恢复率.
- `RestartBudgetTracker::current_tokens()` 查询当前令牌数.
- `RestartBudgetTracker::window_failures()` 查询滑动窗口内失败次数.
- 混沌场景的 `rapid_failure_10k` 需要验证: 即使触发 10000 次失败, `restart_budget` 恢复率 > 0(即 `current_tokens() > 0` 最终恢复).

### 2.6 ShutdownPipeline 与关停验证

**结论**: `ShutdownCoordinator` 完成关停时 phase 为 `Completed` 且 `ShutdownResult` 包含每个 Child 的关停状态. 使用 `shutdown_tree_fanout()` 触发树形关停.

**详细分析**:

- `shutdown_tree_fanout()` 在 `src/runtime/shutdown.rs` 中实现, 遍历 supervisor 树, 依次在每个子树触发 `GracefulDrain` -> `AbortStragglers`.
- `ShutdownPipelineReport` 包含每个子任务的关停结果 `ChildShutdownOutcome`.
- `reconcile_shutdown_slots()` 验证关停后所有 slot 是否停用.
- 混沌场景的 `child_block_forever` 需要验证: 关停完成后 slot 全部停用, 无 dangling handle.

### 2.7 背压策略与事件限速

**结论**: `BackpressureConfig` 和 `BackpressureStrategy`(AlertAndBlock / SampleAndAudit) 定义在 `src/spec/supervisor.rs` 中. 订阅者限速使用 `slow_consumer_ms` 参数.

**详细分析**:

- `BackpressureStrategy::AlertAndBlock` 是 006-5 默认策略, 在事件通道满时发出告警并阻塞生产者.
- `BackpressureStrategy::SampleAndAudit` 在事件通道满时采样记录并审计, 不阻塞生产者.
- `event_gap_total` 计数器追踪 `journal` 中缺失的事件条目数.
- 混沌场景的 `slow_event_subscriber` 需要验证: 在 `slow_consumer_ms=100` 限制下, 背压策略是否与 spec 一致, `event_gap_total` 是否 ≤ `discard_budget`.

### 2.8 IPC 协议与连接速率限制

**结论**: Dashboard IPC 使用 Unix socket(套接字文件) 传输, 握手格式在 `contracts/typed-event-schema.md` 和 `specs/006-1-platform-docs-ipc-security/` 中定义. IPC 风暴防护需要实现固定窗口(1s) + 令牌桶(容量 100, 恢复率 50/s)的速率限制器.

**详细分析**:

- IPC 服务端在 `src/dashboard/runtime.rs` 中实现, 使用 `tokio::net::UnixListener`.
- 合法客户端: 握手 payload 包含 `target_id` 字段且为合法 JSON, 符合 dashboard IPC 协议.
- 劣质客户端: payload 非法 JSON 或缺失 `target_id`.
- 速率限制器 `RateLimiter` 需要实现为独立组件, 不在本切片修改生产代码, 因此放在测试夹具中模拟 IPC 场景行为.

### 2.9 时钟回拨与 monotonic clock

**结论**: 项目核心计时使用 `std::time::Instant`(monotonic clock), 不受 wall clock 回退影响. 夹具中时钟回拨只能模拟 wall clock 回退, 不能回退 `Instant`.

**详细分析**:

- `supervisor_cold_start_and_hot_loop.rs` 中使用 `std::time::Instant` 计时.
- `shutdown/coordinator.rs` 使用 `std::time::Instant` 计算关停超时.
- `policy/budget.rs` 使用 `UNIX_EPOCH` + `SystemTime::now()` 计算窗口, 但实际逻辑依赖 `Instant`.
- **限制**: Rust 的 `Instant` 在 macOS/Linux 上是 CLOCK_MONOTONIC, 无法通过软件回退. `clock_step_backward` 场景只能模拟 wall clock 回退对 `SystemTime` 和滑动窗口的影响.

**定稿方案**: 不注入时间源 trait(不修改生产代码). `FixtureClockController` 通过 `step_backward()` 记录偏移量, 场景代码在启动 supervisor 后断言滑动窗口组件的 `window_failures()` 计数不受 `SystemTime` 偏移影响(因为底层使用 `Instant`). 如果未来需要模拟实际的壁钟回退, 必须在后续切片中引入时间源 trait 并修改 `RestartBudgetTracker`/`FailureWindow` 的构造签名.

### 2.10 Tokio 运行时饥饿探测

**结论**: Tokio 的 `runtime::Metrics` 提供 `num_alive_tasks` 和 `instruments.poll_count`. 通过注入 `tokio::task::yield_now` 饥饿循环可模拟运行时饥饿.

**详细分析**:

- `tokio::runtime::Handle::current().metrics()` 可获取当前运行时指标.
- `metrics.num_alive_tasks()` 返回存活任务数.
- `metrics.instruments.poll_count()` 返回 poll 调用次数(poll_count per instrument).
- 饥饿循环可通过 `tokio::task::yield_now` 在一个循环中反复 yield 而不 await 其他任务来模拟.
- 验证方法: 在饥饿注入前后对比 `poll_count` 增量, 确认控制循环迭代仍在前进.
- **注意**: `runtime::Metrics` 需要 `tokio_unstable` cfg 标志. 如果未启用, 则降级为通过控制循环事件频率间接推断.

### 2.11 RSS 采集 API 平台差异

**结论**: macOS 和 Linux 上获取 RSS(常驻集大小) 的 API 不同. `MetricsCollector` 需要平台条件编译.

**详细分析**:

- **Linux**: 通过读取 `/proc/self/status` 中的 `VmRSS` 行获取 RSS, 单位 kB. 这是 Linux 上最轻量的方式, 不依赖外部 crate.
- **macOS**: `/proc` 不可用. 通过 `libc::proc_pidinfo`(需要 `libc` crate, 已在 `[dependencies]` 中) 获取 `proc_taskinfo` 结构体的 `pti_resident_size` 字段, 单位 bytes. Apple Silicon 上测试通过.
- **备选方案(跨平台)**: 使用 `procfs` crate 或 `sysinfo` crate, 但会增加外部依赖. 本切片原则"不新增外部 crate", 因此使用平台 `#[cfg(target_os)]` 条件编译.
- **实现策略**: `MetricsCollector` 中定义 `fn read_rss_bytes() -> Option<u64>`, 内部使用 `#[cfg(target_os = "linux")]` 读 `/proc/self/status`, `#[cfg(target_os = "macos")]` 调 `libc::proc_pidinfo`. 两种平台都不支持的返回 `None`, 浸泡报告对应行标记为 `N/A`.

### 2.12 测试隔离与 CI 集成

**结论**: 混沌场景使用 `#[ignore]` 标记, 仅 CI nightly 通过 `--include-ignored` 运行. 浸泡测试同理使用独立测试二进制 `soak_suite.rs` + `#[ignore]`.

**详细分析**:

- 现有 `Cargo.toml` 通过 `[[test]]` 条目注册测试二进制, 每个条目独立编译.
- 新增 `chaos_suite` 和 `soak_suite` 条目, 编译产出独立的测试二进制文件.
- JSON 判决书输出到 stdout, CI 通过 `grep` 或 jq 解析判定是否通过.
- 浸泡报告写入 `artifacts/validation/soak-<timestamp>.md`, 由 006-2 的 QualityGateOutcome 外链列引用.

## 3. 需要澄清的问题(全部已解决)

1. ~~SupervisorHandle 是否提供 event receiver 访问?~~ -> 通过 `SupervisorHandle` 的 `event_receiver()` 方法获取 `broadcast::Receiver<SupervisorEvent>`. 确认.
2. ~~`ShutdownPolicy` 对混沌场景的建议超时值?~~ -> `child_block_forever`: graceful=500ms, abort=500ms; 其他场景使用 spec 默认值(60s/10s).
3. ~~Tokio `runtime::Metrics` 是否需要 `tokio_unstable`?~~ -> 需要. 如果未启用 `cfg(tokio_unstable)`, `metrics()` 返回默认零值. 混沌场景使用 fallback: 通过控制循环 emit 事件的频率间接推断.

## 4. 备选方案与决策理由

| 备选方案                                      | 拒绝理由                                                |
| --------------------------------------------- | ------------------------------------------------------- |
| 将混沌测试放在 `src/` 内使用 `#[cfg(test)]`   | 违反宪章 Module Ownership: 混沌测试不应与生产代码同目录 |
| 使用 `trybuild` 或自定义 harness 运行混沌场景 | 过度工程: `cargo test` 的 `#[ignore]` 机制满足需求      |
| 引入 `proptest` 进行混沌模糊测试              | 本切片范围是特定故障波形逐一验证, 非模糊测试            |
| 使用 Python 脚本驱动混沌测试                  | 不能复用 Rust supervisor 的内部 API, 且增加语言边界     |
