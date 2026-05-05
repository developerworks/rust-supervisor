# 质量门禁

## 基线命令

```bash
cargo fmt --check
cargo check
cargo test
cargo doc --no-deps
cargo package --list
scripts/check-coding-standard.sh
scripts/check-maintainability.sh
scripts/generate-sbom.sh
scripts/validate-sbom.sh
cargo publish --dry-run
```

## 编码标准门禁

`scripts/check-coding-standard.sh` 检查这些内容:

- 必需发布物料存在.
- 五个 example(示例) 文件存在.
- 主配置 `examples/config/supervisor.yaml` 存在.
- 文档不包含常见中文标点.
- README(说明文档) 不描述 compatibility wrapper(兼容包装函数), migration layer(迁移层) 或 deprecated facade(废弃门面).

## 可维护性门禁

`scripts/check-maintainability.sh` 检查这些内容:

- manual(手册) 和 docs(文档) 的 `zh` 与 `en` 目录入口同构.
- quality gate(质量门禁) 和 parallel governance(并行治理) 文档双语路径存在.
- 示例文件数量满足契约.
- validation artifact(验证产物) 存在.

## SBOM(软件物料清单) 门禁

`scripts/generate-sbom.sh` 生成 `artifacts/sbom/rust-supervisor.cdx.json` 和 `artifacts/sbom/rust-supervisor.spdx.json`. `scripts/validate-sbom.sh` 校验文件存在, JSON(数据交换格式) 形状, package(包) 名称, `Cargo.lock` 摘要, 以及 secret(密钥), token(令牌), 本地绝对路径和构建临时目录泄漏.
