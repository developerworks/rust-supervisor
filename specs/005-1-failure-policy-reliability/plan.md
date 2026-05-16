# Implementation Plan(实现计划): `005-1` Failure Policy Pipeline(失败策略流水线) and Production Backoff(生产级退避)

**Branch(分支)**: `004-runtime-semantics` | **Date(日期)**: 2026-05-16 | **Primary Spec(主规格)**: `specs/005-1-failure-policy-reliability/spec.md`
**Companion Spec(伴随规格)**: `specs/005-2-work-role-defaults/spec.md` (**Role defaults**(角色默认), **`evaluate budget`(评估预算)** 一致条款)

## Summary(摘要)

本切片要求把 **`restart_execution_plan`(重启执行计划)** 里的 **`restart limit`(重启次数限制)** 与 **`escalation policy`(升级策略)** 读入统一的 **`policy pipeline`(策略流水线)**, 并把 **`MeltdownTracker`(熔断跟踪器)** 扩展为 **`child`(子任务)**, **`group`(分组)**, **`supervisor`(监督器)** 三套互不混算的 **`scope`(作用域)**, 并把 **`BackoffPolicy`(退避策略)** 升级到支持 **`full jitter`(全抖动)**, **`decorrelated jitter`(去相关抖动)**, **并发重启闸门**, **`cold start budget`(冷启动预算)**, **`hot loop detection`(热循环检测)**, 全部结论写入可对账的 **`TypedSupervisionEvent`(类型化监督事件)** 或等价导出通道. **Phase 0(研究阶段)** 结论见 `research.md`, **Phase 1(设计阶段)** 见 `data-model.md`, `contracts/pipeline-and-events.md`, `quickstart.md`.

## Technical Context(技术背景)

**Language/Version(语言和版本)**: Rust(编程语言) 2024, **rust-version** `1.88` (见根目录 `Cargo.toml`)
**Primary Dependencies(主要依赖)**: **Tokio**(异步运行时), **`rand`(随机库)**, **`tracing`(结构化追踪)**, **`serde`(序列化)**, **`metrics`(指标)**, **`confique`(配置)**; **本切片默认不新增 crate**, **除非 Complexity Tracking(复杂度跟踪)** 登记审计理由.
**Storage(存储)**: N/A(不适用), **熔断计数与闸门状态驻留在监督运行时内存**.
**Testing(测试)**: `cargo test`, **验收夹具必须能钉死 RNG seed 与注入时钟** (依赖 **`tokio` dev `test-util`**).
**Target Platform(目标平台)**: **`rust_supervisor` Library**(库), **examples**(示例), **`Linux`(操作系统)** 与 **`macOS`(操作系统)** 上的开发者工作站常见组合.
**Project Type(项目类型)**: **`Tokio`(异步运行时)** **`supervisor runtime`(监督器运行时)**.
**Performance Goals(性能目标)**: **本切片不把吞吐类数值指标写入合同**, **阈值默认值以满足 **`spec.md`** Assumptions(假设) 里秒级时钟可稳定触发为前提**, **具体数字目标留给后续产品化切片**.
**Constraints(约束)**: **禁止 compatibility exports(兼容导出)**, **`src/` Rust 注释英文**, **规格正文中文且术语 **`English(中文说明)`**.
**Scale/Scope(规模和范围)**: **单进程内一棵或多棵监督树**, **闸门计数不隐含跨主机集群全局桶**.

## Constitution Check(宪章检查)

**GATE(关口)**: **Phase 0 前初次自检** ; **Phase 1 产出冻结前复检**.

### Phase 0 gate (初次)

- **Module Ownership(模块所有权)**: **`policy pipeline`(策略流水线)** 编排落在 **`src/runtime/control_loop.rs`** 或其抽取后的同名职责模块; **`MeltdownTracker`(熔断跟踪器)** 落在 **`src/policy/meltdown.rs`** ; **`restart_execution_plan`(重启执行计划)** 落在 **`src/tree/order.rs`** ; **`TypedSupervisionEvent`(类型化监督事件)** 增量落在 **`src/event/payload.rs`** 并由 **`src/observe/`** 管道转发; **入口文件不堆积编排分支**.
- **Supervision Contract(监督契约)**: **本切片改变进程结束之后的自动补救动作以及闸门给出的上限** ; **`success`(成功)** 路径仍须在每一阶段留下可对账观测点; **`manual_stop`(人工停止)** 与 **`external_cancel`(外部取消)** 优先于自动重启; **`shutdown`(关闭)** 语义保持与 **`spec.md`** `Constitution Alignment(宪章一致)` 小节所列边界一致.
- **Test Gate(测试关口)**: **`tasks.md`** 尚未生成; **暂以 **`cargo test`** 默认全量为合并闸门**; **新增测试文件名在 **`/speckit-tasks`** 阶段逐项列出**.
- **Observable Failures(可观察失败)**: **以结构化事件载荷为准**; **`broadcast`** 字符串通道仅作过渡期诊断.
- **Small Increment(小增量)**: **不默认引入新异步运行时或持久化层**.
- **Chinese Writing(中文写作)**: **本文件与派生物使用中文叙述**, **英文术语括注**.

