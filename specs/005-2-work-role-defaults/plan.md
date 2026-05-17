# Implementation Plan(实现计划): 005-2 Work Role Defaults(工作角色默认值)

**Branch(分支)**: `005-2-work-role-defaults` | **Date(日期)**: 2026-05-17 | **Spec(规格)**: `specs/005-2-work-role-defaults/spec.md`
**Input(输入)**: 功能规格来自 `/specs/005-2-work-role-defaults/spec.md`

**Note(说明)**: `/speckit-plan` 命令会填充本模板。执行流程以
`.specify/templates/plan-template.md` 为准。

## Summary(摘要)

本切片要求为五种工作角色(**service**(服务), **worker**(工作者), **job**(作业), **sidecar**(边车), **supervisor**(监督器))定义并实现**RoleDefaultPolicy**(角色默认策略包), 在成功退出、失败退出、人工停止、超时和预算耗尽场景下提供不同的默认行为。依赖 **005-1-failure-policy-reliability** 的失败流水线, 特别是 **evaluate budget**(评估预算) 和 **decide action**(决定动作) 阶段。核心交付物包括: 角色枚举与默认策略数据结构、配置加载集成、运行时默认值注入逻辑、以及针对各角色的验收测试。

## Technical Context(技术背景)

**Language/Version(语言和版本)**: Rust(编程语言) 2024, **rust-version** `1.88` (见根目录 `Cargo.toml`)
**Primary Dependencies(主要依赖)**: **Tokio**(异步运行时), **serde**(序列化), **confique**(配置), **tracing**(结构化追踪); **本切片默认不新增 crate**, **除非 Complexity Tracking(复杂度跟踪)** 登记审计理由。
**Storage(存储)**: N/A(不适用), **角色默认策略驻留在内存配置结构**, **可选持久化到配置文件**。
**Testing(测试)**: `cargo test`, **验收夹具必须能钉死 RNG seed 与注入时钟** (依赖 **`tokio` dev `test-util`**)。
**Target Platform(目标平台)**: **`rust_supervisor` Library**(库), **examples**(示例), **`Linux`(操作系统)** 与 **`macOS`(操作系统)** 上的开发者工作站常见组合。
**Project Type(项目类型)**: **`Tokio`(异步运行时)** **`supervisor runtime`(监督器运行时)**。
**Performance Goals(性能目标)**: **本切片不定义吞吐量或延迟的硬性数值阈值**, **但要求默认值查找与注入在微秒级完成**, **不影响控制循环主路径延迟**。
**Constraints(约束)**: **禁止 compatibility exports(兼容导出)**, **`src/` Rust 注释英文**, **规格正文中文且术语 **`English(中文说明)`**; **默认行为不得违反宪章规定的监督契约边界**。
**Scale/Scope(规模和范围)**: **单进程内一棵或多棵监督树**, **每个子任务声明一个工作角色**, **默认策略包在启动时一次性计算\*\*。

## Constitution Check(宪章检查)

_GATE(关口): Phase 0(研究阶段) 前必须通过。Phase 1(设计阶段) 后必须重新检查。_

### Phase 0 gate (初次)

- **Module Ownership(模块所有权)**: **角色默认策略数据结构**落在 **`src/policy/role_defaults.rs`**; **配置加载集成**落在 **`src/config/`** 模块; **运行时默认值注入逻辑**落在 **`src/runtime/control_loop.rs`** 或其抽取后的同名职责模块; **入口文件不堆积编排分支**。
- **Supervision Contract(监督契约)**: **本切片定义子任务在成功、失败、人工停止、超时和预算耗尽场景下的默认行为**, **这些默认值作为监督契约的基线**; **`success`(成功)** 路径必须保持可观测; **`manual_stop`(人工停止)** 与 **`external_cancel`(外部取消)** 优先于自动重启; **`shutdown`(关闭)** 语义保持与宪章规定的边界一致。
- **Test Gate(测试关口)**: **`tasks.md`** 尚未生成; **暂以 **`cargo test`** 默认全量为合并闸门**; **新增测试文件名在 **`/speckit-tasks`** 阶段逐项列出**。
- **Observable Failures(可观察失败)**: **默认行为决策必须写入结构化事件载荷**; **角色类型、默认策略选择理由必须在日志或事件中可见**。
- **Small Increment(小增量)**: **不默认引入新异步运行时或持久化层**; **角色默认策略包作为纯数据结构**, **不包含复杂状态机**。
- **Chinese Writing(中文写作)**: **本文件与派生物使用中文叙述**, **英文术语括注**。

### Phase 1 post-design gate (复检)

- **`research.md`** 已无 NEEDS CLARIFICATION(需要澄清) 条目。
- **`data-model.md`** 已写明 **五种工作角色的枚举定义** 与 **RoleDefaultPolicy**(角色默认策略包) 字段义务。
- **`contracts/`** 已冻结 **角色到默认行为的映射规则** 与 **配置覆盖优先级**。
- **`quickstart.md`** 已给出 **`src/`** 阅读顺序锚点。

## Project Structure(项目结构)

### Documentation(文档，本功能)

```text
specs/005-2-work-role-defaults/
├── plan.md              # 本文件，由 /speckit-plan 命令生成
├── spec.md              # 功能规格
├── research.md          # Phase 0(研究阶段) 输出
├── data-model.md        # Phase 1(设计阶段) 输出
├── quickstart.md        # Phase 1(设计阶段) 输出
├── contracts/           # Phase 1(设计阶段) 输出
│   └── role-defaults.md # 角色默认行为契约
└── tasks.md             # Phase 2(任务阶段) 输出，由 /speckit-tasks 命令生成
```

### Source Code(源代码，仓库根目录)

```text
src/
├── policy/
│   ├── mod.rs
│   ├── meltdown.rs      # 现有熔断跟踪器
│   ├── decision.rs      # 现有策略决策引擎
│   └── role_defaults.rs # 新增: 工作角色枚举与默认策略包
├── config/
│   ├── mod.rs
│   ├── yaml.rs          # 现有 YAML 配置加载
│   └── configurable.rs  # 现有配置结构
├── runtime/
│   ├── mod.rs
│   ├── control_loop.rs  # 控制循环编排, 注入角色默认值
│   └── lifecycle.rs     # 运行时生命周期状态
├── spec/
│   ├── mod.rs
│   ├── child.rs         # 子任务规格, 包含角色声明
│   └── supervisor.rs    # 监督器规格
└── state/
    ├── mod.rs
    ├── child.rs         # 子任务状态
    └── supervisor.rs    # 监督器状态

tests/
└── work_role_defaults_integration.rs # 端到端集成测试
```

**Structure Decision(结构决定)**: 采用 **Rust 单 crate(包) 项目**结构。**角色默认策略数据结构**新增于 **`src/policy/role_defaults.rs`**; **配置集成**复用现有 **`src/config/`** 模块; **运行时注入逻辑**落在 **`src/runtime/control_loop.rs`**; **验收测试**放在 **`tests/work_role_defaults_integration.rs`**。不引入 Web 应用或移动端结构。

## Complexity Tracking(复杂度跟踪)

> **只有 Constitution Check(宪章检查) 存在违反项时，才填写本节。**

| Violation(违反项)                   | Why Needed(为什么需要) | Simpler Alternative Rejected Because(为什么拒绝更简单方案) |
| ----------------------------------- | ---------------------- | ---------------------------------------------------------- |
| [例如第四个项目]                    | [当前需要]             | [为什么三个项目不够]                                       |
| [例如 Repository pattern(仓储模式)] | [具体问题]             | [为什么直接访问数据库不够]                                 |
