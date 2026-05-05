# Implementation Plan(实现计划): 创建监督器核心

**Branch(分支)**: `001-create-supervisor-core` | **Date(日期)**: 2026-05-05 | **Spec(规格)**: [spec.md](./spec.md)
**Input(输入)**: 功能规格来自 `specs/001-create-supervisor-core/spec.md`

## Summary(摘要)

本计划实现一个基于 Tokio(异步运行时) 的轻量 supervisor(监督器) runtime governance layer(运行时治理层).核心能力包括声明式 child spec(子任务规格),supervisor tree(监督树),restart policy(重启策略),backoff(退避),meltdown(熔断),readiness(就绪),control plane(控制面),state plane(状态平面),event plane(事件平面),observability pipeline(可观测性管线),four-stage shutdown(四阶段关闭),centralized configuration(集中化配置),examples(示例程序),bilingual documentation(双语文档),SBOM(软件物料清单) 和 crates.io readiness(发布就绪).

设计采用项目自有 public API(公开接口),不包装第三方 supervisor crate(监督器库),不引入 actor framework(参与者框架),不提供 compatibility method(兼容方法).实现必须按 module dependency map(模块依赖图) 和 parallel workstream(并行工作流) 并行推进,并且由 lead agent(主代理) 监督 subagent(子代理) 输出,及时纠偏.

## Technical Context(技术背景)

**Language/Version(语言和版本)**: Rust(编程语言) 2024.
**Primary Dependencies(主要依赖)**: `rust-config-tree` v0.1.9,`tokio` 1.52.1,`tokio-util` 0.7.x,`tracing` 0.1.44,`tracing-subscriber` 0.3.23,`metrics` 0.24.x,`serde` 1.x,`serde_json` 1.x,`serde_yaml` 0.9,`thiserror` 2.x,`uuid` 1.x,`rand` 0.10.x.
**Storage(存储)**: N/A(不适用).第一版不引入持久化存储.运行时状态在进程内管理,diagnostic replay(诊断回放) 使用 fixed-capacity event journal(固定容量事件日志缓冲区).
**Testing(测试)**: `cargo fmt --check`,`cargo check`,`cargo test`,`cargo doc --no-deps`,quality gate scripts(质量门禁脚本),SBOM(软件物料清单) 校验,`cargo package --list`,`cargo publish --dry-run`.
**Target Platform(目标平台)**: 本地 Rust library(库),面向 Tokio(异步运行时) 应用,第一版只支持单进程运行.
**Project Type(项目类型)**: Rust library(库) 和 supervisor runtime(监督器运行时) crate(包).
**Performance Goals(性能目标)**: supervisor(监督器) 不进入 business hot path(业务热路径).高频业务消息不得经过 supervisor core(监督器核心).生命周期事件,控制命令,健康检查和状态查询必须保持低频治理边界.
**Constraints(约束)**: 禁止 compatibility exports(兼容导出),禁止旧接口别名,禁止 `pub use`(公开重导出),禁止 `super::` relative path(相对路径),禁止 inline unit test(内联单元测试),禁止 runtime tunable constant(运行时可调常量) 硬编码,禁止 `*Snapshot` 和 `*View` 代码命名,禁止 `state_view` 模块名,测试文件必须以 `_test.rs` 结尾.
**Scale/Scope(规模和范围)**: 第一版覆盖一个进程内的 supervisor tree(监督树),child(子任务),worker(工作任务),blocking worker(阻塞工作任务),runtime handle(运行时句柄),event subscriber(事件订阅者),observability pipeline(可观测性管线),examples(示例程序) 和 bilingual documentation(双语文档).distributed supervision(分布式监督),cross-process messaging(跨进程消息) 和 remote control(远程控制) 不在范围内.

## Constitution Check(宪章检查)

*GATE(关口): Phase 0(研究阶段) 前通过. Phase 1(设计阶段) 后再次通过.*

