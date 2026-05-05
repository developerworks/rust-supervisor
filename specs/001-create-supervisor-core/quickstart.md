# Quickstart(快速开始): 创建监督器核心

本 quickstart(快速开始) 说明第一版实现应该怎样验证.

## 1. 验证项目基线

```bash
cargo fmt --check
cargo check
cargo test
cargo doc --no-deps
cargo package --list
scripts/generate-sbom.sh
scripts/validate-sbom.sh
cargo publish --dry-run
```

## 2. 预期最小用法

本功能应该允许维护者从 rust-config-tree(集中配置树) 加载 centralized configuration(集中化配置),派生 `SupervisorSpec`(监督器规格),并通过 supervisor(监督器) 运行它,而不是手写无人管理的后台 spawn(启动任务).

```rust
use rust_supervisor::config::ConfigSource;
use rust_supervisor::config::SupervisorConfig;
use rust_supervisor::runtime::Supervisor;

let config = SupervisorConfig::load_from_tree(ConfigSource::file(
    "examples/config/supervisor.yaml",
))
.await?;

let config_state = config.to_state()?;
let spec = config_state.to_supervisor_spec()?;

let handle = Supervisor::start(spec).await?;
let state = handle.current_state().await?;
handle.shutdown_tree("operator", "quickstart complete").await?;
```

实现期间 builder(构建器) 名称可以调整,但最终 API(接口) 必须保留这些契约:rust-config-tree(集中配置树) 加载,`ConfigState`(配置状态),声明式 child spec(子任务规格),tree spec(树规格),runtime handle(运行时句柄),readiness(就绪),current_state query(当前状态查询),event journal(事件日志缓冲区),`RunSummary`(运行摘要),observability pipeline(可观测性管线) 和 four-stage shutdown(四阶段关闭).

## 3. 集中配置示例

配置必须集中在 rust-config-tree(集中配置树) v0.1.9 边界,不能散落到模块内部.主配置格式必须使用 YAML(数据序列化格式),示例路径固定为 `examples/config/supervisor.yaml`.

```yaml
supervisor:
  id: root
  strategy: OneForOne
defaults:
  restart:
    policy: Transient
  backoff:
    initial_ms: 100
    max_ms: 5000
    jitter_percent: 10
    reset_after_ms: 60000
  health:
    heartbeat_ms: 1000
    stale_after_ms: 3000
  shutdown:
    graceful_timeout_ms: 5000
    abort_wait_ms: 1000
observability:
  structured_log: true
  tracing: true
  metrics: true
  audit: true
  event_journal_capacity: 256
children:
  - id: binance_ws
    name: Binance WebSocket
    kind: AsyncWorker
    criticality: Degraded
    readiness: Explicit
    tags:
      - market
```

必需配置测试:

```bash
cargo test centralized_yaml_config_builds_supervisor_spec
cargo test invalid_yaml_config_state_rejects_tree_startup
cargo test yaml_configuration_uses_rust_config_tree
```

## 4. 必需验收测试

### Panic Restart(恐慌重启)

```bash
cargo test panic_records_restart_events
```

预期行为:

- 系统发送 `ChildPanicked` 事件.
- 系统发送 `BackoffScheduled` 事件.
- 系统发送 `ChildRestarting` 事件.
- attempt(尝试次数) 在重启后的 child(子任务) 运行前递增.

### Child Quarantine(子任务隔离)

```bash
cargo test child_quarantines_after_restart_window
```

预期行为:

- 60 秒内第 11 次重启会把 child(子任务) 置为 `Quarantined`(已隔离).
- quarantine(隔离) 后不会继续自动重启.
- current state(当前状态) 报告隔离状态和最近策略决定.

### Supervisor Meltdown(监督器熔断)

```bash
cargo test supervisor_meltdown_escalates
```

预期行为:

- 一个 supervisor(监督器) 范围在 60 秒内第 31 次 child(子任务) 失败时,系统发送 `Meltdown`(熔断).
- parent supervisor(父监督器) 收到 escalation(升级).

### Shutdown Without Orphaned Tasks(关闭后不留下孤儿任务)

```bash
cargo test root_shutdown_leaves_no_orphaned_tasks
```

预期行为:

- 每个 child cancellation token(子任务取消令牌) 都被触发.
- 超时前退出的 child(子任务) 报告 graceful completion(优雅完成).
- 未退出的 async child(异步子任务) 在超时后被 abort(强制终止).
- root shutdown(根关闭) 后,runtime task set(运行时任务集合) 为空.

### Supervision Strategies(监督策略)

```bash
cargo test one_for_all_restarts_group_in_order
cargo test rest_for_one_restarts_failed_and_later_children
```

预期行为:

- `OneForAll`(一对全部) 先停止所有 sibling(同级任务),再按定义顺序重启.
- `RestForOne`(从失败处开始) 不重启失败 child(子任务) 之前定义的 child(子任务).
- `GroupStrategy`(分组策略) 只重启匹配 child tag(子任务标签) 的 group(分组) 范围.
- `ChildStrategyOverride`(子任务级覆盖) 优先于 group strategy(分组策略) 和 supervisor-wide strategy(监督器全局策略).
- runtime control loop(运行时控制循环) 通过 `StrategyExecutionPlan`(策略执行计划) 执行重启,事件名使用 `restart_plan`(重启计划).

### Runtime Control(运行时控制)

```bash
cargo test supervisor_handle_operations_are_idempotent
```

预期行为:

- `add_child` 会先通过 `DynamicSupervisorPolicy`(动态监督器策略) 校验,通过后接受 child manifest(子任务清单文本) 并更新 current state(当前状态) 计数.
- `remove_child` 会先关闭 child(子任务),再删除 registry(注册表) 记录.
- `restart_child` 会记录 attempt(尝试次数),generation(代次),backoff(退避) 和 restart decision(重启决策).
- `pause_child` 会让 child(子任务) 进入 `Paused`(已暂停).
- `resume_child` 会恢复 child(子任务) 的运行治理.
- `quarantine_child` 会阻止目标 child(子任务) 自动重启.
- `shutdown_tree` 会运行四阶段关闭.
- `current_state` 只返回当前状态.
- `subscribe_events` 只返回生命周期事件流.

### Event Shape(事件形状)

```bash
cargo test every_state_transition_has_when_where_what
```

预期行为:

- 每次状态迁移产生一条事件.
- 事件包含 `When`(何时),`Where`(何处),`What`(发生内容),sequence(序号),correlation id(关联标识) 和 config version(配置版本).

### Deterministic Time(确定性时间)

```bash
cargo test paused_time_drives_backoff_heartbeat_and_meltdown
```

预期行为:

- 测试不等待真实 60 秒窗口.
- jitter(抖动) 可以关闭,或者可以被确定性控制.

### Readiness(就绪)

```bash
cargo test explicit_readiness_controls_ready_state
```

预期行为:

- explicit readiness(显式就绪) 的 child(子任务) 在报告 ready(已就绪) 前不会出现在 ready(已就绪) 状态.
- child(子任务) 报告 ready(已就绪) 后,系统发送 `ChildReady` 事件.
- current state(当前状态) 和 event(事件) 对 ready(已就绪) 状态保持一致.

### Blocking Task Shutdown(阻塞任务关闭)

```bash
cargo test blocking_task_timeout_records_boundary
```

预期行为:

- blocking task(阻塞任务) 关闭超时后,系统记录不可立即终止边界.
- 系统不把 blocking task(阻塞任务) 当作普通 async task(异步任务) 强制终止.
- 策略决定会说明升级路径.

### Four-Stage Shutdown(四阶段关闭)

```bash
cargo test root_shutdown_runs_four_stages_in_reverse_order
```

预期行为:

- root shutdown(根关闭) 按 request stop(请求停止),graceful drain(优雅排空),abort stragglers(强制终止拖尾任务) 和 reconcile(状态对账) 执行.
- child(子任务) 按声明顺序的逆序关闭.
- reconcile(状态对账) 后 registry(注册表),current state(当前状态),metrics(指标) 和 event journal(事件日志缓冲区) 的最终状态一致.

### Diagnostic Replay(诊断回放)

```bash
cargo test run_summary_includes_recent_journal_events
```

预期行为:

- meltdown(熔断),关闭超时或父级升级发生时,系统生成 `RunSummary`(运行摘要).
- `RunSummary`(运行摘要) 包含最近 event journal(事件日志缓冲区) 中的关键事件.
- 摘要可以解释失败原因,重启次数,关闭原因和最终状态.

## 5. Observability Smoke Check(可观测性冒烟检查)

运行一个 example(示例) 或 integration test(集成测试),并确认它发送这些内容:

```bash
cargo test observability_records_all_signals
cargo run --example observability_probe
```

- 每个 child attempt(子任务尝试) 一个 `tracing`(结构化追踪) span(追踪范围).
- 每次状态迁移一个 structured log(结构化日志).
- 每次状态迁移一个 `tracing`(结构化追踪) event(追踪事件).
- 通过 metrics facade(指标门面) 发送必需指标.
- 每个 control command(控制命令) 都有 command audit event(命令审计事件).
- test recorder(测试记录器) 可以断言所有 signal(信号) 的 sequence(序号),correlation id(关联标识) 或 config version(配置版本) 一致.

## 6. Examples(示例程序)

```bash
cargo run --example supervisor_quickstart
cargo run --example config_tree_supervisor
cargo run --example restart_policy_lab
cargo run --example shutdown_tree
cargo run --example observability_probe
```

