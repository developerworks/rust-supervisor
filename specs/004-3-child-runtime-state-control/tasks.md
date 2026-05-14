---
description: "Task list(任务列表): 子任务运行状态控制"
---

# Tasks(任务): 子任务运行状态控制

**Input(输入)**: 设计文档来自 `/specs/004-3-child-runtime-state-control/`
**Prerequisites(前置文档)**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/child-runtime-state-control.md`, `quickstart.md`

**Tests(测试)**: 本功能改变 `PauseChild(暂停子任务)`, `RemoveChild(移除子任务)`, `QuarantineChild(隔离子任务)` 和 `CurrentState(当前状态)` 的运行时行为, 触发宪章原则 III. "行为变化必须先有测试". `RestartChild(重启子任务)` 与 `ResumeChild(恢复子任务)` 是既有命令, 本任务列表只要求它们不破坏运行状态事实, 不把它们作为本功能的新生命周期交付对象. 每个 user story(用户故事) 都先列测试任务, 再列实现任务.

**Organization(组织方式)**: 任务按 user story(用户故事) 分组, 每个 story 都能独立验证.

## Format(格式): `[ID] [P?] [Story] Description(描述)`

- **[P]**: 任务修改不同文件, 且不依赖未完成任务. 当多个任务都改同一个文件时, 全部不标 `[P]`.
- **[Story]**: `[US1]`, `[US2]`, `[US3]` 把任务映射到 spec.md 中的用户故事. Setup, Foundational 和 Polish 阶段不带 story 标签.
- 任务描述必须写出准确文件路径.
- Rust(编程语言) 项目中, 本功能新增测试文件必须放在外部 `src/tests/` 或 `tests/`, 不写入 `src/` 模块文件的内联模块. `src/control/tests/control_test.rs` 是 `Cargo.toml` 已注册的既有外部测试目标, 本功能只允许在 T014 中更新它的既有断言.

## Path Conventions(路径约定)

Rust single crate(Rust 单包): 仓库根目录下的 `src/`, `tests/` 和 `Cargo.toml`. 本功能不引入新 crate(库). 新增到 `src/tests/` 的外部测试文件必须同步写入 `Cargo.toml` 的 `[[test]]` 目标, 否则 `cargo test --test <name>` 无法发现该测试. 已存在的 `tests/dashboard_protocol_shape_test.rs` 是 Cargo(构建工具) 自动发现的集成测试, 本功能只能原地更新该文件, 不得新增同名 `[[test]]` 目标.

---

## Phase 1(阶段一): Setup(共享基础设施)

**Purpose(目的)**: 创建本功能必需的空模块占位, 并确认基线编译与测试通过.

- [ ] T001 在 `src/runtime/child_runtime_state.rs` 创建空模块文件, 并在 `src/runtime/mod.rs` 中注册 `pub mod child_runtime_state;` 以便后续阶段写入 `ChildRuntimeState(子任务运行状态记录)` 类型.
- [ ] T002 在 `src/control/outcome.rs` 创建空模块文件, 并在 `src/control/mod.rs` 中注册 `pub mod outcome;` 以便后续阶段写入 `ChildControlResult(子任务控制结果)` 与相关公开类型.
- [ ] T003 在仓库根目录运行 `cargo fmt --check` 和 `cargo test` 建立基线, 确认基线全部通过, 失败时必须先修复回归再开始后续阶段. 本任务必须在 T001 与 T002 均已合并为可编译工作区之后执行, 不得标为可与 T001/T002 并行, 理由见下文 Parallel Opportunities(并行机会) 中 Setup(阶段一) 说明.

---

## Phase 2(阶段二): Foundational(阻塞前置基础)

**Purpose(目的)**: 引入全部公开类型, 升级 `ChildRunHandle(子任务运行句柄)`, 替换 `CommandResult::ChildState(子任务状态命令结果)` 变体, 让 `RuntimeControlState(运行时控制状态)` 与 `ShutdownPipeline(关闭流水线)` 共享 `ChildRuntimeState(子任务运行状态记录)`. 本阶段完成前任何 user story(用户故事) 实现都不能开始.

### Foundational Tests(基础阶段测试)

> **NOTE(说明): 先写下列测试, 并确认它们在实现前失败.**

- [ ] T004 在 `src/tests/supervisor_control_test.rs` 中添加 `child_state_result_variant_is_replaced_by_child_control_test` 回归测试: 构造停止类控制命令并断言返回 `CommandResult::ChildControl(子任务控制命令结果)`, 同时确认旧 `CommandResult::ChildState(子任务状态命令结果)` 不再被匹配. 注: `Cargo.toml` 已经把 `src/control/tests/control_test.rs` 注册为独立 `control_test(控制测试)` 目标并参与 `cargo test`, 该文件中的旧 `CommandResult::ChildState(子任务状态命令结果)` 断言必须由 T014 同步更新.
- [ ] T005 在 `src/tests/supervisor_control_test.rs` 中添加 `child_control_result_contains_runtime_state_identity_test` 回归测试: 执行 `PauseChild(暂停子任务)` 后断言 `ChildControlResult(子任务控制结果)` 至少包含 `child_id(子任务标识)`, `generation(代次)`, `attempt(尝试)`, `operation_after(命令后操作)`, `status(状态)` 和 `stop_state(停止状态)` 字段. 该测试必须明确断言活动 attempt(尝试) 的 `ChildControlResult.status(子任务控制结果状态)` 为 `Some(有值)`.
- [ ] T006 在 `src/tests/supervisor_real_shutdown_pipeline_test.rs` 中添加 `shutdown_pipeline_uses_child_runtime_state_handles_test` 回归测试: 启动可取消 child(子任务), 执行 `ShutdownTree(关闭监督树)`, 断言关闭流水线通过共享 `ChildRuntimeState(子任务运行状态记录)` 句柄送达取消, 而不是维护独立的 `ActiveChildAttempt(活动子任务尝试)` 路径.

### Foundational Implementation(基础阶段实现)

- [ ] T007 [P] 在 `src/control/outcome.rs` 定义公开类型 `ChildAttemptStatus(子任务尝试状态)`, `ChildControlOperation(子任务控制操作)`, `ChildStopState(子任务停止状态)`, `ChildControlFailurePhase(子任务控制失败阶段)`, `RestartLimitState(重启次数限制状态)`, `ChildLivenessState(子任务存活状态)`, `ChildControlFailure(子任务控制失败原因)`, `ChildRuntimeRecord(子任务运行状态记录)` 与 `ChildControlResult(子任务控制结果)`, 全部字段含中文文档, 提供 `Serialize / Deserialize / Clone / Debug / PartialEq / Eq` 派生与构造函数. 本任务不得依赖 `src/runtime/child_runtime_state.rs`, 以保持 control(控制) 模块不反向依赖 runtime(运行时) 模块.
- [ ] T008 在 T007 完成后, 在 `src/runtime/child_runtime_state.rs` 定义 `ChildRuntimeState(子任务运行状态记录)` 结构体, 字段按 `data-model.md` 中 Entity 一节定义, 引用 T007 已有的 `ChildAttemptStatus(子任务尝试状态)` 与 `ChildControlOperation(子任务控制操作)`, 包含 `stop_deadline_at_unix_nanos(停止截止时间)` 与 `last_control_failure(最近控制失败原因)`. 有活动 attempt(尝试) 时运行时句柄字段为 `Some(有值)`, 无活动 attempt(尝试) 时 `generation / attempt / status / cancellation_token / abort_handle / completion_receiver / heartbeat_receiver / readiness_receiver` 必须同时为 `None(无值)`. 实现 `new_placeholder`, `activate_attempt`, `cancel`, `abort`, `wait_for_report`, `observe_liveness`, `update_restart_limit` 方法. `to_record(生成记录)` 依赖 T007 的公开状态记录类型, 在 T019 中补齐. 全部方法附中文文档与必要 doctest.
- [ ] T009 [P] 在 `src/event/payload.rs` 添加 6 个新事件变体 `ChildControlCancelDelivered(子任务控制取消已送达)`, `ChildControlStopCompleted(子任务控制停止完成)`, `ChildControlStopFailed(子任务控制停止失败)`, `ChildControlOperationChanged(子任务控制操作变化)`, `ChildRuntimeStateRemoved(子任务运行状态记录已移除)`, `ChildHeartbeatStale(子任务运行状态记录心跳陈旧)`, 字段按 `contracts/child-runtime-state-control.md` 中 Event Contract(事件契约) 表格定义.
- [ ] T010 [P] 在 `src/observe/metrics.rs` 注册 4 个新指标 `supervisor_child_control_command_total(子任务控制命令总数)`, `supervisor_child_runtime_restart_limit_remaining(子任务运行状态记录剩余重启次数)`, `supervisor_child_runtime_heartbeat_stale_total(子任务运行状态记录心跳陈旧总数)`, `supervisor_child_runtime_operation_transitions_total(子任务控制操作转换总数)`, 标签按契约约束. `supervisor_child_runtime_heartbeat_stale_total(子任务运行状态记录心跳陈旧总数)` 不得使用 `child_id(子任务标识)` 标签.
- [ ] T011 修改 `src/readiness/signal.rs`, `src/task/context.rs` 与 `src/child_runner/runner.rs`: 新增 `ReadinessState(就绪状态)` 枚举, 把 `ReadySignal(就绪信号)` 从 `watch::Receiver<bool>` 升级为 `watch::Receiver<ReadinessState>`, 初始值为 `Unreported(未上报)`, 继续提供 `mark_ready(标记就绪)`, 新增 `set_readiness(设置就绪状态)`. `mark_ready(标记就绪)` 必须等价于调用 `set_readiness(ReadinessState::Ready)`(设置就绪状态为已就绪). 在 `ChildRunHandle(子任务运行句柄)` 上新增 `heartbeat_receiver: watch::Receiver<Option<Instant>>` 与 `readiness_receiver: watch::Receiver<ReadinessState>` 字段, 同步修改 `spawn_once(派生一次)` 保存这两个 receiver(接收端) 而不再丢弃, 并更新现有 doctest 与函数文档.
- [ ] T012 修改 `src/control/command.rs`, 把 `CommandResult::ChildState(子任务状态命令结果)` 变体替换为 `CommandResult::ChildControl(子任务控制命令结果) { outcome: ChildControlResult }`, 在 `CurrentState(当前状态)` 中新增 `child_runtime_records: Vec<ChildRuntimeRecord>` 字段, 调整 doctest 与公开 API 文档, 不添加旧变体的类型别名, 不引入 compatibility export(兼容导出).
- [ ] T013 修改 `src/runtime/shutdown_pipeline.rs` 和 `src/runtime/control_loop.rs`: 删除 `ActiveChildAttempt(活动子任务尝试)` 结构体, 使 `ShutdownPipeline(关闭流水线)` 全部消费点改为使用 `ChildRuntimeState(子任务运行状态记录)`, 把 `RuntimeControlState.child_runtime_states` 字段类型改为 `HashMap<ChildId, ChildRuntimeState>`, 删除 `children: HashMap<ChildId, ManagedChildState>` 字段, 同步调整 `spawn_child_attempt(派生子任务尝试)` 与 `prepare_child_attempt(准备子任务尝试)` 中运行状态记录占位, attempt(尝试) 激活和状态更新逻辑.
- [ ] T014 同步修改 `src/dashboard/protocol.rs`, `src/dashboard/ipc_server.rs`, `src/dashboard/model.rs`, `src/tests/supervisor_examples_test.rs`, `src/control/tests/control_test.rs` 与既有 `tests/dashboard_protocol_shape_test.rs` 中所有引用 `CommandResult::ChildState(子任务状态命令结果)` 的位置, 改为使用 `CommandResult::ChildControl(子任务控制命令结果)`, 并更新 dashboard(仪表盘) 返回结果模型以支持 `CurrentState.child_runtime_records(当前状态子任务运行状态记录集合)`. `tests/dashboard_protocol_shape_test.rs` 必须证明控制命令请求字段没有漂移, 同时 dashboard 返回结果模型测试必须覆盖有意变化的 `ChildControl(子任务控制)` 与 `CurrentState.child_runtime_records(当前状态子任务运行状态记录集合)` 返回结果字段. 不得新增 `src/tests/dashboard_protocol_shape_test.rs`, 也不得在 `Cargo.toml` 中注册同名 `[[test]]` 目标. `src/control/tests/control_test.rs` 在 `Cargo.toml` 中被注册为独立 `control_test(控制测试)` 测试目标, 删除旧变体后该目标必须仍然 `cargo test` 通过.

**Checkpoint(检查点)**: 编译通过. 已有 `supervisor_control_test`, `supervisor_real_shutdown_pipeline_test`, `supervisor_runtime_lifecycle_test`, `supervisor_examples_test`, `control_test`(由 `Cargo.toml` 注册的独立测试目标, 路径 `src/control/tests/control_test.rs`) 中与 `CommandResult::ChildState(子任务状态命令结果)` 相关的断言必须同步更新为 `ChildControl(子任务控制)` 变体, 不得继续提供旧形状.

---

## Phase 3(阶段三): User Story 1(用户故事一) - 查看真实子任务尝试状态 (Priority(优先级): P1) MVP(最小可用产品)

**Goal(目标)**: 操作者通过 `CurrentState(当前状态)` 可以一次性读取全部 `ChildRuntimeState(子任务运行状态记录)` 的真实活动尝试, generation(代次), status(状态), operation(操作), heartbeat(心跳), readiness(就绪状态) 和 restart_limit(重启次数限制) 剩余次数, 不依赖任何控制命令副作用.

**Independent Test(独立测试)**: 启动两个声明 child(子任务), 一个会上报 heartbeat 与 readiness, 另一个不上报心跳. 立即调用 `CurrentState(当前状态)`, 验证 `child_runtime_records(子任务运行状态记录集合)` 字段完整且区分两种存活状况.

### Tests for User Story 1(用户故事一的测试)

> **NOTE(说明): 先写下列三个测试函数, 并确认它们在实现前失败.**

- [ ] T015 [US1] 在 `src/tests/supervisor_child_runtime_state_control_test.rs` 新建测试模块, 并在 `Cargo.toml` 中新增 `[[test]] name = "supervisor_child_runtime_state_control_test"` 且 `path = "src/tests/supervisor_child_runtime_state_control_test.rs"` 的测试目标注册. 添加 `current_state_exposes_full_runtime_state_fields_test` 测试: 启动两个按声明顺序排列且会上报 heartbeat 与 readiness 的 child(子任务), 验证 `CurrentState.child_runtime_records(子任务运行状态记录集合)` 数量等于两个声明运行状态记录, 顺序等于声明顺序, 每个 `ChildRuntimeRecord(子任务运行状态记录)` 都包含正确的 `child_id`, `generation = Some(有值)`, `attempt = Some(有值)`, `status = Some(有值)`, `operation = Active(活跃)`, `liveness.last_heartbeat_at_unix_nanos = Some(有值)`, `liveness.readiness = ReadinessState::Ready(就绪状态为已就绪)`, `restart_limit.remaining > 0(剩余大于零)`, `failure = None(无值)`. 同一测试必须抽出 `assert_current_state_fast_20_reads(断言当前状态二十次快速读取)` 测试辅助函数, 使用 `std::time::Instant(标准库时间点)` 或等价方式记录 `CurrentState(当前状态)` 调用结果构造耗时, 连续读取 20 次, 验证每次构造耗时都低于 1 毫秒; 失败时输出最慢一次耗时和 `child_runtime_records(子任务运行状态记录集合)` 数量作为诊断. 后续凡通过 `CurrentState(当前状态)` 读取运行状态事实的测试必须复用该辅助函数.
- [ ] T016 [US1] 在同测试文件中添加 `current_state_distinguishes_no_heartbeat_from_stale_test` 测试: 启动一个尚未发送心跳的 child(子任务), 立即读取 `CurrentState(当前状态)`, 验证 `liveness.last_heartbeat_at_unix_nanos = None(无值)` 且 `liveness.heartbeat_stale = false(否)`. 然后让 child 发出心跳后等待超过 `DEFAULT_HEARTBEAT_TIMEOUT_SECS = 5` 默认阈值, 重新读取, 验证 `last_heartbeat_at_unix_nanos = Some(有值)` 且 `heartbeat_stale = true(是)`. 同一测试必须验证 `ChildHeartbeatStale(子任务运行状态记录心跳陈旧)` 事件字段包含 `child_id`, `attempt`, `since_unix_nanos`, 且同一 `(child_id, attempt)` 在 attempt(尝试) 终止前重复读取 `CurrentState(当前状态)` 不会重复发布事件或重复增加 `supervisor_child_runtime_heartbeat_stale_total(子任务运行状态记录心跳陈旧总数)` counter(计数器).
- [ ] T017 [US1] 在同测试文件中添加 `current_state_distinguishes_unreported_from_degraded_readiness_test` 测试: 启动一个未上报 readiness 的 child(子任务), 验证 `liveness.readiness = ReadinessState::Unreported(就绪状态未上报)`; 切换为 child 上报 `ReadinessState::NotReady(就绪状态未就绪)` 时, 验证 `liveness.readiness = ReadinessState::NotReady(就绪状态未就绪)`.

### Implementation for User Story 1(用户故事一的实现)

- [ ] T018 [US1] 在 `src/runtime/control_loop.rs` 中实现 `build_current_state(构造当前状态)` 私有函数: 对 `child_runtime_states(子任务运行状态记录集合)` 中的每个 `ChildRuntimeState(子任务运行状态记录)` 调用 `observe_liveness(观察存活)` 然后 `to_record(生成记录)`, 按声明顺序排序后写入 `CurrentState.child_runtime_records(子任务运行状态记录集合)`. 修改 `ControlCommand::CurrentState(当前状态控制命令)` 分支调用本函数. 无活动 attempt(尝试) 的运行状态记录也必须输出 `ChildRuntimeRecord(子任务运行状态记录)`, 其中 `generation / attempt / status` 为 `None(无值)`. 实现必须只做非阻塞读取和线性状态记录构造, 不得 `await(异步等待)` 子任务, 不得执行额外 I/O(输入输出), 以满足代表性测试场景中连续 20 次构造每次都低于 1 毫秒的性能目标.
- [ ] T019 [US1] 在 `src/runtime/child_runtime_state.rs` 中定义 `RuntimeTimeBase(运行时时间基准)` 类型, 并在 `src/runtime/control_loop.rs` 的 `RuntimeControlState(运行时控制状态)` 中持有唯一实例. `RuntimeTimeBase(运行时时间基准)` 必须在 supervisor runtime(监督器运行时) 初始化时记录 `base_instant = tokio::time::Instant::now()` 和 `base_unix_nanos = SystemTime::now().duration_since(UNIX_EPOCH)` 的纳秒值. `observe_liveness(观察存活)` 必须通过只读引用接收该基准, 正确处理 `watch::Receiver(观察接收端)` 的 borrow(借用) 语义, 并把 `Option<Instant>` 转换成 `Option<u128>`. 心跳时间戳必须按 `base_unix_nanos + (heartbeat_instant - base_instant)` 换算, 早于基准的心跳使用饱和相减, 不得用 `SystemTime::UNIX_EPOCH.elapsed()` 代表历史心跳. 本任务基于 `DEFAULT_HEARTBEAT_TIMEOUT_SECS = 5` 默认常量计算 `heartbeat_stale(心跳陈旧)` 字段, 不新增 `SupervisorSpec.heartbeat_timeout(监督器声明心跳超时)` 字段. 本任务同时补齐依赖 T007 公开类型的 `ChildRuntimeState::to_record(生成运行状态记录)` 方法, 状态记录必须包含 `failure(失败原因)` 字段.
- [ ] T020 [US1] 在 `src/runtime/control_loop.rs` 和 `src/runtime/child_runtime_state.rs` 引入 runtime(运行时) 侧重启次数限制跟踪结构(例如 `RestartLimitTracker(重启次数限制跟踪器)`), 在每次 `handle_child_exit(处理子任务退出)` 评估后计算 `used / remaining / exhausted`, 再写回 `ChildRuntimeState.restart_limit(子任务运行状态记录重启次数限制状态)`. `window / limit(窗口与上限)` 必须从既有 `RestartLimit(重启次数限制)` 来源解析, 优先级为 child strategy override(子任务策略覆盖), group strategy(分组策略), supervisor spec(监督器声明) 和配置层默认 `PolicyConfig.child_restart_limit / child_restart_window_ms(策略配置子任务重启上限与窗口)`. `updated_at_unix_nanos(更新时间)` 必须通过 T019 挂在 `RuntimeControlState(运行时控制状态)` 上的 `RuntimeTimeBase(运行时时间基准)` 生成, 并在当前值小于或等于前一次值时写入 `previous + 1(前值加一)` 保证单调递增. 本任务不得假设可从无状态 `PolicyEngine(策略引擎)` 或 `RestartPolicy(重启策略)` 直接读取 `used / remaining(已使用与剩余)` 运行时历史.
- [ ] T021 [P] [US1] 在 `src/observe/metrics.rs` 中实现 `supervisor_child_runtime_restart_limit_remaining(子任务运行状态记录剩余重启次数)` gauge(仪表) 的写入路径, 每次 `RestartLimitState(重启次数限制状态)` 刷新时同步更新.
- [ ] T022 [P] [US1] 在 `src/observe/pipeline.rs` 中实现 `ChildHeartbeatStale(子任务运行状态记录心跳陈旧)` 事件发布与 `supervisor_child_runtime_heartbeat_stale_total(子任务运行状态记录心跳陈旧总数)` counter(计数器) 的写入路径. 同一 `(child_id, attempt)` 在该 attempt(尝试) 终止前最多发布一次事件, counter(计数器) 仅在抑制规则允许并实际发布事件时增加 1, 且该 counter(计数器) 不得携带 `child_id(子任务标识)` 标签.

**Checkpoint(检查点)**: `cargo test --test supervisor_child_runtime_state_control_test current_state_` 中以 `current_state_` 开头的三个测试通过. `CurrentState(当前状态)` 已经能返回完整运行状态字段, 第一个 user story(用户故事) 已经独立可用.

---

## Phase 4(阶段四): User Story 2(用户故事二) - 控制命令停止真实运行任务 (Priority(优先级): P2)

**Goal(目标)**: `PauseChild(暂停子任务)`, `RemoveChild(移除子任务)`, `QuarantineChild(隔离子任务)` 必须把控制命令对应的操作应用到 `ChildRuntimeState(子任务运行状态记录)` 的真实活动尝试上, 包括取消, 等待与移除, 不再只更新操作枚举.

**Independent Test(独立测试)**: 启动一个长运行任务, 分别执行三类停止命令, 在 child(子任务) 中观察 `is_cancelled()` 为真; 用 `CurrentState(当前状态)` 验证 `status` 推进到 `Cancelling(取消中)` 然后 `Stopped(已停止)`, operation(操作) 与命令对应.

### Tests for User Story 2(用户故事二的测试)

> **NOTE(说明): 先写下列五个测试函数, 并确认它们在实现前失败.**
> **NOTE(性能复用)**: 下列测试只要通过 `CurrentState(当前状态)` 读取运行状态事实, 就必须复用 T015 的 `assert_current_state_fast_20_reads(断言当前状态二十次快速读取)` 辅助函数, 继续验证连续 20 次构造每次低于 1 毫秒.

- [ ] T023 [US2] 在 `src/tests/supervisor_child_runtime_state_control_test.rs` 中添加 `pause_child_delivers_real_cancellation_test` 测试: 启动长运行 child(子任务), 发出 `PauseChild(暂停子任务)`, 验证 child 上下文 `is_cancelled()` 为真, 运行状态记录 `operation = Paused(已暂停)`, `status = Cancelling(取消中)`, `cancel_delivered = true(已送达)`, 并发出 `ChildControlCancelDelivered(子任务控制取消已送达)` 与 `ChildControlOperationChanged(子任务控制操作变化)` 事件. `ChildControlCancelDelivered(子任务控制取消已送达)` 事件必须断言 `child_id`, `generation`, `attempt`, `command`, `command_id` 全部等于本次命令和目标活动尝试.
- [ ] T024 [US2] 在同测试文件中添加 `remove_child_cancels_and_eventually_removes_runtime_state_test` 测试: 启动长运行 child(子任务), 发出 `RemoveChild(移除子任务)`, 等待 child 退出, 验证运行状态记录最终从 `child_runtime_states` 中删除, 发出 `ChildRuntimeStateRemoved(子任务运行状态记录已移除)` 事件且 `path(路径)` 等于该 child(子任务) 在 supervisor tree(监督树) 中的路径, `final_status = Some(有值)`, `CurrentState.child_runtime_records` 不再包含该 child. 该断言依赖 T018 已经实现的 `build_current_state(构造当前状态)` 行为, 所以 T024 必须在 US1 通过后才有完整验收意义.
- [ ] T025 [US2] 在同测试文件中添加 `quarantine_child_blocks_auto_restart_test` 测试: 启动一个会以 `Failed(失败)` 退出的 child(子任务) 并配置可自动重启策略, 发出 `QuarantineChild(隔离子任务)`, 让当前 attempt(尝试) 退出, 验证 supervision strategy(监督策略) 不触发新 attempt(尝试), 运行状态记录 `operation = Quarantined(已隔离)` 仍可在 `CurrentState(当前状态)` 中观察.
- [ ] T026 [US2] 在同测试文件中添加 `pause_child_blocks_auto_restart_after_exit_test` 测试: 对运行中 child(子任务) 发出 `PauseChild(暂停子任务)`, 让当前 attempt(尝试) 退出, 验证 supervision strategy(监督策略) 不触发新 attempt(尝试), 运行状态记录保持 `operation = Paused(已暂停)` 并且 `generation` 与 `attempt` 不递增.
- [ ] T027 [US2] 在同测试文件中添加 `control_command_targets_current_attempt_test` 测试: 让 child(子任务) 自动重启切换到新 attempt(尝试), 然后发出 `PauseChild(暂停子任务)`, 验证 `ChildControlResult.attempt` 指向当前活动 attempt, 旧 attempt 不再被发送取消.

### Implementation for User Story 2(用户故事二的实现)

**Implementation note(实现说明)**: 允许先完成 T029 抽出 `apply_stop_control_to_runtime_state(应用停止控制到运行状态记录)`, 再实现 T028, T030, T031 三个分支一律经该 helper(辅助函数) 调用, 以避免先写满 T028 再抽 T029 时的重复劳动.

- [ ] T028 [US2] 在 `src/runtime/control_loop.rs` 重写 `ControlCommand::PauseChild(暂停子任务控制命令)` 分支: 读取 `operation_before(命令前操作)` 和既有 `attempt_cancel_delivered(尝试取消已送达)`, 仅在 `operation_before != Paused(命令前操作不是已暂停)` 时把运行状态记录 `operation(操作)` 设为 `Paused(已暂停)` 并发出 `ChildControlOperationChanged(子任务控制操作变化)` 事件; 仅在存在活动 attempt(尝试) 且 `(operation_before != Paused || attempt_cancel_delivered == false)` 时调用一次 `runtime_state.cancel(运行状态记录取消)`, 把 `status(状态)` 推进到 `Cancelling(取消中)`, `stop_state(停止状态)` 推进到 `CancelDelivered(已送达取消)`, 并发出 `ChildControlCancelDelivered(子任务控制取消已送达)` 事件. 如果存在活动 attempt(尝试), 但 `operation_before = Paused(已暂停)` 且既有 `attempt_cancel_delivered = true(尝试取消已送达)`, 必须返回 `idempotent = true(幂等是)` 且不得重复取消. 无活动 attempt(尝试) 时必须返回 `stop_state = NoActiveAttempt(无活动尝试)` 与 `cancel_delivered = false(否)`.
- [ ] T029 [US2] 在 `src/runtime/control_loop.rs` 中抽取停止类命令共享 helper(辅助函数) `apply_stop_control_to_runtime_state(应用停止控制到运行状态记录)`, 统一处理 `PauseChild(暂停子任务)`, `RemoveChild(移除子任务)` 和 `QuarantineChild(隔离子任务)` 的 `operation_before(命令前操作)`, `operation_after(命令后操作)`, 既有 `attempt_cancel_delivered(尝试取消已送达)`, 本次结果 `cancel_delivered(取消已送达)`, `status(状态)`, `stop_state(停止状态)` 与 `idempotent(幂等)` 字段. helper(辅助函数) 必须保证事件只在实际操作变化或实际取消送达时发布; 当活动 attempt(尝试) 已经处于目标操作且已经送达取消时, helper(辅助函数) 必须返回幂等结果并跳过取消.
- [ ] T030 [US2] 在 `src/runtime/control_loop.rs` 重写 `ControlCommand::RemoveChild(移除子任务控制命令)` 分支: 读取 `operation_before(命令前操作)` 和既有 `attempt_cancel_delivered(尝试取消已送达)`, 把 `operation(操作)` 设为 `Removed(已移除)`, 仅在存在活动 attempt(尝试) 且 `(operation_before != Removed || attempt_cancel_delivered == false)` 时调用一次 `runtime_state.cancel(运行状态记录取消)`; 如果存在活动 attempt(尝试), 但 `operation_before = Removed(已移除)` 且既有 `attempt_cancel_delivered = true(尝试取消已送达)`, 必须返回 `idempotent = true(幂等是)` 且不得重复取消. 同步修改 `handle_child_exit(处理子任务退出)`, 在运行状态记录 `operation = Removed(已移除)` 且活动 attempt(尝试) 已退出时从 `child_runtime_states` 中物理删除并发出 `ChildRuntimeStateRemoved(子任务运行状态记录已移除)` 事件. 当 `RemoveChild(移除子任务)` 命中无活动 attempt(尝试) 的运行状态记录时, 必须先构造 `stop_state = NoActiveAttempt(无活动尝试)`, `attempt = None(无值)`, `cancel_delivered = false(否)`, `idempotent = false(否)` 的结果, 再在同一轮命令处理末尾物理删除运行状态记录并发出 `ChildRuntimeStateRemoved(子任务运行状态记录已移除)` 事件, 事件 `final_status = None(无值)`.
- [ ] T031 [US2] 在 `src/runtime/control_loop.rs` 重写 `ControlCommand::QuarantineChild(隔离子任务控制命令)` 分支: 读取 `operation_before(命令前操作)` 和既有 `attempt_cancel_delivered(尝试取消已送达)`, 仅在 `operation_before != Quarantined(命令前操作不是已隔离)` 时把 `operation(操作)` 设为 `Quarantined(已隔离)` 并发出 `ChildControlOperationChanged(子任务控制操作变化)` 事件; 仅在存在活动 attempt(尝试) 且 `(operation_before != Quarantined || attempt_cancel_delivered == false)` 时调用一次 `runtime_state.cancel(运行状态记录取消)`; 如果存在活动 attempt(尝试), 但 `operation_before = Quarantined(已隔离)` 且既有 `attempt_cancel_delivered = true(尝试取消已送达)`, 必须返回 `idempotent = true(幂等是)` 且不得重复取消. 同步修改 `handle_child_exit(处理子任务退出)`, 在运行状态记录 `operation = Quarantined(已隔离)` 时跳过 `PolicyEngine(策略引擎)` 重启评估. 已隔离运行状态记录后续收到 `RemoveChild(移除子任务)` 时必须按 T030 的 `Quarantined(已隔离) -> Removed(已移除)` 路径删除.
- [ ] T032 [US2] 在 `src/runtime/control_loop.rs` 更新自动重启分支: `handle_child_exit(处理子任务退出)` 只在运行状态记录 `operation = Active(活跃)` 时允许 `PolicyEngine(策略引擎)` 触发新 attempt(尝试), 在 `Paused(已暂停)` 或 `Quarantined(已隔离)` 时必须跳过自动重启, 在 `Removed(已移除)` 时必须删除运行状态记录.
- [ ] T033 [US2] 在 `src/runtime/control_loop.rs` 修改 `handle_child_exit(处理子任务退出)`, 当运行状态记录 `stop_state = CancelDelivered(已送达取消)` 时推进到 `Completed(已停止)` 并发出 `ChildControlStopCompleted(子任务控制停止完成)` 事件, 字段包含 `child_id / generation / attempt / exit_kind`.
- [ ] T034 [P] [US2] 在 `src/observe/metrics.rs` 接入 `supervisor_child_control_command_total(子任务控制命令总数)` 与 `supervisor_child_runtime_operation_transitions_total(子任务控制操作转换总数)` 计数器, 在每条控制命令处理后写入 `command(命令名)` 与 `result(结果分类)`, 操作变化时写入 `from / to`.

**Checkpoint(检查点)**: `cargo test --test supervisor_child_runtime_state_control_test` 中 US2 五个测试通过. 三类停止命令真实把取消送达 child future(子任务 future). 第二个 user story(用户故事) 与第一个故事可以共同工作.

---

## Phase 5(阶段五): User Story 3(用户故事三) - 让控制结果反映运行状态事实 (Priority(优先级): P3)

**Goal(目标)**: 控制命令返回的 `ChildControlResult(子任务控制结果)` 必须完整包含目标 child(子任务) 标识, 目标 attempt(尝试) 标识, 取消送达情况, 停止状态, 剩余重启次数, 失败原因和 `idempotent(幂等)` 标记. exit handler(退出处理) 必须把 `stop_state(停止状态)` 推进到最终态并写 audit(审计).

**Independent Test(独立测试)**: 对处于不同状态的运行状态记录执行控制命令, 验证 outcome 字段一致; 对已经处于目标操作且仍存在于 `child_runtime_states(子任务运行状态记录集合)` 中的已停止或从未启动运行状态记录重复执行同一停止类命令 10 次, 全部幂等返回; 让任务忽略取消, 验证超时后 outcome.failure 字段携带 phase 与 reason.

### Tests for User Story 3(用户故事三的测试)

> **NOTE(说明): 先写下列五个测试函数, 并确认它们在实现前失败.**
> **NOTE(性能复用)**: 下列测试只要通过 `CurrentState(当前状态)` 读取运行状态事实, 就必须复用 T015 的 `assert_current_state_fast_20_reads(断言当前状态二十次快速读取)` 辅助函数, 继续验证连续 20 次构造每次低于 1 毫秒.

> **NOTE(补充)**: T035 与 T036 可在 Foundational(阶段二) 完成后相对独立地以编译或断言失败驱动. T037 必须作为可执行失败测试提交, 不得使用 `#[ignore]` 或只提交不可运行骨架. 在 T042 合并前, T037 可以通过测试专用 `ShutdownPolicy.graceful_timeout(关闭策略优雅等待时间)` 短窗口和忽略取消的协作式 child(子任务) 稳定失败, 从而满足先写测试原则.

