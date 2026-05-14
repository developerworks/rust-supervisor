# Implementation Plan(实现计划): 子任务运行状态控制

**Branch(分支)**: `004-runtime-semantics` | **Date(日期)**: 2026-05-15 | **Spec(规格)**: `specs/004-3-child-runtime-state-control/spec.md`
**Input(输入)**: 功能规格来自 `specs/004-3-child-runtime-state-control/spec.md`

## Summary(摘要)

本功能把 `PauseChild(暂停子任务)`, `RemoveChild(移除子任务)`, `QuarantineChild(隔离子任务)` 三条控制命令的语义从 "只改 `ManagedChildState(受管子任务状态)` 枚举" 修正为 "作用于真实活动尝试". 修正后, runtime(运行时) 必须为每个声明 `child(子任务)` 维护一个 `ChildRuntimeState(子任务运行状态记录)`, 运行状态记录在有活动 attempt(尝试) 时挂载 `cancellation_token(取消令牌)`, `abort_handle(强制中止句柄)`, `completion_receiver(完成接收端)`, 心跳和就绪状态的 `watch::Receiver(观察接收端)`, 并始终暴露 `restart_limit(重启次数限制)` 剩余次数状态. 控制命令到达 control loop(控制循环) 时, 必须先按幂等规则判断是否需要在运行状态记录上发起真实取消, 再返回 `ChildControlResult(子任务控制结果)`, 其中包含目标 `child id(子任务标识)`, 目标 `attempt(尝试)` 标识或 `None(无值)`, 取消送达情况, 当前 `stop_state(停止状态)`, `restart_limit(重启次数限制)` 剩余次数和失败时的类型化阶段与原因.

`ChildRuntimeState(子任务运行状态记录)` 是 `004-2-real-shutdown-pipeline` 中 `ActiveChildAttempt(活动子任务尝试)` 的能力升级, 它取代旧活动尝试集合的语义, 并形成新的 `RuntimeControlState.child_runtime_states(运行时控制状态子任务运行状态记录集合)` 字段, 同时向 control loop(控制循环) 暴露心跳与就绪状态的只读状态. shutdown pipeline(关闭流水线) 继续复用同一份运行状态记录句柄, 不另起一套关闭路径. registry(注册表) 继续保存 `ChildRuntime(子任务运行时记录)` 作为声明性事实, 但活动尝试的所有运行时句柄归属 `ChildRuntimeState(子任务运行状态记录)`.

`ChildControlResult(子任务控制结果)` 是新公开类型, 属于 `src/control/outcome.rs`. `CommandResult::ChildState(子任务状态命令结果)` 变体升级为 `CommandResult::ChildControl(子任务控制命令结果)`, 携带 `ChildControlResult(子任务控制结果)`. 项目禁止 compatibility export(兼容导出), 原 `ChildState(子任务状态)` 变体直接被替换, 调用者必须使用新结构.

## Technical Context(技术背景)

