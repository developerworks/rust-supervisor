# Implementation Plan(实现计划): 创建监督器核心

**Branch(分支)**: `001-create-supervisor-core` | **Date(日期)**: 2026-05-04 | **Spec(规格)**: [spec.md](spec.md)
**Input(输入)**: 功能规格来自 `/specs/001-create-supervisor-core/spec.md`

## Summary(摘要)

本功能会构建一个 Rust(编程语言) 2024 单 crate(包) 的 supervisor core(监督器核心)。它把生命周期治理作为产品表面，覆盖声明式 child spec(子任务规格)、supervisor tree(监督树)、restart policy(重启策略)、meltdown policy(熔断策略)、health check(健康检查)、two-phase shutdown(两阶段关闭)、state snapshot(状态快照)、event stream(事件流)、tracing(结构化追踪)、metrics(指标)、audit command event(审计命令事件) 和 deterministic test time(确定性测试时间)。

实现只吸收第三方 crate(库) 的成熟概念，不复制它们的公开 API(接口)。本 crate(包) 会暴露项目自己的 task(任务)、tree(树)、policy(策略)、event(事件)、snapshot(快照)、control(控制) 和 error(错误) 模型，并且不提供 compatibility exports(兼容导出)。

## Technical Context(技术背景)

**Language/Version(语言和版本)**: Rust(编程语言) 2024
**Primary Dependencies(主要依赖)**: `tokio` 1.52.1、`tokio-util` 0.7.18、`tracing` 0.1.44、`metrics` 0.24.5、`thiserror` 2.0.18、`serde` 1.0.228、`serde_json` 1.0.149、`uuid` 1.23.1、`rand` 0.10.1，以及现有 `rust-config-tree` 0.1.7。
**Storage(存储)**: 内存中的 runtime registry(运行时注册表)、event bus(事件总线)、latest snapshot store(最新快照存储) 和 audit event stream(审计事件流)。持久状态由任务代码显式外置，supervisor core(监督器核心) 不拥有它。
**Testing(测试)**: `cargo test`、用于事件顺序调试的 `cargo test -- --nocapture`、`cargo fmt --check` 和 `cargo check`。
**Target Platform(目标平台)**: 运行在 Tokio(异步运行时) 上的单进程 Rust(编程语言) 应用。
**Project Type(项目类型)**: Rust library(库)，并保留轻量 example/CLI(示例或命令行) 入口。
**Performance Goals(性能目标)**: supervisor(监督器) 控制和生命周期事件必须离开业务 hot path(热路径)。生命周期操作属于低频操作。事件发布和快照更新对每个受影响 child(子任务) 为 O(1)，组重启对受监督范围为 O(n)。
**Constraints(约束)**: 不使用 actor framework(参与者框架)，不复制第三方 API(接口)，不提供 compatibility exports(兼容导出)，root shutdown(根关闭) 后不得留下 orphan task(孤儿任务)，backoff(退避)、timeout(超时)、heartbeat(心跳) 和 meltdown(熔断) 行为必须使用 paused time(暂停时间) 测试。
**Scale/Scope(规模和范围)**: 第一版支持一个进程、一个 Tokio(异步运行时)、一棵 root tree(根树)，并允许嵌套 supervisor(监督器) 和 worker(工作任务)。distributed supervision(分布式监督)、remote control(远程控制) 和 cross-process messaging(跨进程消息) 不在范围内。

## Constitution Check(宪章检查)

*GATE(关口): Phase 0(研究阶段) 前必须通过。Phase 1(设计阶段) 后必须重新检查。*

- **Module Ownership(模块所有权)**: PASS(通过)。源码布局会创建 `src/supervision/` 边界，并拆分 spec(规格)、id(标识)、task context(任务上下文)、runtime binding(运行时绑定)、child runner(子任务运行器)、tree(树)、policy(策略)、health(健康)、control(控制)、registry(注册表)、event(事件)、snapshot(快照)、observe(观察)、shutdown(关闭)、error(错误) 和 test support(测试支持)。`src/main.rs` 保持轻量演示入口。
- **Supervision Contract(监督契约)**: PASS(通过)。计划写明生命周期状态、启动、停止、重启、超时处理、取消传播、关闭协议、类型化失败类别和调用者可见控制结果。
- **Test Gate(测试关口)**: PASS(通过)。设计阶段定义 panic restart(恐慌重启)、quarantine(隔离)、meltdown(熔断)、no-orphan shutdown(无孤儿任务关闭)、监督策略、事件形状和 paused time(暂停时间) 行为的验收测试。
- **Observable Failures(可观察失败)**: PASS(通过)。失败路径会产生 typed error(类型化错误)、policy decision(策略决定)、`SupervisorEvent`(监督器事件)、最新快照、tracing span/event(追踪范围和事件)、metrics(指标) 和 command audit event(命令审计事件)。
- **Small Increment(小增量)**: PASS(通过)。依赖只覆盖运行时原语、取消、可观察性、指标、序列化、类型化错误、标识和 jitter(抖动)。计划拒绝 actor framework(参与者框架)、生产依赖 `supertrees` 和 compatibility layer(兼容层)。
- **Chinese Writing(中文写作)**: PASS(通过)。本计划和当前功能文档使用中文写作，英文术语使用 `English(中文说明)`。

