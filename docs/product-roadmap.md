# 产品路线图 (Product Roadmap)

> 最后更新: 2026-05-18 | 当前版本: 0.1.2 | 目标版本: 1.0.0

## 一、产品愿景

`rust-tokio-supervisor` 是一个基于 Tokio(异步运行时) 的生产级任务监督库。它提供声明式 supervisor(监督器) 树、子任务生命周期治理、可配置重启策略、四阶段关闭管线、当前状态查询、事件日志存储与可观测性信号。项目采用三目录架构（核心库 + 中继 + 用户界面），覆盖从嵌入式单进程到多主机部署的全场景。

## 二、版本规划总览

| 里程碑                 | 目标版本 | 目标日期 | 状态   |
| ---------------------- | -------- | -------- | ------ |
| Foundation（基础核心） | 0.1.x    | 2026-Q2  | 进行中 |
| Alpha（运行时治理）    | 0.2.x    | 2026-Q2  | 规划中 |
| Beta（策略与可靠性）   | 0.3.x    | 2026-Q3  | 规划中 |
| Production（生产就绪） | 0.4.x    | 2026-Q3  | 规划中 |
| Release（正式发布）    | 1.0.0    | 2026-Q4  | 规划中 |

## 三、功能切片状态

项目将功能拆分为 17 个独立切片（spec slice），按 6 个序列组织。每个切片对应一个 `specs/` 子目录。

### 序列一：核心基础 (Foundation)

| 切片 | 功能                                | 状态   | 依赖     | 说明                                                                                      |
| ---- | ----------------------------------- | ------ | -------- | ----------------------------------------------------------------------------------------- |
| 001  | Supervisor Core(监督器核心)         | 实现中 | —        | 声明式 child spec、监督树、重启策略、退避、熔断、就绪检查、四阶段关闭                     |
| 002  | Config Schema Support(配置结构支持) | 实现中 | 001      | `SupervisorConfig` 统一入口，支持 confique、schemars、serde，YAML 模板与 JSON Schema 生成 |
| 003  | Supervisor Dashboard(看板)          | 实现中 | 001, 002 | 三目录方案：核心库 Unix 域套接字 IPC + relay 中继 + Vue 看板 UI                           |

### 序列二：运行时治理 (Runtime Governance)

| 切片  | 功能                                            | 状态   | 依赖       | 说明                                                                      |
| ----- | ----------------------------------------------- | ------ | ---------- | ------------------------------------------------------------------------- |
| 004-1 | Runtime Lifecycle Guard(运行时生命周期守卫)     | 实现中 | 001        | `RuntimeControlPlane` 生命周期模型，is_alive、health、join、shutdown 语义 |
| 004-2 | Real Shutdown Pipeline(真实关闭流水线)          | 实现中 | 001, 004-1 | 取消令牌、关闭顺序等待、超时强制中止、状态对账                            |
| 004-3 | Child Runtime State Control(子任务运行状态控制) | 实现中 | 001, 004-2 | Pause、Remove、Quarantine 语义修正，`ChildRuntimeState` 记录              |
| 004-4 | Generation Fencing(代次隔离重启)                | 实现中 | 001, 004-3 | 异步 pending restart 状态机，同一 ChildId 至多一个活动尝试                |

### 序列三：策略与可靠性 (Policy & Reliability)

| 切片  | 功能                                    | 状态   | 依赖       | 说明                                                                          |
| ----- | --------------------------------------- | ------ | ---------- | ----------------------------------------------------------------------------- |
| 005-1 | Failure Policy Pipeline(失败策略流水线) | 实现中 | 001        | MeltdownTracker 三作用域，BackoffPolicy 全抖动/去相关抖动/并发闸门/冷启动预算 |
| 005-2 | Work Role Defaults(工作角色默认值)      | 实现中 | 001, 005-1 | 五种工作角色(service/worker/job/sidecar/supervisor) 默认策略                  |

### 序列四：生产就绪 (Production Readiness)

| 切片  | 功能                                         | 状态     | 依赖         | 说明                                                                                |
| ----- | -------------------------------------------- | -------- | ------------ | ----------------------------------------------------------------------------------- |
| 006-1 | Platform & IPC Security(平台边界与 IPC 安全) | 实现中   | 001, 003     | 9 项 IPC 控制点、Unix-only 策略、平台支持矩阵、三目录架构固化                       |
| 006-2 | Release & Supply Chain(发布门禁与供应链)     | 实现中   | 001          | 签名标签、semver 验证、MSRV、cargo-deny、cargo-semver-checks、cargo-mutants、覆盖率 |
| 006-3 | Lifecycle Shutdown Realism(真实生命周期关闭) | 实现中   | 004-x        | `ChildSlot` 替换 `ManagedChildState`，真实操作取消令牌与 join handle                |
| 006-4 | Restart Policy Production(生产级重启策略)    | 计划阶段 | 005-x, 006-3 | 重启预算、公平性探针、分组熔断隔离、关键/可选分叉观测                               |

### 序列五：高级可观测性 (Advanced Observability)

| 切片  | 功能                                                 | 状态 | 依赖         |
| ----- | ---------------------------------------------------- | ---- | ------------ |
| 006-5 | Typed Events & Observability(类型化事件与可追溯闭环) | 草稿 | 006-3, 006-4 |
| 006-6 | Config & Dynamic Children(配置声明与动态子任务)      | 草稿 | 002, 006-3   |

### 序列六：最终验证与交付 (Validation & Delivery)

