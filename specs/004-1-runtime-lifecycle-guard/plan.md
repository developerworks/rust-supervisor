# Implementation Plan(实现计划): 运行时生命周期守卫

**Branch(分支)**: `004-runtime-semantics` | **Date(日期)**: 2026-05-14 | **Spec(规格)**: `specs/004-1-runtime-lifecycle-guard/spec.md`
**Input(输入)**: 功能规格来自 `/specs/004-1-runtime-lifecycle-guard/spec.md`

## Summary(摘要)

本功能修正 `Supervisor::start_with_policy` 的运行时控制面语义. 当前实现会启动 `run_control_loop`(运行时控制循环), 但是 `SupervisorHandle`(监督器控制句柄) 只保存命令 `channel(通道)` 和事件 `broadcast(广播)` 入口, 不保存控制循环的 `JoinHandle(任务句柄)`, 也不能在控制循环异常退出时主动暴露结构化状态. 本计划新增 `RuntimeControlPlane(运行时控制面)` 生命周期模型和 `RuntimeWatchdog(运行时看门狗)`, 让 `SupervisorHandle`(监督器控制句柄) 可以提供 `is_alive`, `health`, `join` 和 `shutdown` 语义. 运行时控制循环退出时, watchdog(看门狗) 必须把最终结果写入健康状态, 并通过 typed event(类型化事件), metrics(指标) 和 audit log(审计日志) 暴露诊断.

## Technical Context(技术背景)

**Language/Version(语言和版本)**: 当前仓库使用 Rust(编程语言) 2024 和 `rust-version`(编译器版本) 1.88.
**Primary Dependencies(主要依赖)**: 复用已有 `tokio`, `tokio-util`, `serde`, `serde_json`, `thiserror`, `tracing`, `metrics`, `uuid` 和仓库内 `observe(可观察性)` 模块. 本功能不新增 crate(库).
**Storage(存储)**: N/A(不适用). 控制面健康状态和最终退出结果保存在进程内内存结构中.
**Testing(测试)**: `cargo test --test supervisor_runtime_lifecycle_test`, `cargo test --test supervisor_control_test`, `cargo test --test observability_smoke_test`, 最终回归使用 `cargo test`.
**Target Platform(目标平台)**: Linux(操作系统) 和 macOS(操作系统) 上运行的 Rust(编程语言) library(库) 和 supervisor runtime(监督器运行时).
**Project Type(项目类型)**: Rust(编程语言) single crate(单包) library(库), 核心行为位于 `src/runtime/`, 句柄入口位于 `src/control/`.
**Performance Goals(性能目标)**: 正常启动后, 健康查询必须立即返回 alive(存活). 控制循环异常退出后, 下一次控制命令发送前必须可以读取 not alive(非存活) 状态. 对同一个已结束运行时重复调用 `join` 10 次, 每次都必须在 1 秒内返回相同最终结果.
**Constraints(约束)**: 不新增 compatibility exports(兼容导出). 不自动重启控制循环. 不在本规格中实现真实 child task(子任务) 关闭和代际隔离, 这些能力由后续规格处理. 不把运行时控制面生命周期逻辑放入 `src/main.rs`. 所有新增测试必须放在外部测试文件中, 不得写入生产模块的内联测试.
**Scale/Scope(规模和范围)**: 本规格只覆盖一个 `SupervisorHandle`(监督器控制句柄) 对应的一个运行时控制面. 同一进程可启动多个 Supervisor(监督器), 每个实例必须拥有独立的生命周期状态和最终退出结果.

## Constitution Check(宪章检查)

*GATE(关口): Phase 0(研究阶段) 前通过. Phase 1(设计阶段) 后重新检查.*

