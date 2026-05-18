---
description: "Task list for chaos and soak test implementation"
---

# Tasks(任务): 压力故障混沌与浸泡稳定性

**Input(输入)**: 设计文档来自 `specs/006-7-chaos-soak-reliability/`
**Prerequisites(前置文档)**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests(测试)**: 本切片的主体是测试框架. 所有生产代码无变更. 混沌与浸泡测试自身作为测试代码存在.

**Organization(组织方式)**: 任务按 spec 的三个用户故事分组. 每个故事可独立测试. Phase 1(Setup) 创建目录结构和 Cargo.toml 条目, 是其他所有阶段的前置.

## Format(格式): `[ID] [P?] [Story] Description(描述)`

- **[P]**: 可以并行执行, 因为任务修改不同文件, 并且不依赖未完成任务.
- **[Story]**: 标记任务属于哪个用户故事, 例如 US1, US2, US3.
- 任务描述必须写出准确文件路径.
- 任务描述必须使用中文; 英文术语必须写成 `English(中文说明)`.
- Rust(编程语言) 项目中, 所有单元测试, 契约测试和集成测试都必须放在外部 `tests/` 目录, 不得把测试代码写入 `src/` 模块文件.
- 并行任务必须修改不同文件; 如果两个任务会修改同一个文件, 不得同时标记 `[P]`.

---

## Phase 1(阶段一): Setup(共享基础设施)

**Purpose(目的)**: 创建混沌与浸泡测试的目录结构和 Cargo.toml 条目.

- [x] T001 在 `tests/` 下创建 `tests/chaos/`, `tests/chaos/scenarios/`, `tests/chaos/fixtures/`, `tests/soak/`, `tests/soak/fixtures/` 目录结构, 与 `plan.md` 项目结构一致.
- [x] T002 在 `Cargo.toml` 中新增 `[[test]]` 条目 `chaos_suite`(指向 `tests/chaos_suite.rs`) 和 `soak_suite`(指向 `tests/soak_suite.rs`). 确认不新增外部 crate 依赖, 仅复用已有的 `serde_json`, `tokio` 等 dev-dependencies.
- [x] T003 确认 `cargo test`(排除 chaos/soak 测试) 编译通过且测试不变.

**Checkpoint(检查点)**: 目录结构和 Cargo.toml 配置完成, 新增测试二进制可编译.

---

## Phase 2(阶段二): Foundational(阻塞前置基础)

**Purpose(目的)**: JSON 判决书基础设施和测试共享夹具, 是所有用户故事的前置依赖.

- [x] T004 [P] 在 `tests/chaos/verdict.rs` 中实现 `ScenarioVerdict` 结构体(含 `scenario_id`, `semver`, `passed`, `thresholds`, `started_at_unix_nanos`, `duration_ns`, `error` 字段), `ThresholdResult` 结构体(含 `value`, `limit`, `passed` 字段), 以及 `serde::Serialize` 派生. 实现 `VerdictWriter` 输出 JSON 到 stdout.
- [x] T005 [P] 在 `tests/chaos/mod.rs` 中定义 `ChaosScenario` 枚举(11 个变体: `ChildPanicStorm`, `ChildBlockForever`, `ChildIgnoreCancel`, `RapidFailure10k`, `SlowEventSubscriber`, `CommandChannelFull`, `IpcConnectionStorm`, `SocketPathContention`, `RelayCrashLoop`, `ClockStepBackward`, `RuntimeStarvationProbe`). 实现 `scenario_id() -> &'static str` 和 `semver() -> &'static str`(从 `env!("CARGO_PKG_VERSION")` 读取).
- [x] T006 在 `tests/chaos/fixtures/child_spawner.rs` 中实现 `FixtureChildSpawner`: 一个可控的 child spawn 夹具, 支持 `with_panic_delay(delay: Duration)`, `with_block_forever()`, `with_ignore_cancel()` 配置方法, 返回 `ChildRunHandle` 用于监督器集成.
- [x] T007 在 `tests/chaos/fixtures/event_throttle.rs` 中实现 `FixtureEventThrottle`: 一个事件订阅者限速夹具, 支持 `with_slow_consumer_ms(ms: u64)` 配置, 模拟 subscriber 回调限速 100ms/event.
- [x] T008 在 `tests/chaos/fixtures/ipc_stress.rs` 中实现 `FixtureIpcStress` 基础结构: 一个 IPC 劣质连接生成器, 支持 `with_concurrent_clients(n: u32)`(默认 1000) 和 `with_legitimate_payload()`/`with_junk_payload()` 配置. **注意**: 本任务仅实现连接生成基础, `RateLimiter`(速率限制器) 和 `ClientClassification`(客户端分类) 逻辑在 T029 中扩展实现.
- [x] T009 在 `tests/chaos/fixtures/clock_controller.rs` 中实现 `FixtureClockController`: 一个模拟时间源结构体, 支持 `step_backward(duration: Duration)` 方法. 此夹具不修改真实系统时钟, 通过注入时间源 trait 或断言验证 `std::time::Instant` 的 monotonic 行为.
- [x] T010 在 `tests/chaos/fixtures/runtime_probe.rs` 中实现 `FixtureRuntimeProbe`: 一个运行时饥饿探针, 支持 `inject_starvation_loop(duration: Duration)` 方法(在 30s 内反复 `tokio::task::yield_now`). 实现 `poll_count_stalled() -> bool` 检查控制循环迭代.