- **Module Ownership(模块所有权)**: PASS(通过).计划把 identity(身份),configuration(配置),specification(规格),task(任务),tree(树),policy(策略),health(健康),readiness(就绪),control(控制),runtime(运行时),registry(注册表),event(事件),state(状态),journal(事件日志缓冲区),summary(摘要),observe(可观测性),shutdown(关闭),error(错误) 和 test support(测试支持) 拆成独立模块.
- **Supervision Contract(监督契约)**: PASS(通过).规格和契约已经定义生命周期状态,启动,停止,重启,超时,取消,关闭,错误分类,策略决定和调用者可见结果.
- **Test Gate(测试关口)**: PASS(通过).tasks(任务) 阶段必须先列测试任务再列实现任务.本计划要求 integration test(集成测试) 位于 `src/tests/*_test.rs`,unit test(单元测试) 位于模块自己的 `tests/*_test.rs`.
- **Observable Failures(可观察失败)**: PASS(通过).所有失败路径必须产生 typed error(类型化错误),`SupervisorEvent`(监督器事件),structured log(结构化日志),tracing event(追踪事件),metrics(指标),audit event(审计事件) 或 `RunSummary`(运行摘要).
- **Small Increment(小增量)**: PASS(通过).每个 user story(用户故事) 都是独立可验收切片,并且 foundation(基础) 阶段只建立共享契约和测试支持.
- **Chinese Writing(中文写作)**: PASS(通过).计划,研究,数据模型,契约,quickstart(快速开始),任务和分析报告必须使用中文写作,英文术语必须写成 `English(中文说明)`.

## Project Structure(项目结构)

### Documentation(文档,本功能)

```text
specs/001-create-supervisor-core/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── glossary.md
├── quickstart.md
├── contracts/
│   └── public-api.md
├── checklists/
│   └── requirements.md
└── tasks.md
```

### Source Code(源代码,仓库根目录)

```text
src/
├── lib.rs
├── id/
│   ├── mod.rs
│   ├── types.rs
│   └── tests/*_test.rs
├── error/
│   ├── mod.rs
│   ├── types.rs
│   └── tests/*_test.rs
├── config/
│   ├── mod.rs
│   ├── loader.rs
│   ├── state.rs
│   ├── yaml.rs
│   └── tests/*_test.rs
├── spec/
│   ├── mod.rs
│   ├── child.rs
│   ├── supervisor.rs
│   └── tests/*_test.rs
├── task/
│   ├── mod.rs
│   ├── context.rs
│   ├── factory.rs
│   └── tests/*_test.rs
├── tree/
│   ├── mod.rs
│   ├── builder.rs
│   ├── order.rs
│   └── tests/*_test.rs
├── policy/
│   ├── mod.rs
│   ├── backoff.rs
│   ├── decision.rs
│   ├── meltdown.rs
│   └── tests/*_test.rs
├── readiness/
│   ├── mod.rs
│   ├── signal.rs
│   └── tests/*_test.rs
├── health/
│   ├── mod.rs
│   ├── heartbeat.rs
│   └── tests/*_test.rs
├── control/
│   ├── mod.rs
│   ├── command.rs
│   ├── handle.rs
│   └── tests/*_test.rs
├── registry/
│   ├── mod.rs
│   ├── entry.rs
│   ├── store.rs
│   └── tests/*_test.rs
├── runtime/
│   ├── mod.rs
│   ├── supervisor.rs
│   ├── control_loop.rs
│   └── tests/*_test.rs
├── child_runner/
│   ├── mod.rs
│   ├── attempt.rs
│   ├── runner.rs
│   └── tests/*_test.rs
├── event/
│   ├── mod.rs
│   ├── payload.rs
│   ├── time.rs
│   └── tests/*_test.rs
├── state/
│   ├── mod.rs
│   ├── child.rs
│   ├── supervisor.rs
│   └── tests/*_test.rs
├── journal/
│   ├── mod.rs
│   ├── ring.rs
│   └── tests/*_test.rs
├── summary/
│   ├── mod.rs
│   ├── builder.rs
│   └── tests/*_test.rs
├── observe/
│   ├── mod.rs
│   ├── pipeline.rs
│   ├── metrics.rs
│   ├── tracing.rs
│   └── tests/*_test.rs
├── shutdown/
│   ├── mod.rs
│   ├── coordinator.rs
│   ├── stage.rs
│   └── tests/*_test.rs
├── test_support/
│   ├── mod.rs
│   ├── assertions.rs
│   ├── factory.rs
│   └── tests/*_test.rs
└── tests/
    └── *_test.rs

examples/
├── config/supervisor.yaml
├── supervisor_quickstart.rs
├── config_tree_supervisor.rs
├── restart_policy_lab.rs
├── shutdown_tree.rs
└── observability_probe.rs

manual/
├── zh/
└── en/

docs/
├── zh/
└── en/

scripts/
├── check-coding-standard.sh
├── check-maintainability.sh
├── generate-sbom.sh
└── validate-sbom.sh
```

