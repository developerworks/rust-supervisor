# Implementation Plan(实现计划): 真实关闭流水线

**Branch(分支)**: `004-runtime-semantics` | **Date(日期)**: 2026-05-14 | **Spec(规格)**: `specs/004-2-real-shutdown-pipeline/spec.md`
**Input(输入)**: 功能规格来自 `specs/004-2-real-shutdown-pipeline/spec.md`

## Summary(摘要)

本功能把 `ShutdownTree(关闭监督树)` 从阶段推进改成真实的 runtime shutdown pipeline(运行时关闭流水线). 控制循环必须向所有运行中的 child task(子任务) 发送 `CancellationToken(取消令牌)`, 再按 `shutdown_order(关闭顺序)` 等待任务完成, 超时后对滞留任务执行 `abort(强制中止)`, 最后对 registry(注册表), runtime handles(运行时句柄), journal(日志), metrics(指标) 和 dashboard-visible diagnostics(仪表盘可见诊断) 做对账. `ShutdownCoordinator(关闭协调器)` 继续只负责阶段和幂等状态, `runtime(运行时)` 模块拥有任务句柄, 取消令牌和关闭执行.

## Technical Context(技术背景)

**Language/Version(语言和版本)**: Rust(编程语言) 2024, `rust-version = 1.88`
**Primary Dependencies(主要依赖)**: 复用 `tokio`, `tokio-util`, `metrics`, `tracing`, `serde`. 本功能不新增 crate(库).
**Storage(存储)**: N/A(不适用). 本功能不引入持久化存储, 只更新运行时状态, event(事件), metrics(指标) 和 audit(审计) 输出.
**Testing(测试)**: `cargo test`, 新增 `supervisor_real_shutdown_pipeline_test`, 并回归 `supervisor_control_test`, `supervisor_shutdown_test`, `observability_smoke_test`, `dashboard_protocol_shape_test`.
**Target Platform(目标平台)**: Rust library(库) 和 Tokio runtime(Tokio 运行时), 面向本地和服务端 supervisor runtime(监督器运行时).
**Project Type(项目类型)**: Rust single crate(Rust 单包) supervisor runtime(监督器运行时).
**Performance Goals(性能目标)**: 关闭耗时必须受 `ShutdownPolicy(关闭策略)` 中 `graceful_timeout(优雅超时)` 和 `abort_wait(强制中止等待)` 约束, 不允许无限等待.
**Constraints(约束)**: 不添加 compatibility exports(兼容导出). 不改变 `ControlCommand(控制命令)` 的外部调用语义. 关闭期间不得触发自动重启策略. 测试必须放在外部 `src/tests/` 或 `tests/` 目录.
**Scale/Scope(规模和范围)**: 覆盖当前 supervisor tree(监督树) 中全部声明 child(子任务) 和正在运行的 attempt(尝试). 动态 child manifest(子任务清单) 的完整运行接入不在本功能范围内.

## Constitution Check(宪章检查)

*GATE(关口): Phase 0(研究阶段) 前必须通过. Phase 1(设计阶段) 后必须重新检查.*

- **Module Ownership(模块所有权)**: 通过. `src/shutdown/` 保留阶段模型和策略, `src/runtime/shutdown_pipeline.rs` 拥有真实关闭流水线, `src/runtime/control_loop.rs` 只负责接收消息和调用流水线, `src/child_runner/runner.rs` 暴露可取消和可中止的运行句柄.
- **Supervision Contract(监督契约)**: 通过. 本计划明确停止, 取消, 等待, 超时, 强制中止, 对账和调用者可见结果. `ShutdownTree(关闭监督树)` 返回覆盖每个 child(子任务) 的摘要.
- **Test Gate(测试关口)**: 通过. `tasks.md` 必须先列关闭取消, 顺序等待, 超时中止和对账测试, 再列实现任务. 最终验证命令写入 `quickstart.md`.
- **Observable Failures(可观察失败)**: 通过. 关闭失败必须说明 child id(子任务标识), phase(阶段), attempt(尝试), generation(代际) 和 reason(原因). event(事件), metrics(指标) 和 audit(审计) 必须覆盖取消送达, 等待完成, 超时和强制中止.
- **Small Increment(小增量)**: 通过. 本功能不新增 crate(库). 新增 `shutdown_pipeline` 模块的理由是任务句柄所有权不能放入 `ShutdownCoordinator(关闭协调器)`.
- **Chinese Writing(中文写作)**: 通过. 本计划, 派生产物和最终汇报使用中文写作, 英文术语写成 `English(中文说明)`.

