# Implementation Plan(实现计划): 类型化事件与端到端可追溯闭环

**Branch(分支)**: `006-5-typed-events-observability` | **Date(日期)**: 2026-05-18 | **Spec(规格)**: `specs/006-5-typed-events-observability/spec.md`
**Input(输入)**: 功能规格来自 `specs/006-5-typed-events-observability/spec.md`

**Note(说明)**: 本文件由 `/speckit-plan` 命令生成, 基于 `.specify/templates/plan-template.md` 模板.

## Summary(摘要)

本切片在已有 `src/event/payload.rs` 的 `What` 枚举(30+ 变体, 多数已类型化)基础上, 完成三件事: (1) 补齐控制循环剩余迁移弧的类型化事件变体, 确保每条弧对应唯一的稳定 `SupervisorEvent` 变体; (2) 建立 correlation id(关联标识) 的全链路传播机制, 使得任意一次任务失败的 spawn(拉起) 到 shutdown(关停) 全过程可追溯; (3) 实现 event subscriber(事件订阅者) 慢消费时的背压策略(告警顶住或采样降级), 并在 audit(审计) 通道中记录采样比例.

现有基础设施: `src/event/` 已提供 `What` 枚举(含 ChildStarting, ChildFailed, ChildRestarting, ShutdownRequested 等), `CorrelationId`(UUID v4), `EventSequence`, `EventTime`, `Where`(位置). `src/observe/pipeline.rs` 提供 `ObservabilityPipeline` 将事件扇出到 journal(事件日志), tracing(链路追踪), metrics(指标), audit(审计). 本切片不新增外部 crate, 仅在现有事件模型上做一致性增强.

## Technical Context(技术背景)

**Language/Version(语言和版本)**: Rust 2024, rust-version 1.88
**Primary Dependencies(主要依赖)**: Tokio 1.52.3(sync::broadcast 用于事件通道); uuid 1(已引入, 用于 CorrelationId). 不新增外部 crate.
**Storage(存储)**: N/A(不适用). 事件驻留在 `src/journal/ring.rs` 环形缓冲区, 固定容量由配置 `event_journal_capacity` 控制. 本切片不改变存储后端.
**Testing(测试)**: `cargo test`; 背压行为通过注入时钟和受控 subscriber 延迟仿真; correlation id 链路完整性通过构造多次重启脚本后查询 `TestRecorder` 断言. 新增的 `What` 枚举变体通过冒烟测试验证穷尽覆盖.
**Target Platform(目标平台)**: Linux 与 macOS 开发者工作站.
**Project Type(项目类型)**: Tokio supervisor runtime(监督器运行时), Rust library(库).
**Performance Goals(性能目标)**: 单次事件构造+发射延迟 p99 < 10µs(微秒); 完整扇出到 journal/tracing/metrics/audit 四通道 p99 < 100µs. 不改变控制循环主路径延迟基线.
**Constraints(约束)**: 禁止兼容导出. `src/` Rust 注释英文. 规格正文中文且术语 `English(中文说明)`. 已有 `What` 枚举变体不得重命名已有字段(仅可新增), 保持与 005-1 契约的向后兼容.
**Scale/Scope(规模和范围)**: 单进程内单 supervisor 实例; 每个事件扇出到最多 4 个 subscriber(对应 journal/tracing/metrics/audit). 背压策略对每个 subscriber 独立生效.

## Constitution Check(宪章检查)

_GATE(关口): Phase 0(研究阶段) 前必须通过. Phase 1(设计阶段) 后必须重新检查._

- **Module Ownership(模块所有权)**: `SupervisorEvent` schema 定义集中在 `src/event/payload.rs`(已有). 本切片新增的事件变体追加到同一文件. `src/observe/` 下的 pipeline 和 fairness 模块负责扇出和背压. `src/runtime/control_loop.rs` 只做调度连接, 不持有事件 schema. ✅
- **Supervision Contract(监督契约)**: 本切片不改变监督生命周期状态机(启动/停止/重启/超时/取消/关闭). 仅在已有生命周期弧上附加类型化事件. 契约已在 `specs/005-1-failure-policy-reliability/contracts/pipeline-and-events.md` 中定义, 本切片扩展该契约的事件变体集合. ✅
- **Test Gate(测试关口)**: 行为变化(新增事件变体 + 背压策略)必须先列测试再列实现. 测试覆盖: 每个新 `What` 变体的构造与序列化; correlation id 全链路 5 段覆盖; 背压告警阈值触发; 采样降级时 audit 记录. ✅
- **Observable Failures(可观察失败)**: 事件本身就是可观察失败的主要载体. 本切片确保每条监督弧段必发一个类型化事件. 背压告警和降级事件必须附带触发原因和采样比例. ✅
- **Small Increment(小增量)**: 不新增外部 crate. 不新增持久化层. 不新增后台工作者. 所有增量在现有 `src/event/payload.rs` 和 `src/observe/pipeline.rs` 上做有限扩展. ✅
- **Chinese Writing(中文写作)**: 本文件及派生物使用中文叙述, 英文术语括注. ✅