**Structure Decision(结构决定)**: 采用单 crate(包) Rust library(库) 和 top-level directory module(顶层目录模块) 结构.核心行为直接放在 `src/<module>/` 下,不得使用 `src/supervision/` 中间层,也不得使用 `src/<module>.rs` 平铺模块文件.测试文件统一使用 `_test.rs` 后缀.integration test(集成测试) 放在 `src/tests/*_test.rs`,unit test(单元测试) 放在模块自己的 `tests/*_test.rs`.`src/lib.rs` 只包含 crate doc(包文档) 和顶层 `pub mod <mod_name>;` 声明.每个 `src/<module>/mod.rs` 只能包含 `pub mod <mod_name>;` 声明,不得包含 `pub use`(公开重导出),类型定义,函数定义,常量定义或其它逻辑.

## Module Dependency Map(模块依赖图)

### Dependency Layers(依赖层)

```text
Layer 0(基础层):
  id,error

Layer 1(契约层):
  config,spec,task,event,state

Layer 2(策略和健康层):
  policy,readiness,health,shutdown

Layer 3(运行时所有权层):
  registry,tree,child_runner,runtime,control

Layer 4(观察和诊断层):
  observe,journal,summary

Layer 5(测试和学习层):
  test_support,src/tests,examples,manual,docs
```

### Allowed Dependencies(允许依赖)

| Owner Module(所有者模块) | May Depend On(可以依赖) | Reason(原因) |
|---|---|---|
| `id` | 外部标准库和轻量依赖 | 提供基础身份和值对象. |
| `error` | `id` | 错误需要定位受监督单元. |
| `config` | `id`,`error`,`spec` | rust-config-tree(集中配置树) 生成 `ConfigState`(配置状态) 和 `SupervisorSpec`(监督器规格). |
| `spec` | `id`,`error`,`policy`,`readiness`,`health`,`shutdown` | 声明式规格需要策略和运行边界类型. |
| `task` | `id`,`error`,`event` | `TaskContext`(任务上下文) 需要身份,错误和事件接收点. |
| `policy` | `id`,`error`,`task` | 策略读取 typed exit(类型化退出) 和失败类别. |
| `readiness` | `id`,`event` | 就绪信号需要定位 child(子任务) 并发事件. |
| `health` | `id`,`event`,`policy` | 健康过期后进入策略处理. |
| `shutdown` | `id`,`event`,`policy` | 四阶段关闭需要事件和升级策略. |
| `registry` | `id`,`spec`,`task`,`state` | 注册表拥有当前运行时索引和状态. |
| `tree` | `id`,`spec`,`registry` | 树编排需要定义顺序和路径. |
| `child_runner` | `task`,`event`,`policy`,`readiness`,`health`,`shutdown` | 子任务运行器执行生命周期和策略结果. |
| `runtime` | `config`,`tree`,`registry`,`child_runner`,`control`,`observe`,`shutdown` | 运行时只组合下层模块. |
| `control` | `id`,`event`,`state`,`registry`,`runtime` | 控制面暴露 handle(句柄) 和命令结果. |
| `event` | `id`,`error` | 事件只描述事实,不依赖运行时编排. |
| `state` | `id`,`error`,`policy` | 当前状态模型读取策略决定,不依赖事件总线. |
| `journal` | `event` | 事件日志缓冲区只保存事件. |
| `summary` | `event`,`state`,`journal`,`policy` | 运行摘要读取事件,状态和策略决定. |
| `observe` | `event`,`state`,`journal`,`summary` | 可观测性管线消费生命周期事实. |
| `test_support` | 所有公开契约模块 | 测试支持只用于测试和示例,生产模块不得依赖它. |

