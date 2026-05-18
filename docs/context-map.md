# 上下文地图 (Context Map)

> 最后更新: 2026-05-18 | 对应版本: 0.1.2

## 一、项目生态总览

```text
                        ┌─────────────────────────────────────┐
                        │          rust-supervisor             │
                        │       (目标进程 / 核心库)             │
                        │                                     │
                        │  GitHub: developerworks/rust-supervisor│
                        │  Crate: rust-tokio-supervisor        │
                        │  目录: ~/rust-supervisor             │
                        └──────────┬──────────────────────────┘
                                   │
                    ┌──────────────┼──────────────┐
                    │              │              │
                    ▼              ▼              ▼
          ┌────────────┐  ┌────────────┐  ┌────────────┐
          │  Specs      │  │  Docs      │  │  Manual    │
          │  (规格)     │  │  (工程文档) │  │  (用户手册) │
          │             │  │            │  │             │
          │ specs/      │  │ docs/      │  │ manual/    │
          │  17 slices  │  │ en/ zh/    │  │ en/ zh/    │
          └────────────┘  └────────────┘  └────────────┘
```

### 关联仓库

| 仓库                  | 角色             | 路径                      | 技术栈                      | IPC 端点               |
| --------------------- | ---------------- | ------------------------- | --------------------------- | ---------------------- |
| rust-supervisor       | 目标进程(核心库) | `~/rust-supervisor`       | Rust + Tokio                | Unix 域套接字          |
| rust-supervisor-relay | 中继             | `~/rust-supervisor-relay` | Rust + Tokio + TLS          | Unix 域套接字 + wss:// |
| rust-supervisor-ui    | 浏览器看板       | `~/rust-supervisor-ui`    | Vue + shadcn-vue + Tailwind | wss:// (仅连接 relay)  |

## 二、源码上下文地图

### 2.1 顶层模块图

```text
src/
├── lib.rs                  # 包入口, 仅 pub mod 声明
├── id/                     # 标识符: ChildId, SupervisorId, SupervisorPath
├── error/                  # 错误类型: SupervisorError, TaskFailureKind
├── config/                 # 配置: SupervisorConfig, ConfigState, YAML loader
├── spec/                   # 规格: ChildSpec, SupervisorSpec, SupervisionStrategy
├── task/                   # 任务工厂: TaskFactory, TaskContext, TaskResult
├── tree/                   # 监督树: SupervisorTree, restart/shutdown order
├── child_runner/           # 子任务执行: ChildRunner, TaskExit
├── policy/                 # 策略引擎: PolicyEngine, MeltdownTracker, BackoffPolicy
├── control/                # 控制命令: SupervisorHandle, ControlCommand
├── runtime/                # 运行时: Supervisor, ControlLoop, SupervisionPipeline
├── shutdown/               # 关闭协调: ShutdownCoordinator, ShutdownResult
├── registry/               # 注册表: RegistryStore, ChildRuntime
├── state/                  # 状态: SupervisorState, ChildState
├── event/                  # 事件: SupervisorEvent, CorrelationId
├── journal/                # 事件日志: EventJournal (环形缓冲区)
├── observe/                # 可观测性: ObservabilityPipeline, MetricsFacade
├── health/                 # 健康检查: HealthPolicy, Heartbeat
├── readiness/              # 就绪检查: ReadinessPolicy, ReadySignal
├── summary/                # 运行摘要: RunSummary
├── test_support/           # 测试支持
├── ipc/ (Unix only)        # IPC 安全: IpcSecurityPipeline, C1-C9
├── dashboard/ (Unix only)  # 看板 IPC: DashboardState, IPC protocol
├── platform/               # 平台工具
└── types/                  # 通用类型
```

### 2.2 包组织结构

```text
Cargo.toml → 定义 package metadata、依赖和 [[test]] targets
rust-toolchain: 无 (通过 Cargo.toml `rust-version` 指定)
src/tests/    → 集成测试 (source_layout, module_boundary, naming_contract 等)
tests/        → 端到端测试 (concurrent_restart, lifecycle_integration 等)
examples/     → 示例 (supervisor_quickstart, demo, restart_policy_lab 等)
```

## 三、文档上下文地图