- [ ] T035 [US3] 在 `src/tests/supervisor_child_runtime_state_control_test.rs` 中添加 `repeated_stop_commands_are_idempotent_test` 测试: 分别为 `PauseChild(暂停子任务)`, `RemoveChild(移除子任务)` 和 `QuarantineChild(隔离子任务)` 准备独立运行状态记录, 先让记录达到目标操作且已经向活动 attempt(尝试) 送达取消, 再对同一命令重复执行 10 次, 验证 `ChildControlResult.idempotent = true(是)`, `cancel_delivered = false(否)`, `operation_before = operation_after(命令前后操作一致)`, 且不重复发出 `ChildControlCancelDelivered(子任务控制取消已送达)` 或 `ChildControlOperationChanged(子任务控制操作变化)` 事件. 同一测试还必须覆盖无活动 attempt(尝试) 且不会触发物理删除的已暂停和已隔离运行状态记录, 重复 `PauseChild(暂停子任务)` 与 `QuarantineChild(隔离子任务)` 时同样返回幂等结果; `RemoveChild(移除子任务)` 的无活动首次删除路径由 T036 覆盖, 不在此处伪造成幂等.
- [ ] T036 [US3] 在同测试文件中添加 `remove_without_active_attempt_returns_no_active_attempt_test` 测试: 对一个尚未启动 child(子任务) 的注册运行状态记录发出 `RemoveChild(移除子任务)`, 验证 outcome `stop_state = NoActiveAttempt(无活动尝试)`, `attempt = None(无值)`, `generation = None(无值)`, `status = None(无值)`, `cancel_delivered = false(否)`, `operation_after = Removed(已移除)`, `idempotent = false(否)`. 同一测试必须验证运行状态记录在结果构造后从 `child_runtime_states(子任务运行状态记录集合)` 中物理删除, 并发出 `ChildRuntimeStateRemoved(子任务运行状态记录已移除)` 事件且 `path(路径)` 等于该 child(子任务) 在 supervisor tree(监督树) 中的路径, `final_status = None(无值)`. 规格依据见 `spec.md` Edge Cases(边界情况) 中 "注册占位但无活动尝试" 一条.
- [ ] T037 [US3] 在同测试文件中添加 `stop_failure_outcome_carries_phase_and_reason_test` 测试: 让 child(子任务) 忽略取消信号, 通过测试 supervisor runtime(监督器运行时) 已生效的 `ShutdownPolicy.graceful_timeout(关闭策略优雅等待时间)` 配置较短停止等待窗口, 并确认 `abort_after_timeout(超时后强制中止)` 不影响控制命令路径. 发出 `RemoveChild(移除子任务)` 后等待超过 `stop_deadline_at_unix_nanos(停止截止时间)`. 然后再发送 `CurrentState(当前状态)` 或重复 `RemoveChild(移除子任务)` 以触发 `reconcile_stop_deadlines(调和停止截止时间)`, 验证后一次 `outcome.failure(控制结果失败原因)` 或 `ChildRuntimeRecord.failure(运行状态记录失败原因)` 为 `Some(有值)`, `phase = ChildControlFailurePhase::WaitCompletion(子任务控制失败阶段为等待完成)`, `reason(原因)` 非空, 并验证 `ChildControlStopFailed(子任务控制停止失败)` 事件包含 `child_id`, `generation`, `attempt`, `status`, `stop_state`, `phase`, `reason` 和 `recoverable`.
- [ ] T038 [US3] 在同测试文件中添加 `restart_limit_exhaustion_visible_in_outcome_test` 测试: 通过 child strategy override(子任务策略覆盖) 或 supervisor spec(监督器声明) 配置较短 `RestartLimit(重启次数限制)` 窗口和较小上限, 让 child(子任务) 连续 `Failed(失败)` 直到耗尽重启次数限制, 验证 outcome.restart_limit.remaining 在最后一次失败后等于 0, `exhausted = true(已耗尽)`, 且 `window / limit(窗口与上限)` 与配置来源一致; 此时发出 `PauseChild(暂停子任务)` 验证 outcome.restart_limit 字段与运行状态字段一致. 测试必须覆盖 `used(已使用)` 大于 `limit(上限)` 时 `remaining(剩余)` 仍等于 0, 即实现使用 `limit.saturating_sub(used)`(上限对已使用次数做饱和相减). 同一测试必须触发至少 2 次重启次数限制刷新, 并验证后一次 `RestartLimitState.updated_at_unix_nanos(重启次数限制状态更新时间)` 大于前一次; 如果测试夹具让两次刷新落在同一纳秒或系统时间回拨, 实现仍必须通过 `previous + 1(前值加一)` 规则保持递增.
- [ ] T039 [US3] 在同测试文件中添加 `operation_wins_over_auto_restart_race_test` 测试: 让 child(子任务) 即将由 `PolicyEngine(策略引擎)` 决定自动重启, 使用 research.md 决策九指定的测试夹具门控让 child(子任务) 在返回失败前等待测试释放. 测试必须在释放 child(子任务) 退出前先发出 `PauseChild(暂停子任务)` 并确认 `operation = Paused(已暂停)`, 然后释放退出并验证后续 exit handler(退出处理) 不启动新 attempt(尝试). 本任务不得引入 `tokio::time::pause(暂停时间)` 依赖策略, 也不得在 `control_loop(控制循环)` 增加仅测试可见的生产代码钩子.