示例必须覆盖 quickstart(快速开始),rust-config-tree(集中配置树),restart policy(重启策略),four-stage shutdown(四阶段关闭) 和 observability(可观测性).

## 7. Documentation And Release Gates(文档和发布关口)

```bash
cargo test documentation_sync_matches_public_api
cargo test bilingual_documentation_is_isomorphic
cargo test coding_standard_is_enforced
cargo test source_layout_uses_top_level_directory_modules
cargo test mod_rs_contains_only_module_declarations
cargo test no_supervision_directory_layer_exists
cargo test no_flat_top_level_module_files_exist
cargo test source_code_avoids_forbidden_snapshot_and_view_names
cargo test no_view_code_names_exist
cargo test no_compatibility_methods_exist
cargo test cognitive_complexity_stays_within_budget
cargo test maintainability_profile_is_valid
cargo test module_dependency_map_has_no_cycles
cargo test parallel_workstreams_have_no_file_conflicts
cargo test blocker_elimination_records_are_complete
cargo test unattended_implementation_completion_ledger_is_complete
cargo test lead_agent_supervision_reviews_subagents
cargo test correction_loop_closes_all_drifts
cargo test sbom_artifacts_match_cargo_lock
cargo test release_readiness_matches_crates_io
cargo test test_files_end_with_test_rs
cargo test glossary_covers_professional_and_backtick_terms
cargo test yaml_configuration_uses_rust_config_tree
```

完成条件:

- `manual/zh` 和 `manual/en` 目录结构一致.
- `docs/zh` 和 `docs/en` 目录结构一致.
- `specs/001-create-supervisor-core/glossary.md` 存在,并覆盖专业词汇和反引号词汇.
- README(说明文档),LICENSE(许可证) 和 CHANGELOG(变更日志) 存在.
- source comment(源码注释) 和 rustdoc(代码文档注释) 使用英文.
- 源码使用 `src/<module>/` top-level directory module(顶层目录模块) 结构,不存在 `src/supervision/` 中间层,也不存在 `src/<module>.rs` 平铺模块文件.
- `src/lib.rs` 只包含 crate doc(包文档) 和顶层 `pub mod <mod_name>;`,每个 `src/<module>/mod.rs` 只包含 `pub mod <mod_name>;`.
- 内部导入不使用 `super::`.
- 所有测试文件必须以 `_test.rs` 结尾,并且 integration test(集成测试) 放在 `src/tests/*_test.rs`,unit test(单元测试) 放在模块自己的 `tests/*_test.rs` 目录.
- rust-config-tree(集中配置树) 必须使用 v0.1.9,主配置必须使用 YAML(数据序列化格式),并且 quickstart(快速开始),示例和契约不得把 TOML(配置格式),JSON(数据交换格式) 或其它格式作为主配置格式.
- 代码命名只使用 `ConfigState`(配置状态),`SupervisorState`(监督器状态),`ChildState`(子任务状态),`current_state`(当前状态) 和 `state`(状态),不存在任何 `*Snapshot`,`*View`,`state_view` 命名或 `snapshot()` 查询方法.
- 源码,示例和文档不存在旧接口别名,迁移层,历史行为保留开关,废弃 facade(门面),兼容包装函数或第三方 API(接口) 形状复制.
- cognitive complexity(认知复杂度) 符合预算.
- maintainability profile(可维护性画像) 可以追踪模块职责,依赖,测试和文档.
- module dependency map(模块依赖图) 不存在 cycle dependency(循环依赖).
- parallel workstream(并行工作流) 不存在主文件冲突.
- blocker elimination record(卡点消除记录),task completion ledger(任务完成台账),lead agent supervision(主代理监督) 和 correction record(纠偏记录) 全部闭环.
- `artifacts/sbom/rust-supervisor.cdx.json` 存在并通过 CycloneDX JSON(CycloneDX JSON 格式) 校验.
- `artifacts/sbom/rust-supervisor.spdx.json` 存在并通过 SPDX JSON(SPDX JSON 格式) 校验.
- SBOM(软件物料清单) 依赖版本与 `Cargo.lock` 一致.
- `cargo package --list` 不包含 target(构建产物),临时文件或无关大文件.
- `cargo publish --dry-run` 通过.

## 8. Completion Gate(完成关口)

进入实现前,以下文件必须存在:

```text
specs/001-create-supervisor-core/plan.md
specs/001-create-supervisor-core/research.md
specs/001-create-supervisor-core/data-model.md
specs/001-create-supervisor-core/glossary.md
specs/001-create-supervisor-core/contracts/public-api.md
specs/001-create-supervisor-core/quickstart.md
specs/001-create-supervisor-core/tasks.md
```

只有所有 quickstart(快速开始) 检查通过后,实现才算完成.