## Project Structure(项目结构)

### Documentation(文档, 本功能)

```text
specs/006-5-typed-events-observability/
├── plan.md              # 本文件, 由 /speckit-plan 命令生成
├── research.md          # Phase 0(研究阶段) 输出
├── data-model.md        # Phase 1(设计阶段) 输出
├── quickstart.md        # Phase 1(设计阶段) 输出
├── contracts/           # Phase 1(设计阶段) 输出
│   ├── typed-event-schema.md
│   └── correlation-api.md
├── checklists/          # 检查清单
│   └── events.md
└── tasks.md             # Phase 2(任务阶段) 输出, 由 /speckit-tasks 命令生成
```

### Source Code(源代码, 仓库根目录)

```text
src/
├── event/
│   ├── mod.rs            # 已有, 不变
│   ├── payload.rs        # 已有, 新增缺失的 What 变体 + SupervisorEvent 包装
│   └── time.rs           # 已有, CorrelationId + EventSequence 不变
├── observe/
│   ├── mod.rs            # 已有, 不变
│   ├── pipeline.rs       # 已有, 增强背压检测与降级采样
│   ├── fairness.rs       # 已有(006-4), 不变
│   ├── metrics.rs        # 已有, 不变
│   └── tracing.rs        # 已有, 不变
├── journal/
│   └── ring.rs           # 已有, 不变
├── runtime/
│   ├── control_loop.rs   # 已有, 增强: 每条监督弧发射类型化事件
│   └── pipeline.rs       # 已有, 增强: 事件发射阶段的背压感知
├── spec/
│   └── supervisor.rs     # 已有, EventConfig? 可选背压配置
├── shutdown/
│   └── pipeline.rs       # 已有, 增强: 关闭阶段事件类型化
└── config/
    └── loader.rs         # 已有, 新增背压策略配置项

tests/
├── typed_event_coverage_test.rs     # NEW: 穷尽 What 枚举变体冒烟
├── correlation_tracking_test.rs     # NEW: correlation id 5 段覆盖验证
└── backpressure_strategy_test.rs    # NEW: 背压告警与采样降级测试
```

**Structure Decision(结构决定)**: 采用 Rust 单 crate(包) 结构. 事件变体追加到已有 `src/event/payload.rs` 的 `What` 枚举. 背压逻辑嵌入 `src/observe/pipeline.rs`. 这种分离保持事件 schema 与扇出逻辑的边界清晰.

## Complexity Tracking(复杂度跟踪)

> **本切片不违反 Constitution Check(宪章检查). 以下为本切片特有的复杂度说明, 非违反项.**

| Complexity(复杂度项)                  | Why Needed(为什么需要)                                            | Simpler Alternative Rejected Because(为什么拒绝更简单方案) |
| ------------------------------------- | ----------------------------------------------------------------- | ---------------------------------------------------------- |
| What 枚举已 30+ 变体, 新增约 10+ 变体 | 控制循环迁移弧逐条映射到类型化事件                                | 不为遗漏弧建模: 违反宪章"每弧必盖"要求                     |
| 背压策略二选一(runtime 配置)          | 编译期 feature flag 无法在部署后切换                              | 编译期开关: 生产环境无法动态调整, 需要重新编译部署         |
| CorrelationId 跨事件出口传播          | 同一 ID 需嵌入 event payload, tracing span context, metrics label | 仅嵌入 event: tracing/metrics 无法关联到同一 ID, 断层      |
| 采样降级时 audit 全量                 | audit 通道独立于采样策略, 确保合规不丢高风险事件                  | audit 也采样: 合规审计架空, 违反 FR-002                    |