| 文档          | 路径                             | 读者             | 主要覆盖                  |
| ------------- | -------------------------------- | ---------------- | ------------------------- |
| 产品路线图    | `docs/product-roadmap.md`        | 产品经理、贡献者 | 切片状态、版本计划        |
| 系统架构      | `docs/architecture.md`           | 开发者、架构师   | 模块图、数据流、架构决策  |
| 环境说明      | `docs/environment.md`            | 开发者           | 依赖、工具链、CI/CD       |
| 安全说明      | `docs/security.md`               | 安全官、运维     | IPC 安全、供应链、审计    |
| 运维指南      | `docs/operations.md`             | SRE、值班人员    | 部署、巡检、故障处理      |
| 上下文地图    | `docs/context-map.md`            | 新加入开发者     | 全景概览、关联关系        |
| 变更记录      | `docs/change-log.md`             | 所有人           | 文档变更追踪              |
| 技术决策      | `docs/adr/`                      | 架构师、开发者   | 12 条 ADR                 |
| 质量门禁 (EN) | `docs/en/quality-gates.md`       | 发布经理         | shallow/middle/deep gates |
| 质量门禁 (ZH) | `docs/zh/quality-gates.md`       | 发布经理         | 同上 (中文)               |
| 并行治理 (EN) | `docs/en/parallel-governance.md` | 贡献者           | 并行工作流                |
| 并行治理 (ZH) | `docs/zh/parallel-governance.md` | 贡献者           | 同上 (中文)               |
| 英文手册      | `manual/en/`                     | 用户             | 使用教程、概念说明        |
| 中文手册      | `manual/zh/`                     | 用户             | 同上 (中文)               |

## 四、规格 (Spec) 上下文地图

```text
specs/
├── 001-create-supervisor-core/         [实现中] 基础核心
├── 002-config-schema-support/          [实现中] 配置模型
├── 003-supervisor-dashboard/           [实现中] 看板
├── 004-1-runtime-lifecycle-guard/      [实现中] 运行时生命周期守卫
├── 004-2-real-shutdown-pipeline/       [实现中] 真实关闭流水线
├── 004-3-child-runtime-state-control/  [实现中] 子任务运行状态控制
├── 004-4-generation-fencing/           [实现中] 代次隔离重启
├── 005-1-failure-policy-reliability/   [实现中] 失败策略流水线
├── 005-2-work-role-defaults/           [实现中] 工作角色默认值
├── 006-1-platform-docs-ipc-security/   [实现中] 平台边界与 IPC 安全
├── 006-2-release-supply-chain-gates/   [实现中] 发布门禁与供应链
├── 006-3-lifecycle-shutdown-realism/   [实现中] 真实生命周期关闭
├── 006-4-restart-policy-production/    [计划中] 生产级重启策略
├── 006-5-typed-events-observability/   [草稿]   类型化事件
├── 006-6-config-dynamic-children/      [草稿]   配置与动态子任务
├── 006-7-chaos-soak-reliability/       [草稿]   混沌与浸泡测试
└── 006-8-product-bundle-runbooks/      [草稿]   生产包与运维手册
```

### 依赖关系

```text
001 → 002 → 003
 │            │
 ├→ 004-1 → 004-2 → 004-3 → 004-4
 │                                │
 ├→ 005-1 → 005-2 ──────────────┤
 │                                │
 ├→ 006-1 ──────────────────────┤
 │                                │
 └→ 006-2 ──────────────────────┤
                                 │
         006-3 ◄────────────────┤
           │                    │
     006-4 ◄── 006-5 → 006-6
           │         │
     006-7 ◄─────────┘
           │
     006-8 ◄── 006-1, 006-2, 006-7
```

## 五、测试上下文地图

| 测试目录     | 类型          | 关键文件                                                                                                     |
| ------------ | ------------- | ------------------------------------------------------------------------------------------------------------ |
| `src/tests/` | 集成/契约测试 | `source_layout_test.rs`, `module_boundary_test.rs`, `naming_contract_test.rs`, `supervisor_start_test.rs` 等 |
| `tests/`     | 端到端测试    | `concurrent_restart_test.rs`, `lifecycle_integration.rs`, `ipc_security_integration.rs` 等                   |

CI 三层质量门禁:

- **Shallow gates** (每次 PR): fmt, check, clippy, test, doc, publish_dry_run
- **Middle gates** (发布前): dependency_audit, license_check, advisory_check, semver_checks, msrv_verify
- **Deep gates** (夜间): coverage, mutation_testing, fuzzing, loom, miri

## 六、CI/CD 上下文地图

| 工作流        | 触发条件           | 职责                                     |
| ------------- | ------------------ | ---------------------------------------- |
| Shallow Gates | PR/推送到 main     | 快速质量门禁                             |
| Middle Gates  | 发布/workflow_call | 深度门禁                                 |
| Nightly Gates | 定时 (夜间)        | 覆盖测试、变异测试、模糊测试、loom、miri |
| Pages         | 推送到 main        | 部署 mdBook 文档到 GitHub Pages          |

## 七、外部依赖上下文

本项目依赖以下工具的独立仓库:

- [rust-config-tree](https://github.com/developerworks/rust-config-tree) v0.1.9 — 集中配置加载
- Rust 生态: Tokio 1.52, tracing 0.1, metrics 0.24, serde 1.x, schemars 1.x
- 构建工具: cargo-deny, cargo-audit, cargo-semver-checks, cargo-tarpaulin, cargo-mutants, cargo-fuzz

无外部数据库、消息队列或云服务依赖.

## 八、相关文档

- [产品路线图](product-roadmap.md)
- [系统架构](architecture.md)
- [CHANGELOG](../CHANGELOG.md)
