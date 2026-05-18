# Implementation Plan(实现计划): 压力故障混沌与浸泡稳定性

**Branch(分支)**: `main` | **Date(日期)**: 2026-05-19 | **Spec(规格)**: `specs/006-7-chaos-soak-reliability/spec.md`
**Input(输入)**: 功能规格来自 `specs/006-7-chaos-soak-reliability/spec.md`

**Note(说明)**: 本文件由 `/speckit-plan` 命令生成, 基于 `.specify/templates/plan-template.md` 模板.

## Summary(摘要)

本切片在已有监督器运行时基础(`src/runtime/`, `src/shutdown/`, `src/policy/`, `src/observe/`)上, 建立一个独立的混沌测试(chaos test)与浸泡测试(soak test)框架, 覆盖 spec 中定义的 11 个故障波形场景和 24h 浸泡稳定性验证. 核心设计决策如下:

1. **隔离策略**: 所有混沌与浸泡源码放在 `tests/chaos/` 和 `tests/soak/` 目录下, 仅通过 `[dev-dependencies]` 引用, 不修改 `src/` 下的默认库代码. 这是宪章 Module Ownership 原则的硬约束.
2. **CI 入口**: 新增 `cargo test --test chaos_suite -- --include-ignored` 作为 CI nightly 调用入口, 运行时输出 JSON 判决书到 stdout.
3. **Soak 框架**: 浸泡测试通过 `cargo test --test soak_suite -- --ignored` 运行, 默认 24h, 产出 SoakReport(浸泡报告) Markdown 到 `artifacts/validation/`.
4. **不修改监督行为**: 混沌 harness(线束) 只通过测试夹具注入故障, 不改变默认二进制发布特性的行为.
5. **已完成的研究结论**: research.md 中记录了 12 项技术研究结论, 包括 Supervisor 启动模式, ShutdownPolicy, 事件系统 API, ChildSlot/AdmissionSet, RestartBudget, 背压策略, IPC 协议, 时钟回拨(断言方案), Tokio 饥饿探测, RSS 平台差异等.

## Technical Context(技术背景)

**Language/Version(语言和版本)**: Rust 2024, rust-version 1.88
**Primary Dependencies(主要依赖)**: 不新增外部 crate. 复用项目中已有的 `tokio`, `serde_json`, `serde_yaml`, `tracing`, `uuid` 等 dev-dependencies. JSON 判决书序列化使用 `serde_json`. 浸泡报告生成使用纯 Rust 文件 IO, 曲线 PNG 通过 CSV 数据 + CI 后处理 Python 脚本生成.
**Storage(存储)**: 混沌场景 JSON 判决书输出到 stdout(测试框架捕获). 浸泡报告 Markdown 写入 `artifacts/validation/soak-<timestamp>.md`, 关联的 CSV 数据和 PNG 曲线图写在同一目录.
**Testing(测试)**: `cargo test --test chaos_suite -- --include-ignored` 运行 11 个混沌场景; `cargo test --test soak_suite -- --ignored` 运行浸泡测试. 混沌与浸泡测试均加 `#[ignore]` 以避免常规 `cargo test` 触发.
**Target Platform(目标平台)**: macOS 开发者工作站(Apple Silicon, 16GB)与 Linux CI runner.
**Project Type(项目类型)**: Rust library(库) + 独立测试入口(chaos_suite, soak_suite).
**Performance Goals(性能目标)**: 11 个混沌场景串行总执行时间不超过 120s. 浸泡测试单次 24h, 缩短版通过 `SOAK_DURATION_MINUTES` 环境变量控制.
**Constraints(约束)**: 禁止修改 `src/` 生产代码. 混沌 harness 仅通过 `[dev-dependencies]` 引用. JSON 判决书格式必须包含 `scenario_id`, `semver`, `passed`, `thresholds`, `started_at_unix_nanos`, `duration_ns`, `error` 七个顶层字段.
**Scale/Scope(规模和范围)**: 单进程内单 supervisor 实例; 11 个故障波形逐一验证, 不包含组合故障场景. 浸泡测试覆盖 5 类指标: p99 latency, RSS growth, FD drift, event gap, shutdown success ratio.

## Constitution Check(宪章检查)

_GATE(关口): Phase 0(研究阶段) 前必须通过. Phase 1(设计阶段) 后必须重新检查._

- **Module Ownership(模块所有权)**: 混沌和浸泡 harness 放在 `tests/chaos/` 和 `tests/soak/` 目录, 通过 `[dev-dependencies]` 引用, 不修改 `src/` 生产代码. `src/` 模块不受本切片影响. ✅
- **Supervision Contract(监督契约)**: N/A(不适用). 本切片不改变监督行为. 混沌 harness 仅通过测试夹具注入故障, 不修改 Supervisor 的生产路径. 原有监督契约(specs/006-3, 006-4, 006-5)语义不变. ✅
- **Test Gate(测试关口)**: 本切片的主体是测试框架. 生产代码无变更, 因此 `cargo test`(排除 chaos/soak) 覆盖率不变. 新增的 chaos 和 soak 测试通过 `#[ignore]` 与常规 CI 隔离. ✅
- **Observable Failures(可观察失败)**: 每条 chaos 场景输出 JSON 判决书, 含 `passed`, `thresholds`, `error` 字段. 浸泡报告输出 Markdown 阈值对照表, 越界条目带 blocking 标记或豁免工单编号. ✅
- **Small Increment(小增量)**: 不新增外部 crate 依赖. 不修改生产代码. `tests/chaos/` 和 `tests/soak/` 目录不影响 crate 发布产物体积. ✅
- **Chinese Writing(中文写作)**: 本文件及派生物使用中文叙述, 英文术语括注. Rust 源码注释(含测试代码中的 `//` 和 `///`)使用英文, 遵循宪章 VI. ✅
- **Compat Exports(兼容导出)**: 本切片不新增任何 `pub use` 或模块重导出. ✅