**Language/Version(语言和版本)**: Rust(编程语言) 2024, `rust-version = 1.88` (来自 `Cargo.toml`).
**Primary Dependencies(主要依赖)**: 复用 `tokio`, `tokio-util`, `tracing`, `metrics`, `serde`, `serde_json`. 本功能不新增 crate(库).
**Storage(存储)**: N/A(不适用). 本功能只更新进程内运行时状态, event(事件), metrics(指标) 和 audit(审计) 输出.
**Testing(测试)**: 功能与回归矩阵以 `tasks.md` T049 为准, 至少包含 `cargo test --test supervisor_child_runtime_state_control_test`, `--test supervisor_control_test`, `--test supervisor_real_shutdown_pipeline_test`, `--test supervisor_runtime_lifecycle_test`, `--test supervisor_shutdown_test`, `--test observability_smoke_test`, `--test dashboard_protocol_shape_test`, `--test supervisor_examples_test`, `--test control_test`, 以及 `cargo test --test naming_contract_test source_code_uses_approved_state_names`. 近似全量与完整验收见 `tasks.md` T050 与 `quickstart.md` 第 6 节.
**Target Platform(目标平台)**: Rust library(库) 和 Tokio runtime(Tokio 运行时), 面向本地和服务端 supervisor runtime(监督器运行时).
**Project Type(项目类型)**: Rust single crate(Rust 单包) supervisor runtime(监督器运行时).
**Performance Goals(性能目标)**: 单次 `PauseChild(暂停子任务)`, `RemoveChild(移除子任务)`, `QuarantineChild(隔离子任务)` 必须在 control loop(控制循环) 单跳内返回, 不得阻塞等待 child future(子任务 future) 终止. `CurrentState(当前状态)` 读取必须包含 heartbeat(心跳), readiness(就绪状态) 和 restart_limit(重启次数限制) 剩余次数, 代表性测试场景中连续 20 次构造每次都必须低于 1 毫秒.
**Constraints(约束)**: 不新增 crate(库), 不新增 compatibility exports(兼容导出), 不改变 `ControlCommand(控制命令)` 公开 enum 变体形状, 不改变 dashboard protocol(仪表盘协议) 的请求字段. `CommandResult(命令结果)` 的调用结果形状会把 `ChildState(子任务状态)` 替换为 `ChildControl(子任务控制)`, `CurrentState(当前状态)` 调用结果会新增 `child_runtime_records(子任务运行状态记录集合)`, 这些调用结果形状变化属于本功能的有意交付项, 必须通过 dashboard(仪表盘) 返回结果模型和协议形状回归测试覆盖. 本功能新增测试文件必须放在 `src/tests/` 目录的外部测试文件中. 对 `Cargo.toml` 已注册的既有外部测试目标 `src/control/tests/control_test.rs`, 本功能只允许更新既有断言, 不得新增生产模块内联测试. 已存在的 `tests/dashboard_protocol_shape_test.rs` 必须原地更新, 不得在 `src/tests/` 或 `Cargo.toml` 中新增同名测试目标. 控制命令路径禁止 `await` 真实 `child future(子任务 future)` 终止, 必须通过 `ChildAttemptMessage::Exited(子任务退出消息)` 回到 control loop(控制循环) 中再完成停止完成事件, 并通过 `reconcile_stop_deadlines(调和停止截止时间)` 在后续 control loop(控制循环) 轮次暴露停止失败, 以避免阻塞其他控制命令. 控制命令的 `stop_deadline_at_unix_nanos(停止截止时间)` 必须由取消送达时刻加当前 supervisor runtime(监督器运行时) 已生效的 `ShutdownPolicy.graceful_timeout(关闭策略优雅等待时间)` 得到, 本功能不新增单独的控制命令等待窗口配置, 且控制命令路径继续忽略 `abort_after_timeout(超时后强制中止)`. 本切片不新增 `SupervisorSpec.heartbeat_timeout(监督器声明心跳超时)` 字段, 心跳陈旧判断只使用 `src/runtime/child_runtime_state.rs` 的 `DEFAULT_HEARTBEAT_TIMEOUT_SECS = 5` 默认常量. `RestartLimitState(重启次数限制状态)` 的 `window(窗口)` 和 `limit(上限)` 来自既有 `RestartLimit(重启次数限制)` 配置来源, 优先级依次为 child strategy override(子任务策略覆盖), group strategy(分组策略), supervisor spec(监督器声明) 和配置层默认 `PolicyConfig.child_restart_limit / child_restart_window_ms(策略配置子任务重启上限与窗口)`. 当前 `PolicyEngine(策略引擎)` 和 `RestartPolicy(重启策略)` 不保存 `used / remaining(已使用与剩余)` 运行时历史, 这两个字段必须由 runtime(运行时) 侧重启次数限制跟踪结构维护.
**Scale/Scope(规模和范围)**: 覆盖当前 `SupervisorTree(监督树)` 中全部声明 child(子任务). 动态 `AddChild(添加子任务)` 流程的运行时接入和动态 manifest(清单) 数量统计仍由后续切片处理, 本功能不修改动态子任务数量字段.

## Constitution Check(宪章检查)

_GATE(关口): Phase 0(研究阶段) 前必须通过. Phase 1(设计阶段) 后必须重新检查._