### Implementation for User Story 3(用户故事三的实现)

- [ ] T040 [US3] 在 `src/runtime/control_loop.rs` 实现 `build_child_control_outcome(runtime_state, command, operation_before, cancel_delivered, idempotent, failure)` 私有函数, 输出 `ChildControlResult(子任务控制结果)`, 字段映射: `attempt / generation` 取自运行状态记录活动尝试, 无活动 attempt(尝试) 时为 `None(无值)`, `operation_after` 取自运行状态记录 operation(操作), `status` 取自运行状态记录 status(状态), `stop_state` 取自运行状态记录 stop_state(停止状态), `restart_limit` 取自运行状态记录 restart_limit(重启次数限制) 状态, `liveness` 取自 `runtime_state.observe_liveness(观察存活)` 后的状态, `failure` 由调用方提供.
- [ ] T041 [US3] 在 `src/runtime/control_loop.rs` 修改 `Pause / Remove / Quarantine` 三个分支, 全部返回 `CommandResult::ChildControl(子任务控制命令结果) { outcome: build_child_control_outcome(...) }`. 幂等判断: 命令到达时 `operation_before == operation_after` 且未真正调用 `runtime_state.cancel(运行状态记录取消)` 且未触发物理删除时设 `idempotent = true(是)` 且本次 `cancel_delivered = false(否)`. 活动 attempt(尝试) 已经处于目标操作且既有取消已送达时必须命中该幂等分支, 不得重复取消. 无活动 attempt(尝试) 不自动代表幂等; 若命令改变操作或删除运行状态记录, 必须返回 `idempotent = false(否)`. `RestartChild(重启子任务)` 与 `ResumeChild(恢复子任务)` 不在本功能新语义范围内, 只需保持既有回归测试不破坏.
- [ ] T042 [US3] 在 `src/runtime/control_loop.rs` 中实现 `reconcile_stop_deadlines(调和停止截止时间)` 私有函数, 并在每次处理 `ControlCommand(控制命令)`, `CurrentState(当前状态)` 和 `handle_child_exit(处理子任务退出)` 收尾前调用. 当运行状态记录 `stop_state = CancelDelivered(已送达取消)` 且 `stop_deadline_at_unix_nanos(停止截止时间)` 已经过期, 同时 child(子任务) 仍未退出时, 把 `stop_state` 推进到 `Failed(停止失败)`, 写入 `last_control_failure(最近控制失败原因)`, 构造 `ChildControlFailure(子任务控制失败原因)` 且 `phase = ChildControlFailurePhase::WaitCompletion(子任务控制失败阶段为等待完成)`, 并发出 `ChildControlStopFailed(子任务控制停止失败)` 事件. 事件字段必须包含 `child_id`, `generation`, `attempt`, `status`, `stop_state`, `phase`, `reason`, `recoverable`. `stop_deadline_at_unix_nanos(停止截止时间)` 必须由取消送达时刻加当前 supervisor runtime(监督器运行时) 已生效的 `ShutdownPolicy.graceful_timeout(关闭策略优雅等待时间)` 得到. 本任务采用 lazy-only(惰性触发) 语义, 不新增 timer(定时器) 或内部唤醒消息. 注意: 本任务**不**调用 `runtime_state.abort()`(强制中止句柄). 控制命令路径只使用 `runtime_state.cancel()`(软取消); `runtime_state.abort()` 仅由 `004-2-real-shutdown-pipeline` 的 `ShutdownPipeline`(关闭流水线) 在关闭 `supervisor tree`(监督树) 时调用. 控制命令路径忽略 `abort_after_timeout(超时后强制中止)` 策略标志, 该标志是 `ShutdownPipeline` 的配置.
- [ ] T043 [P] [US3] 在 `src/observe/pipeline.rs` 实现 `audit_child_control(命令审计)` 路径, 写入 `command_id`, `requested_by`, `reason`, `child_id`, `generation`, `attempt`, `status`, `operation_before`, `operation_after`, `cancel_delivered`, `stop_state`, `restart_limit_remaining`, `idempotent`, `failure` 字段, 与现有 audit 风格保持一致.
- [ ] T044 [P] [US3] 在 `src/dashboard/model.rs` 把 `ChildControlResult(子任务控制结果)` 与 `ChildRuntimeRecord(子任务运行状态记录)` 映射到 dashboard(仪表盘) 状态展示模型, 明确这是返回结果形状升级; 同时确认 `dashboard_protocol_shape_test` 仅保护 IPC(进程间通信) 请求字段不漂移, 另用 dashboard model(仪表盘模型) 测试覆盖升级后的返回结果字段. 在映射 `ChildRuntimeRecord` 到 dashboard model 时, 根据 `operation` 值按 `contracts/child-runtime-state-control.md` 中 `Operation Mapping` 表派生对应的 `ManagedChildState`(受管子任务状态) 值, 确保对外展示的两者在 audit(审计) 中保持一一对应.

