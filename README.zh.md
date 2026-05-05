# rust-supervisor

`rust-supervisor` 是 Rust(编程语言) 任务监督核心库. 它用声明式配置描述 supervisor(监督器) 树, 用 typed failure(类型化失败) 驱动 restart policy(重启策略), 用 four-stage shutdown(四阶段关闭) 避免 orphan task(孤儿任务), 并把生命周期事实同步到 event journal(事件日志缓冲区), metrics(指标), tracing(结构化追踪) 和 audit event(审计事件).

## 设计原则

- 公开 API(接口) 只来自本项目自有模型.
- 不提供 compatibility wrapper(兼容包装函数), deprecated facade(废弃门面) 或 migration layer(迁移层).
- `current_state`(当前状态) 只回答当前真实状态, 不承担 lifecycle event history(生命周期事件历史) 职责.
- 配置必须通过 rust-config-tree(集中配置树) v0.1.9 加载 YAML(数据序列化格式), 运行时可调常量不得散落到模块内部.
- shutdown(关闭) 必须执行 request stop(请求停止), graceful drain(优雅排空), abort stragglers(强制终止拖尾任务) 和 reconcile(状态对账).
- 关闭术语统一使用 Shutdown Without Orphaned Tasks(关闭后不留下孤儿任务).

## 最小使用方式

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

## 示例命令

```bash
cargo run --example supervisor_quickstart
cargo run --example config_tree_supervisor
cargo run --example restart_policy_lab
cargo run --example shutdown_tree
cargo run --example observability_probe
```

## 发布检查

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

## 文档入口

- `manual/zh/index.md`: 使用者手册入口.
- `manual/en/index.md`: 同构手册入口.
- `docs/zh/index.md`: 工程文档入口.
- `docs/en/index.md`: 同构工程文档入口.
- `docs/zh/quality-gates.md`: 质量门禁说明.
- `docs/zh/parallel-governance.md`: 并行治理说明.
