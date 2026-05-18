# Quickstart(快速开始): 生产包构建与发布

## 构建 MVP tarball

```bash
# 检查 tarball 内容
cargo package --list

# 生成发布 tarball
cargo package

# 输出在 target/package/rust-tokio-supervisor-<version>.crate
```

## 构建手册

```bash
# 需要 mdbook
cargo install mdbook

# 构建手册 HTML
cd manual && mdbook build && cd ..

# 输出在 manual/book/
```

## 验证放行矩阵

```bash
# 检查放行矩阵格式
bash scripts/validate-release-matrix.sh

# 检查 tarball 内容合规
bash scripts/check-tarball-content.sh
```

## 验证健康自检 JSON

```bash
# 启动 supervisor 并获取健康状态
cargo run --example supervisor_quickstart
# 输出应包含符合 health-selfcheck-schema.md 的 JSON
```