**Checkpoint(检查点)**: `cargo test --test supervisor_child_runtime_state_control_test` 中 US3 五个测试通过. 控制命令结果含完整运行状态事实, 三个 user story(用户故事) 全部独立可用且彼此组合可用.

---

## Phase 6(最终阶段): Polish & Cross-Cutting Concerns(收尾和横向事项)

**Purpose(目的)**: 完成文档同步, 跨模块兼容验证, 与全量测试.

- [ ] T045 [P] 在 `manual/zh/runtime-control.md` 中更新章节, 说明 `PauseChild(暂停子任务)`, `RemoveChild(移除子任务)`, `QuarantineChild(隔离子任务)` 与 `CurrentState(当前状态)` 的新语义, 列出 `ChildControlResult(子任务控制结果)` 与 `ChildRuntimeRecord(子任务运行状态记录)` 字段含义, 并说明 `RestartChild(重启子任务)` 与 `ResumeChild(恢复子任务)` 只是既有命令回归范围, 不属于本规格新增生命周期语义, 链接 `specs/004-3-child-runtime-state-control/contracts/child-runtime-state-control.md`.
- [ ] T046 在 `src/runtime/shutdown_pipeline.rs` 验证 `ShutdownPipeline(关闭流水线)` 在运行状态记录 `operation = Paused / Quarantined(已暂停 / 已隔离)` 时仍能 `cancel(取消)` 与 `wait_for_report(等待报告)`, 在 `operation = Removed(已移除)` 时跳过. 必须在 `src/tests/supervisor_real_shutdown_pipeline_test.rs` 增加或更新 3 个最小回归断言, 分别覆盖 `Paused(已暂停)` 取消与等待, `Quarantined(已隔离)` 取消与等待, `Removed(已移除)` 跳过关闭路径.
- [ ] T047 更新 `src/tests/naming_contract_test.rs` 的 `source_code_uses_approved_state_names` 断言, 加入本功能全部新增公开类型: `ChildAttemptStatus(子任务尝试状态)`, `ChildControlOperation(子任务控制操作)`, `ChildStopState(子任务停止状态)`, `ChildControlFailurePhase(子任务控制失败阶段)`, `ChildControlFailure(子任务控制失败原因)`, `RestartLimitState(重启次数限制状态)`, `ChildLivenessState(子任务存活状态)`, `ChildRuntimeState(子任务运行状态记录)`, `ChildRuntimeRecord(子任务运行状态记录)`, `ChildControlResult(子任务控制结果)` 和 `ReadinessState(就绪状态)`, 并继续覆盖既有 `ConfigState(配置状态)`, `SupervisorState(监督器状态)` 和 `current_state(当前状态)` 断言. 删除或替换旧 `ChildState(子任务状态)` 断言, 不得让命名契约继续要求已删除的 `CommandResult::ChildState(子任务状态命令结果)` 变体存在.
- [ ] T048 在仓库根目录运行 `cargo fmt`, 提交格式修正(如有). 本任务会格式化多个 Rust(编程语言) 文件, 不得标记为 `[P]`.
- [ ] T049 在仓库根目录运行 `cargo test --test supervisor_child_runtime_state_control_test --test supervisor_control_test --test supervisor_real_shutdown_pipeline_test --test supervisor_runtime_lifecycle_test --test supervisor_shutdown_test --test observability_smoke_test --test dashboard_protocol_shape_test --test supervisor_examples_test --test control_test`, 确认控制命令请求字段没有漂移, `CommandResult::ChildState(子任务状态命令结果)` 到 `ChildControl(子任务控制)` 的调用结果替换与 `CurrentState.child_runtime_records(当前状态子任务运行状态记录集合)` 调用结果扩展在全部受影响测试中通过. 然后运行 `cargo test --test naming_contract_test source_code_uses_approved_state_names`, 确认命名契约覆盖本功能新增公开类型. `supervisor_child_runtime_state_control_test(子任务运行状态控制测试)` 和 `control_test(控制测试)` 必须来自 `Cargo.toml` 注册目标, `dashboard_protocol_shape_test(仪表盘协议形状测试)` 必须来自既有 `tests/dashboard_protocol_shape_test.rs` 自动发现目标.
- [ ] T050 在仓库根目录运行 `cargo test -- --skip checked_artifacts_avoid_forbidden_state_terms` 完成近似全量验收, 然后运行 `cargo test` 完成完整验收. 如果完整验收失败点来自 sibling UI(同级用户界面) 命名契约, 则必须记录阻塞测试名称和失败断言, 并与 sibling UI(同级用户界面) 命名契约修复一同协调.

