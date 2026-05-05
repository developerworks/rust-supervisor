# Engineering Docs(工程文档) 入口

## 文档地图

- `quality-gates.md`: quality gate(质量门禁) 和 release readiness(发布就绪) 检查.
- `parallel-governance.md`: parallel governance(并行治理) 和 documentation ownership(文档所有权).

## 核心契约

工程实现必须遵守 public API contract(公开接口契约). 示例只能使用本项目拥有的 API(接口) 名称, 文档不得描述 compatibility wrapper(兼容包装函数), migration layer(迁移层) 或 deprecated facade(废弃门面).
Shutdown documentation must use Shutdown Without Orphaned Tasks(关闭后不留下孤儿任务).

## 发布契约

发布物料必须覆盖 README(说明文档), LICENSE(许可证), CHANGELOG(变更日志), manual(手册), docs(文档), examples(示例), SBOM(软件物料清单) 和 validation artifact(验证产物).