## Project Structure(项目结构)

### Documentation(文档，本功能)

```text
specs/001-create-supervisor-core/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── public-api.md
└── tasks.md
```

### Source Code(源代码，仓库根目录)

```text
src/
├── main.rs
├── lib.rs
└── supervision/
    ├── mod.rs
    ├── backoff/
    ├── spec.rs                          # ChildSpec(子任务规格) 和 SupervisorSpec(监督器规格)
    ├── id.rs                            # ChildId(子任务标识)、SupervisorPath(监督器路径)、Generation(代次)、Attempt(尝试次数)
    ├── task.rs                          # TaskFactory(任务工厂)、TaskCtx(任务上下文)、TaskResult(任务结果)
    ├── runtime.rs                       # Tokio(异步运行时) JoinSet(任务集合) 绑定
    ├── child_runner.rs                  # 单个 child(子任务) 生命周期循环
    ├── tree.rs                          # supervisor tree(监督树) 和重启范围
    ├── policy.rs                        # 重启、退避和熔断决策
    ├── health.rs                        # heartbeat(心跳) 和 stale(过期) 检测
    ├── control.rs                       # SupervisorHandle(监督器句柄) 和 ControlCommand(控制命令)
    ├── registry.rs                      # ChildRuntime(子任务运行态) 注册表
    ├── event.rs                         # When(何时)、Where(何处)、What(发生内容) 事件模型
    ├── snapshot.rs                      # 最新状态模型
    ├── observe.rs                       # tracing(结构化追踪)、metrics(指标) 和 subscriber(订阅者)
    ├── shutdown.rs                      # two-phase shutdown(两阶段关闭)
    ├── error.rs                         # typed error(类型化错误)
    └── test_support.rs                  # paused time(暂停时间) 和断言工具

tests/
├── supervisor_lifecycle.rs
├── supervisor_policy.rs
├── supervisor_shutdown.rs
├── supervisor_tree.rs
├── supervisor_observe.rs
└── supervisor_control.rs
```

**Structure Decision(结构决定)**: 本功能使用单个 Rust crate(包)，并创建 `src/supervision/` 作为功能边界。公开导出只通过 `src/lib.rs` 和 `src/supervision/mod.rs` 暴露项目自有类型；项目不得为参考 crate(库) 增加 compatibility re-export(兼容再导出) 模块。

## Complexity Tracking(复杂度跟踪)

没有宪章违反项。模块数量由必须独立维护的约束决定：policy(策略)、runtime(运行时)、event(事件)、state(状态)、health(健康)、shutdown(关闭)、control(控制) 和 test-time behavior(测试时间行为) 各自拥有不同不变量。

## Phase 0(阶段零): Research Output(研究输出)

研究结论记录在 [research.md](research.md)。所有技术背景未知项已经解决。第三方参考只作为概念输入。

## Phase 1(阶段一): Design Output(设计输出)

- Data model(数据模型): [data-model.md](data-model.md)
- Public interface contract(公开接口契约): [contracts/public-api.md](contracts/public-api.md)
- Quickstart and validation path(快速开始和验证路径): [quickstart.md](quickstart.md)

## Post-Design Constitution Check(设计后宪章检查)

- **Module Ownership(模块所有权)**: PASS(通过)。`data-model.md` 和 `contracts/public-api.md` 保留模块拆分，并避免 compatibility exports(兼容导出)。
- **Supervision Contract(监督契约)**: PASS(通过)。实体状态、迁移、控制命令、事件和关闭规则都明确且可测试。
- **Test Gate(测试关口)**: PASS(通过)。`quickstart.md` 列出实现被接受前必须存在的 Cargo(构建工具) 检查和集成测试。
- **Observable Failures(可观察失败)**: PASS(通过)。事件、快照、tracing(结构化追踪)、metrics(指标)、audit(审计) 和 error(错误) 契约都携带任务路径、阶段、原因和策略决定。
- **Small Increment(小增量)**: PASS(通过)。第一版实现保持单进程和内存模型，持久化和分布式控制不在范围内。
- **Chinese Writing(中文写作)**: PASS(通过)。设计文档使用中文写作，并以 `English(中文说明)` 表达英文术语。