---

## Dependencies & Execution Order(依赖和执行顺序)

### Phase Dependencies(阶段依赖)

- **Setup(阶段一)**: 没有依赖, 可立即开始.
- **Foundational(阶段二)**: 依赖 Setup(阶段一) 完成, 阻塞全部 user story(用户故事).
- **User Stories(用户故事阶段)**: 全部依赖 Foundational(阶段二) 完成. US1 可独立交付 MVP(最小可用产品). US2 的 `CurrentState.child_runtime_records(当前状态子任务运行状态记录集合)` 断言依赖 US1 的 `build_current_state(构造当前状态)` 行为, 因此 US2 必须在 US1 通过后完整验收. US3 依赖 US2 的停止命令结果字段, 因此 US3 建议在 US2 实现任务之后启动.
- **Polish(收尾阶段)**: 依赖三个 user story(用户故事) 全部完成.

### User Story Dependencies(用户故事依赖)

- **US1(P1)**: Foundational 完成后可立即开始, 与 US2 / US3 无前置依赖.
- **US2(P2)**: Foundational 完成后可以准备停止命令测试夹具, 但完整测试与验收依赖 US1 的 `build_current_state(构造当前状态)` 已经完成. T024 中对 `CurrentState.child_runtime_records(当前状态子任务运行状态记录集合)` 的断言必须在 T018 之后执行.
- **US3(P3)**: Foundational 完成后可开始, 但 `build_child_control_outcome(构造子任务控制结果)` 在 US2 实现完成后才能填齐 `operation_before / cancel_delivered` 字段, 因此 US3 的实现任务建议在 US2 实现任务之后启动.