- **Module Ownership(模块所有权)**: `src/runtime/lifecycle.rs` 拥有控制面健康状态, 退出结果和幂等等待模型. `src/runtime/watchdog.rs` 拥有 `JoinHandle(任务句柄)` 观察和退出诊断发布. `src/runtime/control_loop.rs` 只执行运行时命令并返回明确退出结果. `src/control/handle.rs` 只暴露调用者可见方法, 不直接解释控制循环内部状态. `src/lib.rs` 和 `src/runtime/mod.rs` 只做模块注册, 不添加 compatibility exports(兼容导出).
- **Supervision Contract(监督契约)**: 本功能改变控制面启动, 健康查询, 异常退出观测, shutdown(关闭) 和 join(等待结束) 语义. 控制面状态包括 starting(启动中), alive(存活), shutting down(正在关闭), completed(已完成) 和 failed(失败). 控制循环正常关闭返回 completed(已完成), 异常退出或 panic(恐慌) 返回 failed(失败). 调用者可见错误必须包含阶段, 原因和是否可恢复.
- **Test Gate(测试关口)**: 任务必须先写 `src/tests/supervisor_runtime_lifecycle_test.rs` 中的外部测试, 再写生产实现. 测试覆盖启动后健康状态, 启动事件, 控制循环异常退出诊断, 结束后健康状态, shutdown(关闭) 后 join(等待结束), 重复 join(等待结束) 幂等和结束后命令错误.
- **Observable Failures(可观察失败)**: 控制循环异常退出必须产生 `RuntimeControlLoopFailed` 或等价 typed event(类型化事件), `supervisor_runtime_control_loop_exit_total` metrics(指标), audit log(审计日志) 记录和 `RuntimeHealthReport(运行时健康报告)`. 错误必须指出 runtime control loop(运行时控制循环), 阶段和原因.
- **Small Increment(小增量)**: 本功能不新增依赖, 不引入持久化层, 不实现自动重启控制循环, 不扩展 relay(中继) 或 dashboard client(看板客户端). 新增后台单元只有 watchdog(看门狗), 其职责是把 `JoinHandle(任务句柄)` 的单次退出结果转换为可重复读取的健康状态.
- **Chinese Writing(中文写作)**: 本计划, research(研究结论), data model(数据模型), contracts(契约), quickstart(快速开始) 和 tasks(任务) 使用中文写作. 英文术语写成 `English(中文说明)`, 文件路径, crate(库) 名称, 命令和协议字段保持原样.

**Post-Design Check(设计后检查)**: Phase 1(设计阶段) 产物已经把模块所有权, 监督契约, 测试关口, 可观察失败, 小增量和中文写作要求映射到 `research.md`, `data-model.md`, `contracts/runtime-control-plane.md` 和 `quickstart.md`. 未发现需要 Complexity Tracking(复杂度跟踪) 的宪章违反项.

## Project Structure(项目结构)

### Documentation(文档, 本功能)

```text
specs/004-1-runtime-lifecycle-guard/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── runtime-control-plane.md
├── checklists/
│   └── requirements.md
└── tasks.md
```

### Source Code(源代码, 仓库根目录)

```text
src/
├── control/
│   └── handle.rs
├── event/
│   └── payload.rs
├── observe/
│   ├── metrics.rs
│   └── pipeline.rs
├── runtime/
│   ├── mod.rs
│   ├── control_loop.rs
│   ├── lifecycle.rs
│   ├── supervisor.rs
│   └── watchdog.rs
└── tests/
    ├── supervisor_runtime_lifecycle_test.rs
    ├── supervisor_control_test.rs
    └── observability_smoke_test.rs

manual/
└── zh/
    └── runtime-control.md
```

**Structure Decision(结构决定)**: 本功能使用现有 Rust(编程语言) single crate(单包) 布局. `runtime(运行时)` 模块拥有生命周期状态, watchdog(看门狗) 和控制循环退出结果. `control(控制)` 模块只把这些能力挂到 `SupervisorHandle`(监督器控制句柄). `event(事件)` 和 `observe(可观察性)` 模块扩展 typed event(类型化事件), metrics(指标) 和 audit log(审计日志) 映射. `manual/zh/runtime-control.md` 同步调用者可见语义.

## Complexity Tracking(复杂度跟踪)

无宪章违反项. watchdog(看门狗) 是实现 FR-002 和幂等 `join` 的最小后台单元. 不新增依赖, 不引入持久化层, 不增加兼容层.