## Project Structure(项目结构)

### Documentation(文档, 本功能)

```text
specs/006-7-chaos-soak-reliability/
├── plan.md              # 本文件, 由 /speckit-plan 生成
├── spec.md              # 功能规格(Approved)
├── research.md          # Phase 0 输出: 技术研究结论(12 项)
├── data-model.md        # Phase 1 输出: 数据模型定义
├── quickstart.md        # Phase 1 输出: 快速开始
├── contracts/           # Phase 1 输出: 接口契约
│   ├── chaos-scenario-verdict.md   # JSON 判决书 schema
│   └── soak-report-format.md       # SoakReport Markdown 格式契约
├── checklists/
│   └── chaos.md          # 已完成的检查清单(34 项全部通过)
└── tasks.md             # Phase 2 输出: 35 个任务
```

### Source Code(源代码, 仓库根目录)

```text
tests/
├── chaos/
│   ├── mod.rs                      # 模块声明, ChaosScenario 枚举, 公共导入
│   ├── scenarios/
│   │   ├── mod.rs                  # ScenarioRouter: 路由 11 个场景
│   │   ├── child_panic_storm.rs    # FR-001: 子任务反复 panic
│   │   ├── child_block_forever.rs  # FR-001: 子任务永久阻塞
│   │   ├── child_ignore_cancel.rs  # FR-001: 忽略取消
│   │   ├── rapid_failure_10k.rs    # FR-001: 快速失败 10k 次
│   │   ├── slow_event_subscriber.rs # FR-001: 慢事件订阅者
│   │   ├── command_channel_full.rs  # FR-001: 命令通道塞满
│   │   ├── ipc_connection_storm.rs  # FR-001: IPC 连接风暴
│   │   ├── socket_path_contention.rs # FR-001: socket 路径占用
│   │   ├── relay_crash_loop.rs      # FR-001: relay 进程崩溃循环
│   │   ├── clock_step_backward.rs   # FR-001: 时钟回拨
│   │   └── runtime_starvation_probe.rs # FR-001: 运行时饥饿
│   ├── verdict.rs                  # ScenarioVerdict + ThresholdResult + VerdictWriter
│   └── fixtures/
│       ├── mod.rs
│       ├── child_spawner.rs        # FixtureChildSpawner
│       ├── clock_controller.rs     # FixtureClockController
│       ├── event_throttle.rs       # FixtureEventThrottle
│       ├── ipc_stress.rs           # FixtureIpcStress + RateLimiter + ClientClassification
│       └── runtime_probe.rs        # FixtureRuntimeProbe
├── chaos_suite.rs                  # 混沌套件测试入口
├── soak_suite.rs                   # 浸泡测试入口
└── soak/
    ├── mod.rs                      # SoakRuntime
    ├── metrics_collector.rs        # MetricsCollector(平台条件编译 RSS)
    ├── report.rs                   # SoakReport + ReportGenerator
    └── fixtures/
        ├── mod.rs                  # 夹具模块声明
        └── steady_traffic.rs       # SteadyTrafficGenerator
```

### Cargo.toml 新增条目

```toml
[[test]]
name = "chaos_suite"
path = "tests/chaos_suite.rs"

[[test]]
name = "soak_suite"
path = "tests/soak_suite.rs"
```

**Structure Decision(结构决定)**: 采用 Rust 单 crate(包) 结构. 混沌与浸泡测试分别放在 `tests/chaos/` 和 `tests/soak/` 子目录下, 通过独立的测试入口文件(`chaos_suite.rs`, `soak_suite.rs`) 调用. 每个 scenario 独立文件便于并行开发和维护. JSON 判决书 schema 在 `contracts/chaos-scenario-verdict.md` 中正式定义, 与 `tests/chaos/verdict.rs` 实现保持对照. RSS 采集使用 `#[cfg(target_os)]` 条件编译处理 Linux 与 macOS 的 API 差异.

## Complexity Tracking(复杂度跟踪)

> **本切片不违反 Constitution Check. 以下为本切片特有的复杂度说明, 非违反项.**

| Complexity(复杂度项)      | Why Needed(为什么需要)                                            | Simpler Alternative Rejected Because(为什么拒绝更简单方案) |
| ------------------------- | ----------------------------------------------------------------- | ---------------------------------------------------------- |
| 11 个独立 scenario 文件   | 每个故障波形的夹具设置和断言逻辑差异大, 独立文件避免 match 巨函数 | 单文件 match 11 路: 超过 800 行, 可维护性差                |
| JSON 判决书序列化         | spec 要求结构化的可机器解析的输出, 不能只靠日志                   | 纯日志: CI 无法自动判定通过/失败                           |
| soak 测试独立入口         | 24h 执行不阻塞常规 CI, `#[ignore]` 标记保证触发可控               | 放常规 test 中: 每次 `cargo test` 执行 24h, 不现实         |
| 夹具隔离(fixtures subdir) | 多个 scenario 共享 spawn/clock/ipc 夹具, 避免代码重复             | 每个 scenario 自包含夹具: 大量重复 supervisor 初始化代码   |
| 平台条件编译 RSS          | macOS(libc::proc_pidinfo) 与 Linux(/proc/self/status) API 不同    | 引入 procfs crate: 违反"不新增外部 crate"约束              |