### Within Each User Story(每个用户故事内部)

- 行为变化的测试必须先写, 实现前确认测试失败.
- 先写公开结果类型, 再写运行时类型方法(`ChildRuntimeState::observe_liveness`, `ChildRuntimeState::to_record`), 最后写 `control_loop(控制循环)` 中的调用.
- 先写 outcome 构造, 再写 audit 与 metric 写入.

### Parallel Opportunities(并行机会)

- Setup(阶段一): T003 必须在 T001 与 T002 均已合并为可编译工作区之后执行, 不得在 T001 或 T002 仅完成一半时把 `cargo test` 当作并行基线. 若需要记录未改动的纯基线, 应先在干净提交上单独跑完 T003, 再开始 T001 与 T002 的占位改动.
- Foundational Tests(基础阶段测试): T004 与 T005 都改 `src/tests/supervisor_control_test.rs`, 必须串行; T006 改 `src/tests/supervisor_real_shutdown_pipeline_test.rs`, 可与 T004 或 T005 并行. Foundational Implementation(基础阶段实现) 必须在 Foundational Tests 全部写完后才能开始, 不得与 T004-T006 并行.
- Foundational Implementation 内部: T007, T009, T010 修改不同文件且不依赖其他基础实现任务, 可并行. T008 依赖 T007 的公开 outcome(结果) 类型, 不标 `[P]`, 必须在 T007 后执行. T011, T012, T013, T014 各自有依赖关系, 串行.
- US1 测试 T015-T017 都改同一测试文件, 串行. 实现任务中, T019 修改 `src/runtime/child_runtime_state.rs` 并提供 `ChildRuntimeState::to_record(生成运行状态记录)`, T018 修改 `src/runtime/control_loop.rs` 并调用该方法. 两者可在测试写完后分工起草, 但编译检查必须在 T018 和 T019 都完成后执行. T020 同时修改 `src/runtime/control_loop.rs` 与 `src/runtime/child_runtime_state.rs`, 必须在 T018 和 T019 之后串行执行. T021 修改 `src/observe/metrics.rs`, T022 修改 `src/observe/pipeline.rs`, 两者可在 T020 的重启次数限制和存活状态写入点稳定后并行.
- US2 测试 T023-T027 都改同一测试文件, 串行; 实现任务 T028-T033 都改 `control_loop.rs`, 串行; T034 与 control loop 实现并行.
- US3 测试 T035-T039 都改同一测试文件, 串行; 实现任务 T040-T042 都改 `control_loop.rs`, 串行; T043, T044 可与 control loop 实现并行.

