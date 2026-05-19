# 文档变更记录 (Documentation Change Log)

> 最后更新: 2026-05-19

本文档追踪 `docs/` 目录及关联文档的结构化变更, 与 `CHANGELOG.md` (代码变更) 互补.

## 2026-05-19

### 新增

- `tests/chaos/`: 混沌测试套件 (11 个故障波形场景, 对应 006-7 切片)
  - `child_panic_storm`, `child_block_forever`, `child_ignore_cancel`
  - `rapid_failure_10k`, `slow_event_subscriber`, `command_channel_full`
  - `ipc_connection_storm`, `socket_path_contention`, `relay_crash_loop`
  - `clock_step_backward`, `runtime_starvation_probe`
- `tests/chaos/verdict.rs`: JSON 判决书 schema 和校验测试
- `tests/chaos/fixtures/`: FixtureChildSpawner, FixtureEventThrottle, FixtureIpcStress (+RateLimiter, ClientClassification), FixtureClockController, FixtureRuntimeProbe
- `tests/soak/`: 24h 浸泡测试框架 (MetricsCollector, SteadyTrafficGenerator, SoakReport, SoakRuntime)
- `tests/chaos_suite.rs`: 混沌套件入口 (cargo test --test chaos_suite)
- `tests/soak_suite.rs`: 浸泡测试入口 (cargo test --test soak_suite)
- `manual/*/operations-runbook.md`: 运维手册 (4 个 P1 场景, 含期望 metrics 和升级路径)
- `scripts/check-tarball-content.sh`: tarball 内容校验脚本
- `scripts/validate-release-matrix.sh`: 放行矩阵校验 + CSV->HTML 转换
- `specs/006-7-chaos-soak-reliability/`: 完整规格工件 (plan, research, data-model, contracts, tasks)
- `specs/006-8-product-bundle-runbooks/`: 完整规格工件 (plan, research, data-model, contracts, tasks)

### 变更

- `Cargo.toml`: 新增 chaos_suite 和 soak_suite 测试目标
- `manual/*/getting-started.md`: 增加步数上限 (Step 1-5 of 5) 和健康自检 JSON schema 引用
- `manual/*/configuration.md`: 增加密钥占位符 (`${SECRET_NAME}`) 和升级章节
- `artifacts/quality-gate-outcome.csv`: 新增 chaos-test, soak-24h 闸门行
- `.specify/feature.json`: 已补充 006-7, 006-8 切片路径

## 2026-05-18

### 新增

- `docs/adr/` 目录: 12 条技术决策记录
  - ADR-001: 构建自有 supervisor 模型
  - ADR-002: TaskFactory fresh future
  - ADR-003: Supervisor Tree 生命周期
  - ADR-004: Tokio 原语
  - ADR-005: 分离 State 和 Event
  - ADR-006: 禁止 Snapshot/View 命名
  - ADR-007: tracing + metrics 可观测性
  - ADR-008: Typed Error + Policy Decision
  - ADR-009: 三目录架构
  - ADR-010: Unix-only IPC
  - ADR-011: Policy Pipeline 顺序
  - ADR-012: 集中化配置
- `docs/security.md`: 安全说明文档
  - IPC 9 项控制点 (C1-C9)
  - 供应链安全 (SBOM/attestation)
  - 代码安全实践
  - 安全配置清单
- `docs/operations.md`: 运维指南
  - 部署步骤和文件布局
  - 巡检指标和事件流
  - 4 种故障处理剧本
  - P1 事故响应速查表
  - 性能调优建议
- `docs/context-map.md`: 上下文地图
  - 项目生态总览图
  - 源码/文档/specs/测试/CI 地图
  - 外部依赖上下文
- `docs/change-log.md`: 文档变更记录

### 变更

- `docs/en/index.md`: 新增 product-roadmap.md, architecture.md 索引
- `docs/zh/index.md`: 新增 product-roadmap.md, architecture.md 索引
- `docs/architecture.md`: 创建系统架构文档
- `docs/product-roadmap.md`: 创建产品路线图文档

## 2026-05-17

- 仓库初始化, 文档目录结构建立
- `docs/en/index.md`: 创建英文工程文档入口
- `docs/zh/index.md`: 创建中文工程文档入口
- `docs/en/quality-gates.md`: 创建质量门禁文档
- `docs/zh/quality-gates.md`: 创建质量门禁文档 (中文)
- `docs/en/parallel-governance.md`: 创建并行治理文档
- `docs/zh/parallel-governance.md`: 创建并行治理文档 (中文)

## 文档结构

```text
docs/
├── README.md (未来)
├── adr/                         # 技术决策记录 (新建)
│   ├── README.md
│   ├── 0001-build-own-supervisor-model.md
│   ├── ... (共计 12 条 ADR)
│   └── 0012-centralized-config-rust-config-tree.md
├── architecture.md              # 系统架构 (新建)
├── change-log.md                # 文档变更记录 (新建)
├── context-map.md               # 上下文地图 (新建)
├── environment.md               # 环境说明 (新建)
├── operations.md                # 运维指南 (新建)
├── product-roadmap.md           # 产品路线图 (新建)
├── security.md                  # 安全说明 (新建)
├── screenshot.png
├── en/
│   ├── index.md                 # 工程文档入口 (英文)
│   ├── quality-gates.md
│   └── parallel-governance.md
└── zh/
    ├── index.md                 # 工程文档入口 (中文)
    ├── quality-gates.md
    └── parallel-governance.md
```