**Checkpoint(检查点)**: 判决书基础设施完成, 5 个夹具全部可用.

---

## Phase 3(阶段三): User Story 1(用户故事一) - 已知故障波形可复跑 (Priority: P1) MVP

**Goal(目标)**: 11 个故障波形场景每个都能通过 `cargo test --test chaos_suite -- --include-ignored` 一键复跑, 输出 JSON 判决书.

**Independent Test(独立测试)**: `cargo test --test chaos_suite -- --include-ignored` 执行所有场景, 退出码为 0 且 JSON 判决书中所有 `passed: true`.

### Tests for User Story 1(用户故事一的测试)

> **NOTE**: 这些测试本身是混沌场景(scenario)的实现, 先实现再运行验证.

- [x] T011 [P] [US1] 在 `tests/chaos/scenarios/child_panic_storm.rs` 中实现 `child_panic_storm` 场景: 使用 `FixtureChildSpawner` 通过 `with_panic_delay(Duration::from_millis(1))` 在 60s 内反复 spawn 并 panic. 验证 `self_panic_count == 0`, emit 延迟 p99 < 100µs. 输出 JSON 判决书.
- [x] T012 [P] [US1] 在 `tests/chaos/scenarios/child_block_forever.rs` 中实现 `child_block_forever` 场景: spawn 一个永不返回的 blocking worker, 触发关停. 验证关停阶段在 `graceful_timeout + abort_wait` 内完成, 无泄漏 slot. 输出 JSON 判决书.
- [x] T013 [P] [US1] 在 `tests/chaos/scenarios/child_ignore_cancel.rs` 中实现 `child_ignore_cancel` 场景: spawn 后忽略 `CancellationToken`, 触发 abort. 验证 abort 后 slot 在 `abort_wait` 内停用, 无 dangling handle. 输出 JSON 判决书.
- [x] T014 [P] [US1] 在 `tests/chaos/scenarios/rapid_failure_10k.rs` 中实现 `rapid_failure_10k` 场景: 60s 内触发 10_000 次快速失败(fail -> restart -> fail). 验证 `restart_budget` 恢复率 > 0(未耗尽), emit 延迟 p99 < 10ms. 输出 JSON 判决书.
- [x] T015 [P] [US1] 在 `tests/chaos/scenarios/slow_event_subscriber.rs` 中实现 `slow_event_subscriber` 场景: 使用 `FixtureEventThrottle` 设置 `slow_consumer_ms=100`, 运行高频事件泵. 验证背压策略与 006-5 默认(`AlertAndBlock`)一致, `event_gap_total` ≤ `discard_budget`. 输出 JSON 判决书.
- [x] T016 [P] [US1] 在 `tests/chaos/scenarios/command_channel_full.rs` 中实现 `command_channel_full` 场景: 快速填充 mpsc channel(capacity=256) 至满. 验证 `send()` 返回 `Err(Closed)` 而非无限阻塞, 控制循环不 panic. 输出 JSON 判决书.
- [x] T017 [P] [US1] 在 `tests/chaos/scenarios/ipc_connection_storm.rs` 中实现 `ipc_connection_storm` 场景: 使用 `FixtureIpcStress` 同时发起 1000 个劣质 TCP 握手. 验证合法客户端握手成功率 100%, 服务端 accept 队列 p50 < 1ms. 输出 JSON 判决书.
- [x] T018 [P] [US1] 在 `tests/chaos/scenarios/socket_path_contention.rs` 中实现 `socket_path_contention` 场景: 在已占用的 socket 路径上启动 dashboard. 验证返回结构化错误含 `field_path="ipc.path"` 和 `hint`, 不 panic. 输出 JSON 判决书.
- [x] T019 [P] [US1] 在 `tests/chaos/scenarios/relay_crash_loop.rs` 中实现 `relay_crash_loop` 场景: 模拟 relay 进程被 SIGKILL 后由监督器拉起 5 次. 验证第 5 次拉起后链路对齐在 10s 内完成, dashboard 状态与监督视图一致. 输出 JSON 判决书.
- [x] T020 [P] [US1] 在 `tests/chaos/scenarios/clock_step_backward.rs` 中实现 `clock_step_backward` 场景: 使用 `FixtureClockController` 模拟时钟回拨 10s. 验证滑动窗口预算不被扭曲(使用 monotonic clock), 熔断器状态未意外重置. 输出 JSON 判决书.
- [x] T021 [P] [US1] 在 `tests/chaos/scenarios/runtime_starvation_probe.rs` 中实现 `runtime_starvation_probe` 场景: 使用 `FixtureRuntimeProbe` 注入 `tokio::yield_now` 饥饿循环 30s. 验证控制循环迭代计数在 30s 内持续前进(>0 iter/s), emit 延迟 p99 < 100ms. 输出 JSON 判决书.