### Phase 1 post-design gate (复检)

- **`research.md`** 已无 NEEDS CLARIFICATION(需要澄清) 条目.
- **`contracts/pipeline-and-events.md`** 已冻结六阶段顺序与 **`protection restrictiveness ladder`(保护从严档位序)** .
- **`data-model.md`** 已写明 **`scopes_triggered`(已触发作用域列表)** 与 **`lead_scope`(主导归因作用域)** 字段义务.
- **`quickstart.md`** 已给出 **`src/`** 阅读顺序锚点.

## Phase 0 Outputs (研究阶段产出)

- **Frozen file(冻结文件)**: `research.md`

## Phase 1 Outputs (设计阶段产出)

- **Entities and fields(实体与字段)**: `data-model.md`
- **External-stable contracts(对外稳定契约)**: `contracts/pipeline-and-events.md`
- **Contributor orientation(贡献者导读)**: `quickstart.md`

## Project Structure(项目结构)

### Documentation(文档, 本功能)

```text
specs/005-1-failure-policy-reliability/
├── plan.md
├── spec.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── pipeline-and-events.md
└── tasks.md                          # /speckit-tasks 生成
specs/005-2-work-role-defaults/
└── spec.md                           # 角色默认, 验收一致 evaluate budget
```

### Source Code(源代码)

```text
src/
├── runtime/
│   └── control_loop.rs               # pipeline orchestration anchor
├── tree/
│   └── order.rs                      # StrategyExecutionPlan / restart_execution_plan
├── policy/
│   ├── meltdown.rs                   # MeltdownTracker extension surface
│   └── decision.rs                   # PolicyEngine boundary
├── event/
│   └── payload.rs                    # TypedSupervisionEvent payloads
├── observe/
│   └── pipeline.rs                   # forwarding typed diagnostics
└── tests/
    └── supervisor_*                 # behavior regressions (exact names in tasks.md)
```

**Structure Decision(结构决定)**: 沿用现有 `rust_supervisor` crate 单包布局, 只在 `runtime`, `policy`, `event`, `observe` 边界内增量扩展, 不新建 `backend/` 或 `frontend/` 分叉目录.

## Verification Scope (验证范围)

**Primary command(主命令)**: `cargo test`

**Must-cover behaviors(必须覆盖的行为)** (until superseded by `tasks.md`):

1. 六阶段 **`policy pipeline`(策略流水线)** 顺序在外部可观测.
2. **`restart_execution_plan`** 中的 **`restart_limit`** 与 **`escalation_policy`** 进入 **`evaluate budget`(评估预算)** 结论, **不得静默丢弃**.
3. **`MeltdownTracker`(熔断跟踪器)** 须对 **`group`(分组)** 这一 **`scope`(作用域)** 单独计数; **多层 **`local verdict`(局部判定)** 汇总得到的 **`effective meltdown verdict`(有效熔断判定)** 须在 **`protection restrictiveness ladder`(保护从严档位序)** 上取最严一档**.
4. **`full jitter`(全抖动)**, **`decorrelated jitter`(去相关抖动)**, **并发闸门**, **`cold start budget`(冷启动预算)**, **`hot loop detection`(热循环检测)** 在 **`BackoffPolicy`(退避策略)** 路径上可被 **`RNG seed`(随机种子)** 与注入时钟验收.

## Complexity Tracking(复杂度跟踪)

> **本节仅在明知偏离宪章时填写**, **当前无登记项**.

## Extension Hooks (扩展钩子)

**Optional Pre-Hook**(可选前置钩子): **extension** `git`, **command** `speckit.git.commit`, **description** Auto-commit before implementation planning, **Prompt**: Commit outstanding changes before planning? **To execute**: `/speckit.git.commit`

**Optional Post-Hook**(可选后置钩子): **extension** `git`, **command** `speckit.git.commit`, **description** Auto-commit after implementation planning, **Prompt**: Commit plan changes? **To execute**: `/speckit.git.commit`