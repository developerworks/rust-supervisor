# Implementation Plan(实现计划): 生产级重启策略与分组隔离观测

**Branch(分支)**: `[006-4-restart-policy-production]` | **Date(日期)**: 2026-05-17 | **Spec(规格)**: `specs/006-4-restart-policy-production/spec.md`
**Input(输入)**: 功能规格来自 `specs/006-4-restart-policy-production/spec.md`

## Summary(摘要)

本切片在 006-3 的 ChildSlot 基础上, 接入重启预算 (restart budget), 熔断器 (meltdown fuse), 分组策略 (group strategy), 升级策略 (escalation policy), 退避抖动 (backoff jitter). 失败流水线入口 (decide action 阶段) 已在 005-1 和 005-2 中定义, 本切片将策略评估管线插入 decide action 之前. 核心交付物包括: RestartBudgetSnapshot 数据结构, GroupFaultBoundary 配置模型, SeverityClass 枚举, 策略决策事件与指标字段. 快速失败波形下实测再起频率不得超出文档曲线上界的 105%; 分组 B 在分组 A 熔断时不得出现 5% 以上的额外停机.

## Technical Context(技术背景)

**Language/Version(语言和版本)**: Rust(编程语言) 2024, rust-version 1.88
**Primary Dependencies(主要依赖)**: 本切片不新增 crate. 依赖 005-1 的 failure pipeline(失败流水线) 和 005-2 的 role defaults(角色默认值).
**Storage(存储)**: 策略配置驻留在内存结构 (RestartBudgetSnapshot, GroupFaultBoundary). 可选持久化到配置文件.
**Testing(测试)**: cargo test. 验收夹具必须钉死时钟 (tokio dev test-util) 以验证退避抖动与预算耗尽.
**Target Platform(目标平台)**: Linux(操作系统) 与 macOS(操作系统) 开发者工作站.
**Project Type(项目类型)**: Tokio(异步运行时) supervisor runtime(监督器运行时) 策略模块.
**Performance Goals(性能目标)**: 策略评估在微秒级完成, 不阻塞控制循环调度.
**Constraints(约束)**: 禁止兼容导出. 策略裁决字段必须稳定写入 typed event(类型化事件) 与 metrics(指标).
**Scale/Scope(规模和范围)**: 单进程内多分组多 child(子任务) 策略评估; 分组隔离边界以配置拓扑中的 dependency edge(依赖边) 声明为准.

## Constitution Check(宪章检查)

*GATE(关口): Phase 0(研究阶段) 前必须通过. Phase 1(设计阶段) 后必须重新检查.*

- **Module Ownership(模块所有权)**: 策略评估代码落在 src/policy/ 模块; 分组配置落在 src/config/ 模块; 决策事件字段落在 src/observe/ 模块. main.rs 只做参数解析与依赖拼装.
- **Supervision Contract(监督契约)**: 本切片改变再起节拍与 shutdown(关闭) 耦合节奏. 必须与 006-3 关停切片联合验收.
- **Test Gate(测试关口)**: tasks.md 中测试任务先于实现任务. 验收测试覆盖: 快速失败 10k 波形下的预算曲线, 双分组隔离对照, critical/optional 分叉诊断键差异.
- **Observable Failures(可观察失败)**: 每一次策略裁决写入 typed event 与 metrics, 携带 correlation id. 预算耗尽触发 escalate(升级) 时写入结构化错误.
- **Small Increment(小增量)**: 不新增异步运行时或持久化层. 策略评估作为纯函数注入控制循环.
- **Chinese Writing(中文写作)**: 本文件与派生物使用中文叙述, 英文术语括注.

## Project Structure(项目结构)

### Documentation(文档, 本功能)

```text
specs/006-4-restart-policy-production/
├── spec.md              # 功能规格
├── plan.md              # 本文件
├── research.md          # Phase 0(研究阶段) 输出: 退避算法比较与熔断阈值研究
├── data-model.md        # Phase 1(设计阶段) 输出: RestartBudgetSnapshot / GroupFaultBoundary / SeverityClass 字段定义
├── quickstart.md        # Phase 1(设计阶段) 输出: 策略配置示例与阅读顺序
├── contracts/
│   ├── budget-policy.md         # 预算评估与耗尽契约
│   ├── group-fault-boundary.md  # 分组故障边界契约
│   ├── escalation-policy.md     # 升级策略契约
│   └── backoff-jitter.md        # 退避抖动策略契约
└── tasks.md             # Phase 2(任务阶段) 输出
```

### Source Code(源代码, 仓库根目录)

```text
src/
├── main.rs              # 仅参数解析与依赖拼装
├── policy/
│   ├── mod.rs           # 模块入口
│   ├── budget.rs        # RestartBudgetSnapshot 与预算评估
│   ├── meltdown.rs      # MeltdownFuse 熔断器
│   ├── jitter.rs        # BackoffJitter 退避抖动
│   ├── group.rs         # GroupFaultBoundary 分组故障隔离
│   ├── escalation.rs    # EscalationPolicy 升级策略
│   └── severity.rs      # SeverityClass 分类枚举
├── config/
│   └── policy_config.rs # 策略配置加载与校验
├── observe/
│   ├── mod.rs           # 观测模块入口
│   ├── policy_events.rs # 策略决策事件定义
│   └── policy_metrics.rs # 策略决策指标定义
└── lib.rs               # 公开 API 最小集合, 禁止兼容导出

tests/
├── budget_storm_test.rs        # 快速失败波形预算测试
├── group_isolation_test.rs     # 双分组隔离验收
├── severity_fork_test.rs       # critical/optional 分叉诊断键测试
└── jitter_distribution_test.rs # 退避抖动分布测试
```

**Structure Decision(结构决定)**: 策略模块按策略维度分文件, 便于单文件审阅与单元测试. 决策事件与指标集中在 observe 模块, 与 policy 模块通过契约接口对接.

## Complexity Tracking(复杂度跟踪)

> **只有 Constitution Check(宪章检查) 存在违反项时, 才填写本节.**

| Violation(违反项) | Why Needed(为什么需要) | Simpler Alternative Rejected Because(为什么拒绝更简单方案) |
|---|---|---|
| N/A(不适用) | - | - |
