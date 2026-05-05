# rust-supervisor

`rust-supervisor` 是 Rust(编程语言) 任务监督核心库. 它面向 Tokio(异步运行时) 服务, 提供声明式 supervisor(监督器) 树, child(子任务) 生命周期治理, restart policy(重启策略), four-stage shutdown(四阶段关闭), current_state(当前状态) 查询, event journal(事件日志缓冲区) 和 observability(可观测性) 信号.

本项目没有历史 API(接口) 包袱, 不提供 compatibility wrapper(兼容包装函数), migration layer(迁移层) 或旧名称 alias(别名). 使用者应该通过拥有模块路径导入公开类型, 例如 `rust_supervisor::runtime::supervisor::Supervisor`.

## 能力边界

- 声明 `ChildSpec`(子任务规格) 和 `SupervisorSpec`(监督器规格).
- 通过 `TaskFactory`(任务工厂) 或 `service_fn`(函数适配器) 启动 fresh future(新异步任务).
- 使用 `OneForOne`(一对一), `OneForAll`(一对全部) 和 `RestForOne`(从失败处开始) 监督策略.
- 根据 typed failure(类型化失败), backoff(退避), jitter(抖动), fuse(熔断) 和 policy engine(策略引擎) 产生 `RestartDecision`(重启决策).
- 使用 `SupervisorHandle`(监督器句柄) 执行 `add_child`, `remove_child`, `restart_child`, `pause_child`, `resume_child`, `quarantine_child`, `shutdown_tree`, `current_state` 和 `subscribe_events`.
- 通过 rust-config-tree(集中配置树) v0.1.9 加载 YAML(数据序列化格式) 主配置.
- 输出 structured log(结构化日志), tracing(结构化追踪), metrics(指标), audit event(审计事件), event journal(事件日志缓冲区) 和 `RunSummary`(运行摘要).
- 遵守 Shutdown Without Orphaned Tasks(关闭后不留下孤儿任务) 关闭语义.

## 快速开始

配置文件固定使用 YAML(数据序列化格式), 示例位于 `examples/config/supervisor.yaml`.

```bash
cargo run --example supervisor_quickstart
```

核心调用路径如下:

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

## 示例

```bash
cargo run --example supervisor_quickstart
cargo run --example config_tree_supervisor
cargo run --example restart_policy_lab
cargo run --example shutdown_tree
cargo run --example observability_probe
cargo run --example supervisor_tree_story
cargo run --example runtime_control_story
cargo run --example policy_failure_matrix
cargo run --example diagnostic_replay
```

## 手册

- `manual/zh/index.md`: 中文手册入口, 覆盖配置, 监督树, 任务模型, 策略, 运行时控制, 关闭, 可观测性, 示例和质量门禁.
- `manual/en/index.md`: 同构手册入口, 与中文手册保持相同页面结构.

## 质量门禁

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

当前文档门禁说明见 `docs/zh/quality-gates.md` 和 `docs/en/quality-gates.md`. 并行治理说明见 `docs/zh/parallel-governance.md` 和 `docs/en/parallel-governance.md`.

## 许可证

本项目使用 MIT(麻省理工许可证) 许可证. 详见 `LICENSE`.
