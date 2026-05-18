# Implementation Plan(实现计划): 配置声明与动态子任务治理

**Branch(分支)**: `006-6-config-dynamic-children` | **Date(日期)**: 2026-05-19 | **Spec(规格)**: `specs/006-6-config-dynamic-children/spec.md`
**Input(输入)**: 功能规格来自 `specs/006-6-config-dynamic-children/spec.md`

**Note(说明)**: 本文件由 `/speckit-plan` 命令生成, 基于 `.specify/templates/plan-template.md` 模板.

## Summary(摘要)

本切片在已有 `src/spec/child.rs` (ChildSpec, TaskKind, RestartPolicy), `src/config/loader.rs` (YAML 配置加载), `src/config/configurable.rs` (SupervisorConfig) 基础上, 完成三件事: (1) 扩展静态 YAML schema 支持 FR-001 要求的 9 类字段(children, dependencies, health, readiness, resource limits, command permissions, environment, secrets reference, restart budgets); (2) 实现 add_child 动态追加的全流水线(解析→校验→注册→拉起→审计持久化)并保证事务原子性; (3) 建立 audit 对账机制(spec 快照哈希 + compensating 事务段落), 确保重启后变更可复盘.

现有基础设施: YAML 配置加载使用 `serde_yaml` 0.9 + `rust-config-tree` 0.1.9, 配置 schema 由 `confique` 0.4.0 派生宏驱动. `src/config/configurable.rs` 定义 `SupervisorConfig`, `src/spec/child.rs` 定义 `ChildSpec`, `src/spec/supervisor.rs` 定义 `SupervisorSpec`. 运行时拓扑在 `src/tree/order.rs` 中维护. 审计重用 `src/event/payload.rs` 和 `src/observe/pipeline.rs`.

## Technical Context(技术背景)

**Language/Version(语言和版本)**: Rust 2024, rust-version 1.88
**Primary Dependencies(主要依赖)**: serde_yaml 0.9(YAML 解析), rust-config-tree 0.1.9(配置树), confique 0.4.0(schema 派生), uuid 1(事务 ID). 不新增外部 crate.
**Storage(存储)**: 审计记录驻留在 `src/journal/ring.rs` 环形缓冲区. 快照哈希存储在进程内存中, 重启后从 YAML 文件重新计算, 不依赖跨重启持久化.

> **审计环形缓冲区策略**: 通用审计通道(非 add_child 专用)在写满时覆盖最旧条目. 但 add_child 事务要求审计记录不丢失(FR-002), 因此 CompensatingRecord 使用独立的高水位审计通道或预留槽位.
> **容量保证**: `event_journal_capacity` 配置值必须 ≥ max_children(1000) × 2 + SC-002 压力脚本条目数. 默认容量为 8192, 足以容纳 1000 次 add_child 审计条目(每次约 2 条: CompensatingRecord + 最终条目). 超出容量时 add_child 返回 `Err(AuditStorageFailure)`, 不静默覆盖.
> **Testing(测试)**: `cargo test`; golden YAML 比对照脚本(解析树 vs 注册表差分); add_child 事务通过注入故障夹具验证原子性和 compensating; 并发通过模拟并发 add_child 请求验证隔离性.
> **Target Platform(目标平台)**: Linux 与 macOS 开发者工作站.
> **Project Type(项目类型)**: Tokio supervisor runtime(监督器运行时), Rust library(库).
> **Performance Goals(性能目标)**: 加载 1000 个 child 的 YAML 文件解析 p99 < 50ms; 单次 add_child API 全流水线 p99 < 10ms(含审计持久化). 拓扑 DAG 环路检测时间复杂度 O(V+E).
> **Constraints(约束)**: 禁止兼容导出. `src/` Rust 注释英文. 规格正文中文且术语 `English(中文说明)`. 新增字段必须与 002 切片的 SupervisorSpec 基线无冲突. 拓扑视图必须可通过运行时 API 查询.
> **Scale/Scope(规模和范围)**: 单进程内单 supervisor 实例; 支持最大 1000 个 child, 最大依赖深度 10 层. add_child 事务当前不跨进程.

## Constitution Check(宪章检查)

_GATE(关口): Phase 0(研究阶段) 前必须通过. Phase 1(设计阶段) 后必须重新检查._

