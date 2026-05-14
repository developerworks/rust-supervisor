# 快速开始

语言: [English](../en/getting-started.html)

## 前置条件

本项目是 Rust(编程语言) library(库), 需要 Cargo(构建工具) 和 Tokio(异步运行时) 应用环境. 本仓库示例已经包含运行所需依赖.

主配置文件是 `examples/config/supervisor.yaml`. 配置必须通过 rust-config-tree(集中配置树) v0.1.9 加载 YAML(数据序列化格式), 然后形成 `ConfigState`(配置状态).

## 最小运行命令

```bash
cargo run --example supervisor_quickstart
```

该示例执行固定路径: `load_config_state` 读取 YAML(数据序列化格式), `ConfigState::to_supervisor_spec` 派生 `SupervisorSpec`(监督器规格), `Supervisor::start` 启动 runtime(运行时), `current_state` 查询当前状态, `shutdown_tree` 关闭整棵树.

## 最小代码路径

```rust
use rust_supervisor::config::loader::load_config_state;
use rust_supervisor::runtime::supervisor::Supervisor;

#[tokio::main]
async fn main() -> Result<(), rust_supervisor::error::types::SupervisorError> {
    let state = load_config_state("examples/config/supervisor.yaml")?;
    let spec = state.to_supervisor_spec()?;
    let handle = Supervisor::start(spec).await?;
    let current = handle.current_state().await?;
    println!("{current:#?}");
    handle.shutdown_tree("operator", "quickstart complete").await?;
    Ok(())
}
```

## 运行结果

当前示例用于验证接入路径, 不是业务任务模板. 使用者需要把自己的 worker(工作任务) 放入 `ChildSpec`(子任务规格) 和 `TaskFactory`(任务工厂)边界, 不应该在业务代码中分散启动无人管理的后台任务.