---

## Parallel Example(并行示例): Foundational Implementation After Tests(基础测试后的基础实现)

```bash
# 先完成 T004, T005, T006 三个 Foundational Tests(基础阶段测试), 再并行执行下列三个独立实现任务:
Task(任务): "在 src/control/outcome.rs 定义 ChildAttemptStatus, ChildControlOperation, ChildStopState, RestartLimitState 等公开类型"
Task(任务): "在 src/event/payload.rs 添加六个新事件变体"
Task(任务): "在 src/observe/metrics.rs 注册四个新指标"

# T007 完成后再执行:
Task(任务): "在 src/runtime/child_runtime_state.rs 定义 ChildRuntimeState 并引用 src/control/outcome.rs 的公开类型"
```

---

## Implementation Strategy(实现策略)

### MVP First(先做最小可用产品)

1. 完成 Phase 1(阶段一) Setup(初始化).
2. 完成 Phase 2(阶段二) Foundational(基础). 现有测试套件中与 `CommandResult::ChildState(子任务状态命令结果)` 相关的断言会失败, 必须在本阶段同步调整.
3. 完成 Phase 3(阶段三) User Story 1(用户故事一), 让操作者可以读到真实运行状态字段. 这是本功能的 MVP(最小可用产品).
4. 停止并验证 `cargo test --test supervisor_child_runtime_state_control_test`, 确认 US1 测试通过.
5. 可在此时演示或交付 US1.