- **Module Ownership(模块所有权)**: YAML 解析层在 `src/config/` 模块, 校验逻辑在 `src/spec/` 模块, 运行时注册在 `src/tree/` 模块. 本切片新增的 ChildDeclaration 解析与 ChildSpec 校验保持目录分层, 不创建 god module(全能模块). ✅
- **Supervision Contract(监督契约)**: 本切片改变监督行为: add_child(追加子任务) 涉及新的子任务启动生命周期. 必须写明 add_child 事务的原子性保证, 部分失败时的补偿段落实体, 以及动态追加 child 与冷启动路径的生命周期一致性. ✅
- **Test Gate(测试关口)**: 行为变化必须先列测试再列实现. 测试覆盖: golden YAML 字段级一致性, add_child 事务原子性(含故障注入), 并发隔离性, 环路检测, 审计对账. ✅
- **Observable Failures(可观察失败)**: YAML 拒绝必须打印 field_path(字段路径) 与人读 hint(提示段落). add_child 失败必须返回结构化错误, 并在审计写入带操作者摘要的失败记录. compensating(补偿) 段落必须写明未完成事务编号. ✅
- **Small Increment(小增量)**: 不新增外部 crate. 不新增持久化层(审计重用 002 的 journal). 配置加载和校验在现有 `src/config/` 和 `src/spec/` 模块上扩展. ✅
- **Chinese Writing(中文写作)**: 本文件及派生物使用中文叙述, 英文术语括注. ✅

## Project Structure(项目结构)

### Documentation(文档, 本功能)

```text
specs/006-6-config-dynamic-children/
├── plan.md              # 本文件, 由 /speckit-plan 命令生成
├── research.md          # Phase 0(研究阶段) 输出
├── data-model.md        # Phase 1(设计阶段) 输出
├── quickstart.md        # Phase 1(设计阶段) 输出
├── contracts/           # Phase 1(设计阶段) 输出
│   ├── child-declaration-schema.md
│   └── add-child-api.md
├── checklists/          # 检查清单
│   └── config.md
└── tasks.md             # Phase 2(任务阶段) 输出, 由 /speckit-tasks 命令生成
```

### Source Code(源代码, 仓库根目录)

```text
src/
├── config/
│   ├── mod.rs            # 已有, 不变
│   ├── loader.rs         # 已有, 增强: 9 类字段解析, 依赖拓扑校验
│   ├── configurable.rs   # 已有, 扩展 SupervisorConfig: children, dependencies 等
│   ├── state.rs          # 已有, 增强 ConfigState: add_child 事务
│   └── yaml.rs           # 已有, 不变
├── spec/
│   ├── child.rs          # 已有, 扩展 ChildSpec: resource_limits, secrets_ref 等
│   ├── supervisor.rs     # 已有, 扩展 SupervisorSpec: group_configs 增强
│   ├── child_declaration.rs  # NEW: ChildDeclaration 解析与校验
│   └── mod.rs            # 已有, 注册 child_declaration
├── tree/
│   ├── order.rs          # 已有, 扩展: 动态 add_child 拓扑更新 + 环路检测
│   └── mod.rs            # 已有
├── event/
│   └── payload.rs        # 已有, 新增 ChildDeclarationRejected, ChildDeclarationAccepted 等事件
├── observe/
│   └── pipeline.rs       # 已有, 不变
└── runtime/
    └── control_loop.rs   # 已有, 增强: add_child RPC 入口

tests/
├── golden_yaml_consistency_test.rs  # NEW: golden YAML 字段级一致性测试
├── add_child_transaction_test.rs    # NEW: add_child 事务原子性与故障注入测试
└── topology_concurrent_test.rs      # NEW: 并发 add_child 隔离性测试
```

**Structure Decision(结构决定)**: 采用 Rust 单 crate(包) 结构. YAML 声明解析在 `src/config/` 下, ChildSpec 校验在 `src/spec/` 下, 动态注册在 `src/tree/` 下. 新增 `ChildDeclaration` 类型放在 `src/spec/child_declaration.rs` 中, 与 `ChildSpec` 同属 spec 模块. 这种分离保持解析/校验/注册三层边界清晰.

## Complexity Tracking(复杂度跟踪)

> **本切片不违反 Constitution Check(宪章检查). 以下为本切片特有的复杂度说明, 非违反项.**

| Complexity(复杂度项)                         | Why Needed(为什么需要)                                     | Simpler Alternative Rejected Because(为什么拒绝更简单方案) |
| -------------------------------------------- | ---------------------------------------------------------- | ---------------------------------------------------------- |
| add_child 五步事务(解析→校验→注册→拉起→审计) | 确保动态追加的原子性, 任一步失败整体回退                   | 半解析 manifest: 注册成功后拉起失败时泄漏孤 child          |
| compensating 段落                            | 断电等非优雅失败后, 重启时可通过补偿段落实体识别未完成事务 | 仅整体回退: 断电后无法判断操作是否已持久化, 需遍历比对     |
| 拓扑 DAG 环路检测                            | 防止依赖声明中引入循环导致启动死锁                         | 无环路检测: 启动时出现死锁, 无明确错误信息                 |
| 9 类字段 schema 扩展                         | 覆盖 spec FR-001 的数据类型、必填/可选、默认值             | 最小字段集: 无法满足用户故事的要求                         |
