# 质量门禁

语言: [English](../en/quality-gates.html)

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

## 文档同步

manual(手册), docs(工程文档), README(说明文档), examples(示例程序), quickstart(快速开始), public API(公开接口)契约和 glossary(词汇表)需要同步. 公开接口, 配置模式, 示例行为或 observability signal(可观测性信号)变化时, 文档必须同轮更新.

## 编码标准

`scripts/check-coding-standard.sh` 检查发布物料, 示例文件, 主配置, 文档标点和禁止兼容表达. 中文文档必须使用英文标点.

## 可维护性

`scripts/check-maintainability.sh` 检查 manual(手册)和 docs(工程文档)同构入口, 示例数量, 验证产物, Shutdown Without Orphaned Tasks(关闭后不留下孤儿任务)术语和 rust-config-tree(集中配置树)术语.

## SBOM 和发布

`scripts/generate-sbom.sh` 生成 CycloneDX JSON(CycloneDX JSON 格式)和 SPDX JSON(SPDX JSON 格式). `scripts/validate-sbom.sh` 校验文件存在, JSON(数据交换格式)形状, package(包)名称, `Cargo.lock` 摘要和敏感路径泄漏.