### Implementation for User Story 1(用户故事一的实现)

- [x] T022 [P] [US1] 在 `tests/chaos/scenarios/mod.rs` 中实现 `ScenarioRouter`: 一个 `run_all()` 方法按 spec FR-001 列表顺序串行执行 11 个场景, 收集 JSON 判决书并输出. 实现 `run(scenario_id: &str)` 单场景路由.
- [x] T023 [US1] 在 `tests/chaos_suite.rs` 中实现混沌套件测试入口: 定义 `#[test] #[ignore] fn chaos_suite()` 测试函数, 调用 `ScenarioRouter::run_all()`, 汇总所有判决书. 如果有任何 `passed == false`, 测试失败. 输出 JSON 数组到 stdout.

**Checkpoint(检查点)**: `cargo test --test chaos_suite -- --include-ignored` 通过, 11 个场景全部输出合法 JSON 判决书.

---

## Phase 4(阶段四): User Story 2(用户故事二) - 浸泡产出尾迹与资源曲线 (Priority: P1)

**Goal(目标)**: 不少于 24h(二十四小时) 浸泡测试框架可用, 产出 SoakReport Markdown 文件.

**Independent Test(独立测试)**: `cargo test --test soak_suite -- --ignored` 执行 24h 浸泡, 退出码为 0, `artifacts/validation/` 下生成 `.md` 报告文件.

### Tests for User Story 2(用户故事二的测试)

> **NOTE**: 浸泡测试本身是测试代码, 测试代码就是实现. 先实现框架再运行验证.