### Forbidden Dependencies(禁止依赖)

- Layer 0(基础层) 和 Layer 1(契约层) 不得反向依赖 runtime(运行时),control(控制),examples(示例) 或 docs(文档).
- `event`(事件) 不得依赖 `observe`(可观测性),避免事件模型和具体输出绑定.
- `state`(状态) 不得命名为 `state_view`(状态视图),也不得提供 `*View` 后缀类型.
- `runtime`(运行时) 不得访问其它模块内部状态,只能通过公开契约类型组合.
- `examples`(示例) 和 docs(文档) 不得反向影响核心模块 API(接口) 形状.
- 任意模块不得使用 `super::` relative path(相对路径).内部导入必须使用 `crate::` absolute path(绝对路径).

## Parallel Workstream Plan(并行工作流计划)

| Workstream(工作流) | Scope(范围) | Primary Files(主文件) | Independent Tests(独立测试) | Blockers To Remove(需要消除的卡点) |
|---|---|---|---|---|
| WS1 Contract Foundation(契约基础) | id,error,event,state | `src/id/`,`src/error/`,`src/event/`,`src/state/` | `src/tests/source_layout_test.rs`,`src/tests/module_boundary_test.rs`,`src/tests/import_rule_test.rs`,`src/tests/module_dependency_test.rs` | 稳定公开契约,避免后续反复改名. |
| WS2 Configuration(集中配置) | rust-config-tree(集中配置树),YAML(数据序列化格式),`ConfigState`(配置状态) | `src/config/`,`examples/config/supervisor.yaml` | `src/tests/config_boundary_test.rs`,`src/config/tests/yaml_config_test.rs`,`src/tests/supervisor_config_test.rs` | 禁止模块局部默认值和硬编码可调常量. |
| WS3 Declaration And Task(声明和任务) | `ChildSpec`,`SupervisorSpec`,`TaskFactory`,`TaskContext` | `src/spec/`,`src/task/` | `src/spec/tests/spec_test.rs`,`src/task/tests/task_test.rs`,`src/readiness/tests/readiness_test.rs`,`src/tests/supervisor_start_test.rs` | 先稳定 trait(特征) 和上下文,再接运行时. |
| WS4 Policy And Time(策略和时间) | policy,backoff,meltdown,deterministic time(确定性时间) | `src/policy/`,`src/test_support/` | `src/policy/tests/policy_test.rs`,`src/policy/tests/backoff_test.rs`,`src/policy/tests/meltdown_test.rs`,`src/tests/supervisor_policy_test.rs` | 退避和熔断值必须来自配置. |
| WS5 Runtime Tree(运行时树) | runtime,tree,registry,child runner | `src/runtime/`,`src/tree/`,`src/registry/`,`src/child_runner/` | `src/tree/tests/tree_test.rs`,`src/registry/tests/registry_test.rs`,`src/tests/supervisor_tree_test.rs` | 避免多个工作流同时写 `runtime/`,先定义接口再集成. |
| WS6 Control And Shutdown(控制和关闭) | handle,commands,health,shutdown,blocking worker(阻塞工作任务) | `src/control/`,`src/health/`,`src/shutdown/` | `src/control/tests/control_test.rs`,`src/health/tests/health_test.rs`,`src/shutdown/tests/shutdown_test.rs`,`src/tests/supervisor_control_test.rs`,`src/tests/supervisor_shutdown_test.rs` | 四阶段关闭和阻塞边界必须先写测试. |
| WS7 Observability Diagnostics(可观测性和诊断) | observe,journal,summary,metrics,audit | `src/observe/`,`src/journal/`,`src/summary/` | `src/observe/tests/observe_test.rs`,`src/journal/tests/journal_test.rs`,`src/summary/tests/summary_test.rs`,`src/tests/observability_smoke_test.rs` | observability(可观测性) 只能消费事实,不能控制生命周期. |
| WS8 Docs Examples Release(文档示例和发布) | examples,manual,docs,SBOM,release readiness(发布就绪) | `examples/`,`manual/`,`docs/`,`README.md`,`CHANGELOG.md`,`Cargo.toml`,`artifacts/sbom/` | `src/tests/supervisor_examples_test.rs`,`src/tests/supervisor_docs_sync_test.rs`,`src/tests/bilingual_docs_test.rs`,`src/tests/release_readiness_test.rs`,`src/tests/sbom_test.rs` | 文档和示例必须跟公开契约同步. |
| WS9 Quality Governance(质量治理) | coding standard(编码标准),cognitive complexity(认知复杂度),maintainability(可维护性),lead agent supervision(主代理监督) | `scripts/`,`artifacts/validation/`,`specs/001-create-supervisor-core/tasks.md` | `src/tests/coding_standard_test.rs`,`src/tests/complexity_test.rs`,`src/tests/maintainability_test.rs`,`src/tests/parallel_governance_test.rs` | 主代理必须持续审查子代理输出并纠偏. |