- **Module Ownership(模块所有权)**: 通过. `src/runtime/child_runtime_state.rs` 新增, 拥有 `ChildRuntimeState(子任务运行状态记录)` 类型和构造逻辑, 取代 `src/runtime/shutdown_pipeline.rs` 中现有 `ActiveChildAttempt(活动子任务尝试)`. `src/control/outcome.rs` 新增, 拥有 `ChildAttemptStatus(子任务尝试状态)`, `ChildControlOperation(子任务控制操作)`, `ChildStopState(子任务停止状态)`, `ChildControlResult(子任务控制结果)`, `RestartLimitState(重启次数限制状态)`, `ChildLivenessState(子任务存活状态)`, `ChildRuntimeRecord(子任务运行状态记录)` 等公开结果类型. `src/control/command.rs` 升级 `CommandResult(命令结果)` 形状, 不引入 compatibility exports(兼容导出). `src/runtime/control_loop.rs` 内部把 `child_runtime_states: HashMap<ChildId, ChildRuntimeState>` 作为唯一运行状态事实. `src/child_runner/runner.rs` 在 `ChildRunHandle(子任务运行句柄)` 中暴露 heartbeat 与 readiness 的只读 receiver, 但 child runner(子任务运行器) 仍不直接拥有运行状态记录.
- **Supervision Contract(监督契约)**: 通过. 本计划明确 `PauseChild(暂停子任务)`, `RemoveChild(移除子任务)`, `QuarantineChild(隔离子任务)` 的生命周期效果, 失败行为和与自动重启之间的交互. 控制命令立即返回 `ChildControlResult(子任务控制结果)`, 但真实 child exit(子任务退出) 仍通过 `ChildAttemptMessage::Exited(子任务退出消息)` 回到 control loop(控制循环), 并由 exit handler(退出处理) 决定是否触发后续删除或重启动作. 关闭路径与 `004-2-real-shutdown-pipeline` 完全复用, 不另起一套关闭语义.
- **Test Gate(测试关口)**: 通过. `tasks.md` 必须先列出新增外部测试 `supervisor_child_runtime_state_control_test`, 再列出生产实现任务. 既有 `control_test(控制测试)` 外部测试目标可以更新旧断言, 但不得新增内联测试. 测试必须覆盖: 读取真实运行状态字段, 三类控制命令真实发送取消, 已停止任务的幂等返回, 重启次数限制耗尽时控制结果显示剩余次数, 跨 `attempt(尝试)` 不误送取消, 控制命令与自动重启竞态时子任务控制操作优先.
- **Observable Failures(可观察失败)**: 通过. 新增事件包含 `ChildControlCancelDelivered(子任务控制取消已送达)`, `ChildControlStopCompleted(子任务控制停止完成)`, `ChildControlStopFailed(子任务控制停止失败)`, `ChildControlOperationChanged(子任务控制操作变化)`, `ChildRuntimeStateRemoved(子任务运行状态记录已移除)` 和 `ChildHeartbeatStale(子任务运行状态记录心跳陈旧)`. metrics(指标) 新增 `supervisor_child_control_command_total{command, result}` counter(计数器), `supervisor_child_runtime_restart_limit_remaining{child_id}` gauge(仪表), 不带 `child_id(子任务标识)` 标签的 `supervisor_child_runtime_heartbeat_stale_total(子任务运行状态记录心跳陈旧总数)` counter(计数器), 以及 `supervisor_child_runtime_operation_transitions_total{from, to}` counter(计数器). audit log(审计日志) 必须记录 `command_id(命令标识)`, `child_id(子任务标识)`, `generation(代次)`, `attempt(尝试)`, `status(状态)`, 取消送达情况和最终结果分类.
- **Small Increment(小增量)**: 通过. 不新增 crate(库). 新增 module 只有 `src/runtime/child_runtime_state.rs` 和 `src/control/outcome.rs`. heartbeat 复用已存在的 `tokio::sync::watch(观察通道)`, readiness(就绪状态) 把现有 `watch::Receiver<bool>` 升级为 `watch::Receiver<ReadinessState>`, 不新增 channel(通道) 类型. `PolicyEngine(策略引擎)` 保持无状态, `RestartLimitState(重启次数限制状态)` 由 runtime(运行时) 侧最小重启次数限制跟踪结构维护, 不把运行时重启次数历史塞入 `RestartPolicy(重启策略)`.
- **Chinese Writing(中文写作)**: 通过. 本计划, research(研究结论), data-model(数据模型), contracts(契约) 和 quickstart(快速开始) 必须使用中文写作, 英文术语写成 `English(中文说明)`. 代码标识符, 文件路径, crate(库) 名和协议字段保持原样.