## Project Structure(项目结构)

### Documentation(文档,本功能)

```text
specs/004-2-real-shutdown-pipeline/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── shutdown-pipeline.md
└── tasks.md
```

### Source Code(源代码,仓库根目录)

```text
src/
├── child_runner/
│   └── runner.rs
├── control/
│   └── command.rs
├── event/
│   └── payload.rs
├── observe/
│   ├── metrics.rs
│   └── pipeline.rs
├── runtime/
│   ├── control_loop.rs
│   ├── lifecycle.rs
│   ├── message.rs
│   ├── mod.rs
│   └── shutdown_pipeline.rs
├── shutdown/
│   ├── coordinator.rs
│   └── stage.rs
├── task/
│   └── context.rs
└── tests/
    ├── observability_smoke_test.rs
    ├── supervisor_control_test.rs
    ├── supervisor_real_shutdown_pipeline_test.rs
    └── supervisor_shutdown_test.rs
```

**Structure Decision(结构决定)**: 采用 Rust single crate(Rust 单包) 结构. `src/runtime/shutdown_pipeline.rs` 承接真实关闭流水线, 因为它需要访问运行时 attempt(尝试) 句柄和控制循环状态. `src/shutdown/coordinator.rs` 不接收任务句柄, 继续保持纯阶段状态机.

## Complexity Tracking(复杂度跟踪)

| Violation(违反项) | Why Needed(为什么需要) | Simpler Alternative Rejected Because(为什么拒绝更简单方案) |
|-------------------|------------------------|-------------------------------------------------------------|
| N/A(不适用) | 当前计划没有违反宪章 | N/A(不适用) |

## Phase 0(研究阶段) 输出

研究结论写入 `specs/004-2-real-shutdown-pipeline/research.md`. 结论是复用 `tokio-util::sync::CancellationToken` 和 `tokio::task::AbortHandle`, 不新增依赖. 控制循环必须保存 active attempt(活动尝试) 的 token(令牌), abort handle(强制中止句柄) 和完成接收端. 当前 `ChildRunner::run_once` 内部的二级 `tokio::spawn` 会让外层任务中止后无法保证真实 child future(子任务 future) 被中止, 所以实现必须调整 runner(运行器) 的句柄边界.

## Phase 1(设计阶段) 输出

设计产物包括:

- `specs/004-2-real-shutdown-pipeline/data-model.md`
- `specs/004-2-real-shutdown-pipeline/contracts/shutdown-pipeline.md`
- `specs/004-2-real-shutdown-pipeline/quickstart.md`

## Post-Design Constitution Check(设计后宪章检查)

- **Module Ownership(模块所有权)**: 通过. 数据模型把 `ShutdownPipeline(关闭流水线)`, `RunningChildAttempt(运行中子任务尝试)`, `ChildShutdownOutcome(子任务关闭结果)` 和 `ShutdownReconcileReport(关闭对账报告)` 分配到运行时边界.
- **Supervision Contract(监督契约)**: 通过. 契约定义了 `ShutdownResult(关闭结果)` 的返回扩展和每个 child(子任务) 的最终结果.
- **Test Gate(测试关口)**: 通过. 任务生成必须先写 `supervisor_real_shutdown_pipeline_test` 的行为测试, 再实现生产代码.
- **Observable Failures(可观察失败)**: 通过. 契约要求 event(事件), metrics(指标) 和 audit(审计) 同步记录关闭阶段和 per-child(逐子任务) 结果.
- **Small Increment(小增量)**: 通过. 不新增依赖, 不新增外部服务, 不引入 compatibility layer(兼容层).
- **Chinese Writing(中文写作)**: 通过. 设计产物使用中文和 ASCII(基础英文字符集) 标点.