| 切片  | 功能                                        | 状态 | 依赖                |
| ----- | ------------------------------------------- | ---- | ------------------- |
| 006-7 | Chaos & Soak Testing(混沌与浸泡测试)        | 草稿 | 006-3, 006-4, 006-5 |
| 006-8 | Product Bundle & Runbooks(生产包与运维手册) | 草稿 | 006-1, 006-2, 006-7 |

## 四、版本发布计划

### v0.1.x — Foundation (基础核心)

**目标**: 完成序列一（001-003），交付可运行的监督器核心和看板原型。

- [ ] 001: Supervisor Core — 监督树启动/停止/重启、退避熔断、就绪检查、四阶段关闭
- [ ] 002: Config Schema — `SupervisorConfig` 统一加载、模板生成、JSON Schema 导出
- [ ] 003: Dashboard — Unix 域套接字 IPC、snapshot、event/log subscription、relay 注册、Vue UI

**发布检查**: `cargo test` 全量通过、SBOM 校验、`cargo publish --dry-run` 通过

### v0.2.x — Runtime Governance Alpha (运行时治理 Alpha)

**目标**: 完成序列二（004-1 至 004-4），建立完整的运行时生命周期控制。

- [ ] 004-1: Runtime Lifecycle Guard — is_alive、health、join、shutdown 语义
- [ ] 004-2: Real Shutdown Pipeline — 取消令牌、按序等待、超时中止、对账
- [ ] 004-3: Child Runtime State — Pause/Remove/Quarantine 修正、ChildRuntimeState
- [ ] 004-4: Generation Fencing — 代次隔离、pending restart 状态机

### v0.3.x — Policy Beta (策略 Beta)

**目标**: 完成序列三和序列四（005-1 至 006-4），交付生产级策略引擎。

- [ ] 005-1: Failure Policy Pipeline — MeltdownTracker 三作用域、BackoffPolicy 增强
- [ ] 005-2: Work Role Defaults — 五种角色默认策略
- [ ] 006-1: Platform & IPC Security — 9 项 IPC 控制点、平台支持矩阵
- [ ] 006-2: Release & Supply Chain — 发布门禁脚本、供应链证明
- [ ] 006-3: Lifecycle Shutdown Realism — ChildSlot、取消令牌实操
- [ ] 006-4: Restart Budget & Group — 重启预算、分组熔断、公平性探针

### v0.4.x — Production RC (生产就绪候选)

**目标**: 完成序列五和序列六（006-5 至 006-8），交付完整测试证据包和生产文档。

- [ ] 006-5: Typed Events — 类型化事件、correlation id 全链路追踪、慢订阅者背压
- [ ] 006-6: Dynamic Children — 配置声明拓扑、add_child 五步事务、审计持久化
- [ ] 006-7: Chaos & Soak — 11 种混沌场景、24h 浸泡测试、报表自动化
- [ ] 006-8: Product Bundle — 部署指南、运维手册、放行矩阵、MVP tarball

### v1.0.0 — Stable Release (正式发布)

- 所有切片完成实现并通过验收
- 质量门禁全量通过（单元、集成、性质、模糊、loom、混沌、24h 浸泡）
- SBOM、供应链证明、签名标签就绪
- 中英文手册和工程文档同步发布

## 五、依赖关系图

```text
001 ──→ 002 ──→ 003
 │                 │
 ├──→ 004-1 ──→ 004-2 ──→ 004-3 ──→ 004-4
 │                                          │
 ├──→ 005-1 ──→ 005-2 ─────────────────────┤
 │                                           │
 ├──→ 006-1 ───────────────────────────────┤
 │                                          │
 └──→ 006-2 ───────────────────────────────┤
                                            │
                     006-3 ◄────────────────┤
                       │                    │
                 006-4 ◄──── 006-5 ──→ 006-6
                       │           │
                 006-7 ◄───────────┘
                       │
                 006-8 ◄──── 006-1, 006-2, 006-7
```

## 六、关键技术决策

| 决策          | 选择                                  | 理由                                          |
| ------------- | ------------------------------------- | --------------------------------------------- |
| 配置加载      | rust-config-tree v0.1.9               | 集中配置边界，避免常量散落                    |
| 配置模型      | `SupervisorConfig` 统一入口           | 同时支持 confique、schemars、serde            |
| Dashboard IPC | Unix 域套接字，`#[cfg(unix)]`         | 安全（不暴露网络），平台编译隔离              |
| 三目录架构    | 核心库 + relay + UI                   | 进程级解耦，各组件独立演化                    |
| 关闭语义      | Shutdown Without Orphaned Tasks       | 四阶段：请求停止 → 优雅排空 → 强制中止 → 对账 |
| 策略管线      | budget -> meltdown -> backoff 顺序    | 预算不足直接拒绝，熔断后不计算退避            |
| 平台支持      | Unix 全系列核心 + IPC，Windows 仅核心 | Rust `#[cfg(unix)]` 编译期保证                |
| 兼容性        | 禁止兼容导出                          | 降低维护成本，避免 API 膨胀                   |

## 七、相关资源

- [功能规格目录](specs/)
- [CHANGELOG](CHANGELOG.md)
- [质量门禁 - 英文](docs/en/quality-gates.md)
- [质量门禁 - 中文](docs/zh/quality-gates.md)
- [并行治理 - 英文](docs/en/parallel-governance.md)
- [并行治理 - 中文](docs/zh/parallel-governance.md)
- [用户手册 - 英文](manual/en/)
- [用户手册 - 中文](manual/zh/)
- [发布记录](artifacts/release-record.json)
