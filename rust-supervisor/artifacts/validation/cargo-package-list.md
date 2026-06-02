# cargo package --list(打包清单) 验证

- First command(首次命令): `cargo package --list`
- First result(首次结果): failed(失败)
- Failure reason(失败原因): working directory(工作区) 有未提交改动.
- Fix(修复): 按 Cargo(构建工具) 建议使用 `--allow-dirty`, 并收紧 `Cargo.toml` include(包含清单) 为仓库根锚定路径.
- Final command(最终命令): `cargo package --list --allow-dirty`
- Final result(最终结果): passed(通过)
- Evidence(证据): 清单不再包含 `.agents` 开发材料, 并包含 validation artifact(验证产物) 和 SBOM(软件物料清单).