- [x] T024 [P] [US2] 在 `tests/soak/metrics_collector.rs` 中实现 `MetricsCollector`: 每秒采集 `p99_latency_ms`(控制循环 emit 延迟, 1s 滑动窗口), 每 60s 采集 `rss_mb`(通过 `/proc/self/status` 或 `libc` 获取 RSS), 每 60s 采集 `fd_count`(通过 `/dev/fd` 计数). 输出 CSV 数据到内存缓冲区.
- [x] T025 [P] [US2] 在 `tests/soak/fixtures/steady_traffic.rs` 中实现 `SteadyTrafficGenerator`: 合成稳态流量脚本, 维持 1000 req/s(每秒请求数) 的负载. 支持 `start()` 和 `stop()` 方法. 流量通过模拟 child event 或 command 注入.
- [x] T026 [US2] 在 `tests/soak/report.rs` 中实现 `SoakReport` 结构和 `ReportGenerator`: 接收 `MetricsCollector` 的 CSV 数据, 计算每个指标的 p99/avg/max, 对照 SoakReport 浸泡阈值表生成 Markdown 报告. 报告格式符合 `contracts/soak-report-format.md`.
- [x] T027 [US2] 在 `tests/soak/mod.rs` 中实现 `SoakRuntime`: 统筹浸泡流程——启动 `SteadyTrafficGenerator`, 启动 `MetricsCollector`, 等待指定时长(默认 24h, 可通过 `SOAK_DURATION_MINUTES` 环境变量覆盖), 停止流量, 执行 `ShutdownSequence::run(100x)`(合成关停 100 次并计算 `shutdown_success_ratio`), 调用 `ReportGenerator::generate()`, 写入文件到 `artifacts/validation/soak-{timestamp}.md`.
- [x] T028 [US2] 在 `tests/soak_suite.rs` 中实现浸泡测试入口: 定义 `#[test] #[ignore] fn soak_24h()` 测试函数, 调用 `SoakRuntime::run(Duration::from_secs(86400))`. 如果任何指标越界且无豁免工单, 测试失败.

**Checkpoint(检查点)**: 浸泡框架完成, 可通过 `SOAK_DURATION_MINUTES=5 cargo test --test soak_suite -- --ignored --nocapture` 验证缩短版浸泡流程.

---

## Phase 5(阶段五): User Story 3(用户故事三) - IPC 风暴与中继生命周期 (Priority: P2)

**Goal(目标)**: IPC 连接风暴场景和中继崩溃恢复场景可独立验证.

**Independent Test(独立测试)**: `cargo test --test chaos_suite -- --include-ignored ipc_connection_storm relay_crash_loop` 的两个场景分别通过.

### Tests for User Story 3(用户故事三的测试)

> **NOTE**: US3 的测试场景已在 US1 中实现(T017 IPC 连接风暴, T019 中继崩溃循环). 本阶段增加 US3 特有的速率限制器和客户端分类验证.

- [x] T029 [P] [US3] 在 `tests/chaos/fixtures/ipc_stress.rs` 中(已有 T008 基础)扩展 `RateLimiter` 实现: 固定窗口(1s) + 令牌桶(容量 100, 恢复率 50/s). 实现 `try_acquire() -> bool`. 添加 `ClientClassification` 逻辑: 合法客户端(payload 含合法 `target_id` 字段的 JSON) vs 劣质客户端(非法 JSON 或缺失字段).
- [x] T030 [US3] 在 `tests/chaos/scenarios/ipc_connection_storm.rs`(T017 已有)扩展: 验证速率限制耗尽时合法客户端仍能完成握手, 或在结构化错误中读到 `ResourceExhausted { resource: "ipc_accept", limit: 100 }`. 验证服务端在劣质客户端断开后 1s 内恢复 accept.
- [x] T031 [US3] 在 `tests/chaos/scenarios/relay_crash_loop.rs`(T019 已有)扩展: 验证用户可见 dashboard 状态与监督视图在契约阈值内对齐, 否则降级原因段落对用户可读.

**Checkpoint(检查点)**: IPC 风暴和 relay 崩溃场景的速率限制和客户端分类断言通过.

---

## Phase 6(阶段六): Polish(收尾与交叉关注点)

**Purpose(目的)**: 补齐覆盖率、文档和 006-2 集成.

- [x] T032 在 `specs/006-7-chaos-soak-reliability/quickstart.md` 中更新 CI 运行说明, 确认 chaos_suite 和 soak_suite 的运行命令与预期退出码.
- [x] T033 更新 `specs/006-2-release-supply-chain-gates/spec.md` 的 `QualityGateOutcome` 外链列, 登记混沌套件与浸泡归档路径. 归档格式使用 `contracts/chaos-scenario-verdict.md` 和 `contracts/soak-report-format.md` 定义的 schema.
- [x] T034 在 `tests/chaos/verdict.rs` 中添加 JSON 判决书 schema 校验测试: 每个场景输出的 JSON 必须通过 `chaos-scenario-verdict.md` schema 验证.
- [x] T035 确认 `cargo test`(排除 chaos/soak 测试) 在实现前后无回归. 确认 `cargo clippy` 无新增警告.

**Checkpoint(检查点)**: 全部 35 个任务完成, CI 可运行混沌与浸泡套件.
