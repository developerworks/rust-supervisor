# rust-tokio-supervisor

`rust-tokio-supervisor` 是 rust-supervisor 项目在 crates.io(发布注册表) 上使用的 package name(包名). 它是 Rust(编程语言) 任务监督核心库. 它用声明式配置描述 supervisor(监督器) 树, 用 typed failure(类型化失败) 驱动 restart policy(重启策略), 用 four-stage shutdown(四阶段关闭) 避免 orphan task(孤儿任务), 并把生命周期事实同步到 event journal(事件日志缓冲区), metrics(指标), tracing(结构化追踪) 和 audit event(审计事件).

发布包名是 `rust-tokio-supervisor`. Library crate name(库包名) 是 `rust_supervisor`.

## 设计原则

- 公开 API(接口) 只来自本项目自有模型.
- 不提供 compatibility wrapper(兼容包装函数), deprecated facade(废弃门面) 或 migration layer(迁移层).
- `current_state`(当前状态) 只回答当前真实状态, 不承担 lifecycle event history(生命周期事件历史) 职责.
- 配置必须通过 rust-config-tree(集中配置树) v0.1.9 加载 YAML(数据序列化格式), 运行时可调常量不得散落到模块内部.
- `SupervisorConfig`(监督器配置) 是公开 root configuration struct(根配置结构体), 它同时支持 `confique::Config`(配置派生), `schemars::JsonSchema`(结构模式生成特征), `Serialize`(序列化) 和 `Deserialize`(反序列化).
- dashboard IPC(看板进程间通信) 只属于 target process(目标进程) 本机入口. 当前仓库只实现 Unix domain socket(Unix 域套接字), snapshot(快照), event record(事件记录), log record(日志记录), command mapping(命令映射) 和 shared contract(共享契约).
- shutdown(关闭) 必须执行 request stop(请求停止), graceful drain(优雅排空), abort stragglers(强制终止拖尾任务) 和 reconcile(状态对账).
- 关闭术语统一使用 Shutdown Without Orphaned Tasks(关闭后不留下孤儿任务).

## Dashboard(看板) 三目录边界

dashboard(看板) 功能固定拆成三个目录.

- `/Users/0x00/Documents/rust-supervisor`: 目标进程 IPC(进程间通信) 配置, 目标侧 IPC(进程间通信) 服务端, snapshot(快照) 生成和共享契约.
- `/Users/0x00/Documents/rust-supervisor-relay`: relay(中继), dynamic registration(动态注册), `wss://`, mTLS(双向传输层安全协议认证), session gating(会话门控) 和 command audit(命令审计).
- `/Users/0x00/Documents/rust-supervisor-ui`: Vue(网页界面框架), shadcn-vue(组件库), Tailwind(样式框架) dashboard client(看板客户端).

target process(目标进程) 不把 IPC(进程间通信) 暴露到外网. 它只在 `ipc.enabled=true` 时打开本机 Unix domain socket(Unix 域套接字). relay(中继) 可以读取 snapshot(快照), 但是 event(事件) 和 log(日志) subscription(订阅) 必须由已认证 dashboard session(看板会话) 触发.

## 配置结构模式

`rust_supervisor::config::configurable::SupervisorConfig` 是 crate user(crate 使用者) 可以复用的配置入口. 它用于 YAML(数据序列化格式) 加载, template generation(模板生成) 和 schema generation(结构模式生成). 使用者不需要为 template(模板) 或 schema(结构模式) 维护第二套模型.

官方配置文件保持单文件:

- `examples/config/supervisor.yaml`: 完整可运行配置.
- `examples/config/supervisor.template.yaml`: 完整单文件模板.

本 crate(包) 不默认写入 `x-tree-split`(树形拆分扩展). 如果使用者项目需要拆分配置文件, 可以在自己的项目中包装或复用 `SupervisorConfig`(监督器配置), 并自行决定 tree split layout(树形拆分布局).

dashboard IPC(看板进程间通信) 的可选配置如下.

```yaml
ipc:
  enabled: true
  target_id: payments-worker-a
  path: /run/rust-supervisor/payments-worker-a.sock
  permissions: "0600"
  bind_mode: create_new
  registration:
    enabled: true
    relay_registration_path: /run/rust-supervisor/dashboard-relay-registration.sock
    display_name: "payments worker a"
    authorization_scope: "payments:operate"
    lease_seconds: 30
```

当 `ipc.enabled=true` 时, `ipc.path` 和 `ipc.registration.relay_registration_path` 必须是 absolute path(绝对路径). registration(注册) 使用 dynamic registration(动态注册). relay config(中继配置) 不允许写死 target list(目标列表).

## 最小使用方式

```rust
use rust_supervisor::runtime::supervisor::Supervisor;

#[tokio::main]
async fn main() -> Result<(), rust_supervisor::error::types::SupervisorError> {
    let handle = Supervisor::start_from_config_file("examples/config/supervisor.yaml").await?;
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
cargo run --example supervisor_tree_story
cargo run --example runtime_control_story
cargo run --example policy_failure_matrix
cargo run --example diagnostic_replay
```

## 手册入口

- `manual/zh/index.md`: 中文手册入口, 覆盖配置, 监督树, 任务模型, 策略, 运行时控制, 关闭, 可观测性, 示例和质量门禁.
- `manual/en/index.md`: 同构手册入口, 与中文手册保持相同页面结构.

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

dashboard(看板) 的验证需要覆盖三个目录.

```bash
cargo test
cargo test --manifest-path /Users/0x00/Documents/rust-supervisor-relay/Cargo.toml
npm --prefix /Users/0x00/Documents/rust-supervisor-ui install
npm --prefix /Users/0x00/Documents/rust-supervisor-ui run test
npm --prefix /Users/0x00/Documents/rust-supervisor-ui run build
npm --prefix /Users/0x00/Documents/rust-supervisor-ui run test:e2e
```

## 文档入口

- `manual/zh/index.md`: 使用者手册入口.
- `manual/en/index.md`: 同构手册入口.
- `docs/zh/index.md`: 工程文档入口.
- `docs/en/index.md`: 同构工程文档入口.
- `docs/zh/quality-gates.md`: 质量门禁说明.
- `docs/zh/parallel-governance.md`: 并行治理说明.