## Project Structure(项目结构)

### Documentation(文档, 本功能)

```text
specs/004-3-child-runtime-state-control/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── child-runtime-state-control.md
├── checklists/
│   └── requirements.md
└── tasks.md
```

### Source Code(源代码, 仓库根目录)

```text
src/
├── child_runner/
│   ├── attempt.rs
│   └── runner.rs
├── control/
│   ├── command.rs
│   ├── handle.rs
│   ├── outcome.rs
│   └── mod.rs
├── event/
│   └── payload.rs
├── observe/
│   ├── metrics.rs
│   └── pipeline.rs
├── registry/
│   ├── entry.rs
│   └── store.rs
├── runtime/
│   ├── control_loop.rs
│   ├── shutdown_pipeline.rs
│   ├── child_runtime_state.rs
│   └── mod.rs
├── task/
│   └── context.rs
└── tests/
    ├── observability_smoke_test.rs
    ├── supervisor_child_runtime_state_control_test.rs
    ├── supervisor_control_test.rs
    ├── supervisor_real_shutdown_pipeline_test.rs
    └── supervisor_runtime_lifecycle_test.rs

tests/
└── dashboard_protocol_shape_test.rs

manual/
└── zh/
    └── runtime-control.md
```

**Structure Decision(结构决定)**: 采用 Rust single crate(Rust 单包) 结构. `src/runtime/child_runtime_state.rs` 新增, 拥有 `ChildRuntimeState(子任务运行状态记录)` 类型, 它在 `004-2-real-shutdown-pipeline` 引入的 `ActiveChildAttempt(活动子任务尝试)` 基础上增加 heartbeat(心跳), readiness(就绪状态), `restart_limit(重启次数限制)` 剩余次数和运行状态记录 `operation(操作)` 状态字段. `src/runtime/shutdown_pipeline.rs` 改为复用 `ChildRuntimeState(子任务运行状态记录)`, 不再独立维护一份活动尝试结构. `src/control/outcome.rs` 新增, 拥有 `ChildAttemptStatus(子任务尝试状态)`, `ChildControlOperation(子任务控制操作)`, `ChildControlResult(子任务控制结果)`, `RestartLimitState(重启次数限制状态)`, `ChildLivenessState(子任务存活状态)` 和 `ChildRuntimeRecord(子任务运行状态记录)`, 与 `004-2-real-shutdown-pipeline` 中 `src/shutdown/report.rs` 的公开报告类型保持同级. `src/control/command.rs` 把 `CommandResult::ChildState(子任务状态命令结果)` 替换为 `CommandResult::ChildControl(子任务控制命令结果)`, 并直接删除旧变体. `src/readiness/signal.rs` 把 readiness(就绪状态) 观察值升级为 `ReadinessState(就绪状态)` 枚举. `src/child_runner/runner.rs` 在 `ChildRunHandle(子任务运行句柄)` 中暴露 heartbeat 与 readiness 的 receiver. `src/task/context.rs` 继续提供现有 heartbeat 能力, 并新增标记未就绪的 readiness(就绪状态) API(应用程序接口).

## Complexity Tracking(复杂度跟踪)

| Violation(违反项) | Why Needed(为什么需要) | Simpler Alternative Rejected Because(为什么拒绝更简单方案) |
| ----------------- | ---------------------- | ---------------------------------------------------------- |
| N/A(不适用)       | 当前计划没有违反宪章   | N/A(不适用)                                                |

## Phase 0(研究阶段) 输出

研究结论写入 `specs/004-3-child-runtime-state-control/research.md`. 主要决策:

