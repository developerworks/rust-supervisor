# ASSUMPTIONS(假设记录)

## 执行默认值

- 许可证使用 MIT(麻省理工许可证).
- crate(包) 版本使用 `0.1.0`.
- 主配置格式使用 YAML(数据序列化格式), 示例路径是 `examples/config/supervisor.yaml`.
- rust-config-tree(集中配置树) 版本固定为 v0.1.9.
- 当用户没有提供 GitHub(代码托管平台) 仓库状态时, crates.io(软件包发布平台) 验证使用 `cargo publish --dry-run --allow-dirty`.

## API(接口) 默认值

- `ConfigState`(配置状态) 是集中配置加载后的唯一派生入口.
- `ConfigState::to_supervisor_spec` 派生 `SupervisorSpec`(监督器规格).
- `Supervisor::start` 只接收 `SupervisorSpec`(监督器规格), 不提供 compatibility method(兼容方法).
- `SupervisorHandle`(监督器句柄) 提供 `add_child`, `remove_child`, `restart_child`, `pause_child`, `resume_child`, `quarantine_child`, `shutdown_tree`, `current_state` 和 `subscribe_events`.
- 测试文件统一放在模块自己的 `tests/*_test.rs` 或 `src/tests/*_test.rs`.

## 发布默认值

- SBOM(软件物料清单) 生成 CycloneDX JSON(CycloneDX JSON 格式) 和 SPDX JSON(SPDX JSON 格式) 两个文件.
- package include(打包包含清单) 使用仓库根锚定路径, 避免把 `.agents` 等开发材料打入 crate(包).
- Cargo(构建工具) 自动生成的 `.cargo_vcs_info.json` 和 `Cargo.toml.orig` 属于正常打包校验输出.