## Blocker Elimination Plan(卡点消除计划)

| Blocker(卡点) | Elimination Action(消除动作) | Evidence(证据) |
|---|---|---|
| shared file bottleneck(共享文件瓶颈) | 按 module ownership(模块所有权) 拆分 `src/state/`,`src/event/`,`src/runtime/` 和 `src/observe/` 的写入范围. | tasks(任务) 中每个 `[P]` 任务写不同主文件或不同目录边界. |
| unstable contract(不稳定契约) | 先完成 `id`,`error`,`event`,`state`,`spec`,`task` 契约测试. | foundational test(基础测试) 先于 runtime(运行时) 实现. |
| blocking dependency(阻塞依赖) | 让 WS2,WS3,WS4,WS7,WS8 在接口稳定后并行,把 runtime integration(运行时集成) 放到合并任务. | dependency graph(依赖图) 显示前置和可并行任务. |
| manual gate(人工门禁) | implementation phase(实现阶段) 使用 unattended implementation(无人值守实现) 和 task completion ledger(任务完成台账). | tasks(任务) 包含 ledger(台账),supervision record(监督记录) 和 completion check(完成检查). |
| long serial validation(长串行验证) | 把验证拆为 per-story cargo test(按故事测试),quality scripts(质量脚本) 和 final gate(最终关口). | quickstart(快速开始) 和 tasks(任务) 都列出分层验证命令. |
| unclear owner(负责人不清晰) | 每个 workstream(工作流) 指定 owner boundary(所有权边界),lead agent(主代理) 负责最终审查. | lead agent supervision record(主代理监督记录). |
| hidden coupling(隐藏耦合) | module dependency check(模块依赖检查) 拒绝 cycle dependency(循环依赖) 和跨模块内部访问. | `scripts/check-maintainability.sh` 和对应测试. |

## Lead Agent Supervision Plan(主代理监督计划)

- lead agent(主代理) 必须把每个 subagent workstream(子代理工作流) 绑定到明确文件集合,测试集合和验收证据.
- lead agent(主代理) 必须审查 subagent output(子代理输出),覆盖规格一致性,模块边界,文件边界,测试命名,文档同步,禁止兼容方法和验收证据.
- lead agent(主代理) 发现 development drift(开发偏差) 时,必须记录 correction record(纠偏记录),说明 drift type(偏差类型),affected files(受影响文件),expected requirement(期望要求),actual output(实际输出),correction action(纠偏动作),review result(复核结果) 和 final evidence(最终证据).
- workstream(工作流) 只有在 correction loop(纠偏循环) 闭环或 clean review record(清洁审查记录) 存在后,才能进入 completed task(已完成任务) 状态.
- implementation completion check(实现完成检查) 必须证明 task completion ledger(任务完成台账) 没有 pending task(待处理任务),in-progress task(进行中任务),失败检查或未记录完成证据.

