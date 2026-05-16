# 工程文档入口

## 文档地图

- `quality-gates.md`: quality gate(质量门禁) 和发布检查.
- `parallel-governance.md`: parallel governance(并行治理) 和 Worker D 文档所有权.

## 核心契约

工程实现必须遵守 `specs/001-create-supervisor-core/contracts/public-api.md` 所定义的 public API contract(公开接口契约). 源码入口不得使用 pub use(公开重导出), 不得提供 compatibility method(兼容方法), 不得使用 `super::` relative path(相对路径), 并且示例必须使用本项目自有 API(接口) 名称.

工程文档不得描述 legacy wrapper(遗留包装), migration layer(迁移层) 或 deprecated facade(废弃门面). 关闭文档必须使用 Shutdown Without Orphaned Tasks(关闭后不留下孤儿任务). 配置文档必须使用 rust-config-tree v0.1.9 作为集中 YAML(数据序列化格式)配置边界.

## 发布契约

发布前必须存在 README(说明文档), LICENSE(许可证), CHANGELOG(变更日志), manual(手册), docs(文档), examples(示例), SBOM(软件物料清单) 和 validation artifact(验证产物).
