# Implementation Plan(实现计划): 生产级重启策略与分组隔离观测

**Branch(分支)**: `006-4-restart-policy-production` | **Date(日期)**: 2026-05-18 | **Spec(规格)**: `specs/006-4-restart-policy-production/spec.md`
**Input(输入)**: 功能规格来自 `specs/006-4-restart-policy-production/spec.md`

**Note(说明)**: 本文件由 `/speckit-plan` 命令生成, 基于 `.specify/templates/plan-template.md` 模板.

## Summary(摘要)

本切片在 005-1(failure-policy-reliability) 和 005-2(work-role-defaults) 已建立的策略入口基础上, 补齐生产级重启策略的三大能力: (1) restart budget(重启预算) 和 fairness probe(公平性探针) 接入统一评估管线, 确保快速失败不会压出无限重启风暴; (2) group strategy(分组策略) 隔离验证, 确认分组熔断不误伤无关任务; (3) critical(关键) 与 optional(可选) 子任务的分叉路径在 typed event(类型化事件) 和 metrics(指标) 双通道完全可观测.

现有 `src/policy/` 模块已提供 BackoffPolicy(退避策略), MeltdownPolicy/MeltdownTracker(熔断策略/跟踪器), FailureWindow(失败窗口), PolicyEngine(策略引擎), WorkRole(工作角色) 基础实现.本切片的增量在于统一评估管线(按 `budget → meltdown → backoff` 顺序: 预算不足直接拒绝不经过熔断, 熔断后不计算退避), 预算快照, 公平性探测, 分组隔离断言, 以及关键/可选分叉的事件与指标通道.

**依赖**: 强依赖 `specs/005-1-failure-policy-reliability/`, `specs/005-2-work-role-defaults/`, `specs/006-3-lifecycle-shutdown-realism/`(ChildSlot 基础设施).

## Technical Context(技术背景)

**Language/Version(语言和版本)**: Rust(编程语言) 2024, rust-version 1.88
**Primary Dependencies(主要依赖)**: Tokio(异步运行时) 已提供 time, sync 原语.不新增外部 crate.
**Storage(存储)**: N/A(不适用).预算快照和熔断计数器驻留在运行时内存中.
**Testing(测试)**: `cargo test`; 快速失败波形用固定 RNG seed(随机种子) 仿真; 分组隔离用双分组对照实验.并发正确性由 loom(并发模型测试) 夜间 CI 独立覆盖, 不纳入本切片 tasks.md(loom 测试框架与常规 `cargo test` 不兼容).
**Target Platform(目标平台)**: Linux(操作系统) 与 macOS(操作系统) 开发者工作站.
**Project Type(项目类型)**: Tokio(异步运行时) supervisor runtime(监督器运行时).
**Performance Goals(性能目标)**: 单次 `try_consume()` 调用延迟 p99 < 10µs(微秒), 完整 evaluate_budget 阶段 p99 < 100µs, 不影响控制循环主路径延迟.
**Constraints(约束)**: 禁止兼容导出.`src/` Rust 注释英文.规格正文中文且术语 `English(中文说明)`.
**Scale/Scope(规模和范围)**: 单进程内多 group(分组) 拓扑.每个 group 包含若干 child, 每组独立熔断预算.

## Constitution Check(宪章检查)

_GATE(关口): Phase 0(研究阶段) 前必须通过.Phase 1(设计阶段) 后必须重新检查._

- **Module Ownership(模块所有权)**: 策略裁决代码落在 `src/policy/` 模块.观测发射代码落在 `src/observe/` 模块.`src/runtime/control_loop.rs` 只做调度连接. ✅
- **Supervision Contract(监督契约)**: 本切片改变重启节拍与熔断语义.必须写明 RestartBudget(重启预算) 的生命周期状态, MeltdownFuse(熔断器) 的触发-恢复状态机, GroupStrategy(分组策略) 的隔离契约, Critical/Optional(关键/可选) 升级分叉. ✅
- **Test Gate(测试关口)**: 行为变化必须先列测试, 再列实现.测试覆盖: 快速失败波形预算限流, 双分组隔离, 关键/可选事件分叉. ✅
- **Observable Failures(可观察失败)**: 预算耗尽, 熔断触发, 分组隔离违反, 升级路径分叉全部输出 typed event(类型化事件), 并附带 CorrelationId(关联标识). ✅
- **Small Increment(小增量)**: 不新增外部 crate.不新增异步运行时或持久化层.仅在现有 policy 模块上增强. ✅
- **Chinese Writing(中文写作)**: 本文件及派生物使用中文叙述, 英文术语括注. ✅

## Project Structure(项目结构)

### Documentation(文档,本功能)

```text
specs/006-4-restart-policy-production/
├── plan.md              # 本文件,由 /speckit-plan 命令生成
├── research.md          # Phase 0(研究阶段) 输出
├── data-model.md        # Phase 1(设计阶段) 输出
├── quickstart.md        # Phase 1(设计阶段) 输出
├── contracts/           # Phase 1(设计阶段) 输出
└── tasks.md             # Phase 2(任务阶段) 输出,由 /speckit-tasks 命令生成
```

### Source Code(源代码,仓库根目录)

```text
src/
├── policy/
│   ├── mod.rs           # 模块入口
│   ├── decision.rs      # PolicyEngine, RestartDecision(已有, 增强 budget 集成)
│   ├── backoff.rs       # BackoffPolicy + JitterMode(已有)
│   ├── meltdown.rs      # MeltdownPolicy, MeltdownTracker(已有, 增强 group 隔离)
│   ├── failure_window.rs # FailureWindow(已有)
│   ├── role_defaults.rs  # WorkRole, EffectivePolicy(已有, 增强 critical/optional)
│   ├── budget.rs         # NEW: RestartBudget(重启预算) 跟踪器
│   └── group.rs          # NEW: GroupStrategy(分组策略) 隔离断言
├── observe/
│   ├── pipeline.rs      # ObservabilityPipeline(已有, 增强 fairness probe)
│   └── fairness.rs      # NEW: FairnessProbe(公平性探针)
├── event/
│   └── payload.rs       # 已有, 新增 BudgetExhausted, GroupFuseTriggered, EscalationBifurcated 事件
├── runtime/
│   └── control_loop.rs  # 已有, 增强: 接入 budget 评估, group 隔离, fairness 探测
├── spec/
│   ├── supervisor.rs    # 已有, 新增 GroupConfig
│   └── child.rs         # 已有, ChildSpec 新增 severity, group 字段

tests/
├── policy_budget_waveform_test.rs    # NEW: 快速失败波形预算限流测试
├── policy_group_isolation_test.rs    # NEW: 双分组隔离对照测试
├── policy_critical_optional_test.rs  # NEW: critical/optional 分叉观测测试
└── policy_fairness_probe_test.rs     # NEW: 公平性探针测试
```

**Structure Decision(结构决定)**: 采用 Rust 单 crate(包) 结构.新增 `budget.rs` 和 `group.rs` 放在 `src/policy/` 下, `fairness.rs` 放在 `src/observe/` 下.这种分离保持策略逻辑与观测逻辑的边界清晰.

## Complexity Tracking(复杂度跟踪)

> **只有 Constitution Check(宪章检查) 存在违反项时,才填写本节.**

| Violation(违反项) | Why Needed(为什么需要) | Simpler Alternative Rejected Because(为什么拒绝更简单方案) |
| ----------------- | ---------------------- | ---------------------------------------------------------- |
| N/A(不适用)       | -                      | -                                                          |