### Incremental Delivery(增量交付)

1. Setup + Foundational 提交一组.
2. US1 提交一组, 独立测试通过.
3. US2 提交一组, 与 US1 共同验证, 测试通过.
4. US3 提交一组, 与 US1 / US2 共同验证, 测试通过.
5. Polish 完成最后一组, 全量 `cargo test` 通过.

### Parallel Team Strategy(并行团队策略)

1. 团队一起完成 Setup 与 Foundational.
2. Foundational 完成后, 先由 US1 人员完成 `CurrentState(当前状态)` 运行状态记录行为. US1 通过后再由不同人员分别接手 US2 / US3 测试任务. 由于实现任务大量集中在 `src/runtime/control_loop.rs`, 实现阶段建议由同一人按 US1, US2, US3 顺序推进, 外部 audit(审计), metric(指标) 和 dashboard(仪表盘) 任务可由另一人并行.

---

## Notes(说明)

- 本功能新增主测试写入外部测试文件 `src/tests/supervisor_child_runtime_state_control_test.rs`. 既有回归测试按 T004-T006 写入 `src/tests/supervisor_control_test.rs` 与 `src/tests/supervisor_real_shutdown_pipeline_test.rs`. T014 只更新 `src/control/tests/control_test.rs` 这个已注册外部测试目标中的既有断言. 全部测试都不写入 `src/` 模块文件内联.
- `[P]` 标记仅在任务修改不同文件且无前置依赖时使用.
- 项目宪章禁止 compatibility export(兼容导出), `CommandResult::ChildState(子任务状态命令结果)` 必须直接替换为 `ChildControl(子任务控制)`, 不添加类型别名.
- 每完成一个 user story(用户故事) 后建议提交一次, commit message(提交消息) 风格与 `004-2-real-shutdown-pipeline` 保持一致.
- `manual/zh/runtime-control.md` 同步章节属于本功能的最终交付项, 不得遗漏.
