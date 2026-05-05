# CHANGELOG(变更日志)

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