## Phase 0 Research Output(研究阶段输出)

`research.md` 必须记录这些决定:

- 使用项目自有 supervisor model(监督器模型),不包装第三方 crate(库).
- 使用 `TaskFactory`(任务工厂) 创建 fresh future(新异步任务),不克隆任务实例.
- 使用 supervisor tree(监督树),Tokio(异步运行时) `JoinSet`(任务集合) 和 `CancellationToken`(取消令牌).
- 分离 current state(当前状态) 和 lifecycle event(生命周期事件).
- 使用 `SupervisorState`(监督器状态),`ChildState`(子任务状态),`state`(状态) 命名,禁止 `*Snapshot`,`*View`,`snapshot()` 和 `state_view`.
- 使用 `tracing`(结构化追踪),`metrics`(指标),typed error(类型化错误),deterministic test time(确定性测试时间),readiness(就绪),blocking task(阻塞任务),four-stage shutdown(四阶段关闭),event journal(事件日志缓冲区),`RunSummary`(运行摘要),low-cardinality metrics label(低基数指标标签).
- 固定 rust-config-tree(集中配置树) v0.1.9 和 YAML(数据序列化格式).
- 用 `glossary.md`(词汇表) 管理专业词汇和反引号词汇.
- 禁止 compatibility method(兼容方法).
- 使用 module dependency map(模块依赖图),parallel workstream(并行工作流),blocker elimination(卡点消除),lead agent supervision(主代理监督) 和 unattended implementation(无人值守实现) 治理实现阶段.

## Phase 1 Design Output(设计阶段输出)

- `data-model.md` 必须覆盖 configuration entities(配置实体),declarative entities(声明式实体),runtime entities(运行时实体),policy entities(策略实体),state machines(状态机),control plane(控制面),state plane(状态平面),event plane(事件平面),observability plane(可观测性平面),documentation entities(文档实体),quality governance entities(质量治理实体),parallel execution entities(并行执行实体) 和 release entities(发布实体).
- `contracts/public-api.md` 必须覆盖 module boundaries(模块边界),naming contract(命名契约),configuration contract(配置契约),task contract(任务契约),runtime control contract(运行时控制契约),state contract(状态契约),event contract(事件契约),shutdown contract(关闭契约),observability contract(可观测性契约),parallel governance contract(并行治理契约),documentation contract(文档契约),release contract(发布契约) 和 test support contract(测试支持契约).
- `quickstart.md` 必须覆盖 baseline commands(基线命令),YAML(数据序列化格式) 配置,最小用法,验收测试,可观测性冒烟检查,示例程序,文档发布关口和并行治理关口.
- `AGENTS.md` 已经指向 `specs/001-create-supervisor-core/plan.md`,不需要额外变更.

## Post-Design Constitution Check(设计后宪章检查)

- **Module Ownership(模块所有权)**: PASS(通过).模块依赖图和源码结构已经说明所有权边界.
- **Supervision Contract(监督契约)**: PASS(通过).数据模型和公开契约已经覆盖生命周期,失败,关闭和可见结果.
- **Test Gate(测试关口)**: PASS(通过).任务生成必须为每个行为变化先列测试任务.
- **Observable Failures(可观察失败)**: PASS(通过).事件,日志,追踪,指标,审计和运行摘要都有契约.
- **Small Increment(小增量)**: PASS(通过).用户故事和并行工作流可以独立验证.
- **Chinese Writing(中文写作)**: PASS(通过).本计划和设计产物使用中文正文和 `English(中文说明)` 术语格式.

## Complexity Tracking(复杂度跟踪)

| Violation(违反项) | Why Needed(为什么需要) | Simpler Alternative Rejected Because(为什么拒绝更简单方案) |
|-------------------|------------------------|-------------------------------------------------------------|
| N/A(不适用) | 当前计划没有宪章例外. | N/A(不适用). |