- `ChildRuntimeState(子任务运行状态记录)` 归属 `src/runtime/child_runtime_state.rs`, 因为运行状态字段含 Tokio 句柄, 这些资源属于 runtime(运行时) 边界. registry(注册表) 保存 `ChildRuntime(子任务运行时记录)` 作为声明性事实.
- `RestartLimit(重启次数限制)` 不修改无状态 `PolicyEngine(策略引擎)`. `RestartLimitState(重启次数限制状态)` 的窗口和上限来自既有 `RestartLimit(重启次数限制)` 配置来源, 已使用次数和剩余次数由 runtime(运行时) 侧重启次数限制跟踪结构在 child exit(子任务退出) 处理期间刷新.
- 控制命令路径采用 "立即返回当前状态, 真实退出通过现有 ChildAttemptMessage(子任务消息) 路径" 模型, 不引入新阻塞等待.
- heartbeat 与 readiness 通过保存 `tokio::sync::watch::Receiver(观察接收端)` 暴露给 control loop(控制循环), 避免新增 channel(通道) 类型. readiness(就绪状态) 的 receiver(接收端) 使用 `ReadinessState(就绪状态)` 枚举, 不再使用 `bool(布尔值)` 初始 `false(否)`.
- 控制命令与自动重启之间的优先级由 `ChildControlOperation(子任务控制操作)` 字段决定, 与 `ManagedChildState(受管子任务状态)` 保持兼容. 该公开枚举属于 `src/control/outcome.rs`, runtime(运行时) 只维护字段值.

## Phase 1(设计阶段) 输出

设计产物:

- `specs/004-3-child-runtime-state-control/data-model.md`: 完整数据模型目录, 至少包含 `ChildRuntimeState(子任务运行状态记录)`, `ChildAttemptStatus(子任务尝试状态)`, `ChildControlOperation(子任务控制操作)`, `ChildStopState(子任务停止状态)`, `ReadinessState(就绪状态)`, `ChildControlFailurePhase(子任务控制失败阶段)`, `ChildControlResult(子任务控制结果)`, `ChildControlFailure(子任务控制失败原因)`, `RestartLimitState(重启次数限制状态)`, `ChildLivenessState(子任务存活状态)`, `ChildRuntimeRecord(子任务运行状态记录)` 等实体, 字段约束与状态图均以该文件为权威.
- `specs/004-3-child-runtime-state-control/contracts/child-runtime-state-control.md`: 描述控制命令的输入与输出契约, 事件契约, metrics(指标) 契约和 audit(审计) 契约.
- `specs/004-3-child-runtime-state-control/quickstart.md`: 提供 `cargo test`(构建工具测试) 和人工检查命令, 用于验证实现是否满足规格.

## Post-Design Constitution Check(设计后宪章检查)

- **Module Ownership(模块所有权)**: 通过. 数据模型把 runtime handles(运行时句柄) 留在 `src/runtime/child_runtime_state.rs`, 把公开结果类型和公开运行状态记录枚举留在 `src/control/outcome.rs`, 不让 control(控制) 模块反向依赖 runtime(运行时) 模块.
- **Supervision Contract(监督契约)**: 通过. 契约定义 `ChildControlResult(子任务控制结果)` 的全部必填字段和事件序列, 失败必须暴露失败阶段, 目标 `child id(子任务标识)`, `generation(代次)`, `attempt(尝试)`, 当前子任务尝试状态和原因.
- **Test Gate(测试关口)**: 通过. 任务生成必须先列 `supervisor_child_runtime_state_control_test` 的全部行为测试, 再列实现任务.
- **Observable Failures(可观察失败)**: 通过. 契约要求真正向活动尝试送达取消的控制命令产生 `ChildControlCancelDelivered(子任务控制取消已送达)` 事件, 操作变化产生 `ChildControlOperationChanged(子任务控制操作变化)` 事件, 运行状态记录物理删除产生 `ChildRuntimeStateRemoved(子任务运行状态记录已移除)` 事件, 幂等路径不得产生取消送达或操作变化事件. 停止失败时必须产生 `ChildControlStopFailed(子任务控制停止失败)` 事件, metrics(指标) 按 `result(结果分类)` 计数, 并按 `from / to(原操作与新操作)` 记录操作转换.
- **Small Increment(小增量)**: 通过. 不新增 crate(库), 不新增持久化, 不新增外部服务, heartbeat 与 readiness 复用已有 `tokio::sync::watch(观察通道)`, readiness(就绪状态) 仅替换观察值类型.
- **Chinese Writing(中文写作)**: 通过. 全部派生文档使用中文和 ASCII(基础英文字符集) 标点.
