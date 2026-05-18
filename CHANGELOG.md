# CHANGELOG(变更日志)

All notable changes to this project are documented in this file.
Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added(新增)

- IPC(进程间通信) 安全控制点 C1-C9 实现与集成测试.
- 平台支持矩阵与三目录架构说明文档.
- 发布门禁与供应链证明基础设施 (shallow/middle/deep gates).
- 产品路线图文档 `docs/product-roadmap.md` 及工程文档入口索引.
- 系统架构文档 `docs/architecture.md`.
- 工程文档系列: `docs/adr/` (12 条 ADR), `docs/security.md`, `docs/operations.md`, `docs/context-map.md`, `docs/change-log.md`.

### Changed(变更)

- 当前版本仍处于实现阶段, 公开 API(接口) 以 `specs/001-create-supervisor-core/contracts/public-api.md` 为准.

### Migration(迁移脚注)

- **Event schema_id = 1**: 首次冻结事件 schema 版本. 本次新增 10 个 `What` 枚举变体 (`BudgetDenied`, `GenerationFenced`, `HealthCheckPassed`, `HealthCheckFailed`, `Paused`, `Resumed`, `Quarantined`, `BackpressureAlert`, `BackpressureDegradation`, `AuditRecorded`) 和 1 个新顶层字段 (`schema_id: u64`). 序列化格式变更为 `{"type": "snake_case", "payload": {...}}` 结构. 向后兼容: 旧版本事件仍可在 journal 中回放, 未知字段静默忽略.

### Fixed(修复)

-

### Security Notes(安全说明)

- PATCH(补丁级别) 版本如改动高风险示例命令行须单独在此列出.
- 本轮无安全相关修复.

---

## 0.1.0 - Unreleased(未发布)

### Added(新增)

- 增加 supervisor core(监督器核心) 的 README(说明文档) 和中文 README(说明文档).
- 增加 YAML(数据序列化格式) 主配置示例 `examples/config/supervisor.yaml`.
- 增加 quickstart(快速开始), config tree(配置树), restart policy(重启策略), shutdown tree(关闭树) 和 observability probe(可观测性探针) 示例.
- 增加 manual(手册) 和 docs(文档) 的中英同构入口.
- 增加 quality gate(质量门禁), maintainability(可维护性), SBOM(软件物料清单) 生成和 SBOM(软件物料清单) 校验脚本.
- 增加 documentation ownership(文档所有权) 验证记录.

### Changed(变更)

- 当前版本仍处于实现阶段, 公开 API(接口) 以 `specs/001-create-supervisor-core/contracts/public-api.md` 为准.

### Security(安全)

- SBOM(软件物料清单) 校验拒绝 secret(密钥), token(令牌), 本地绝对路径和构建临时目录进入发布产物.
