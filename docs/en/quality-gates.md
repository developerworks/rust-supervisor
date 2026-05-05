# Quality Gates(质量门禁)

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

`scripts/check-coding-standard.sh` 检查必需文档, example(示例), YAML(数据序列化格式) 配置, ASCII(美国信息交换标准代码) 标点约束和 no compatibility(禁止兼容) 约束.

## 可维护性门禁

`scripts/check-maintainability.sh` 检查 manual(手册), docs(文档), quality gate(质量门禁), parallel governance(并行治理), example(示例) 数量和 validation artifact(验证产物).

## SBOM(软件物料清单) 门禁

`scripts/generate-sbom.sh` 生成 CycloneDX JSON(CycloneDX JSON 格式) 和 SPDX JSON(SPDX JSON 格式) 的最小发布产物. `scripts/validate-sbom.sh` 校验格式, 当前 crate(包), `Cargo.lock` 摘要和敏感信息泄漏.
