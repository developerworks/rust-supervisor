# rust-tokio-supervisor

`rust-tokio-supervisor` 是 rust-supervisor 项目在 crates.io(发布注册表) 上使用的 package name(包名). 它是 Rust(编程语言) 任务监督核心库. 它用声明式配置描述 supervisor(监督器) 树, 用 typed failure(类型化失败) 驱动 restart policy(重启策略), 用 four-stage shutdown(四阶段关闭) 避免 orphan task(孤儿任务), 并把生命周期事实同步到 event journal(事件日志缓冲区), metrics(指标), tracing(结构化追踪) 和 audit event(审计事件).

发布包名是 `rust-tokio-supervisor`. Library crate name(库包名) 是 `rust_supervisor`.

## Project Links(项目链接)

- core library(核心库): [rust-supervisor](https://github.com/developerworks/rust-supervisor)
- relay(中继): [rust-supervisor-relay](https://github.com/developerworks/rust-supervisor-relay)
- user interface(用户界面): [rust-supervisor-ui](https://github.com/developerworks/rust-supervisor-ui)
- manual(手册): [language selector(语言选择页)](https://developerworks.github.io/rust-supervisor/), [English manual(英文手册)](https://developerworks.github.io/rust-supervisor/en/), [Chinese manual(中文手册)](https://developerworks.github.io/rust-supervisor/zh/)
- dashboard workflow(看板流程): [English(英文)](https://developerworks.github.io/rust-supervisor/en/dashboard.html), [Chinese(中文)](https://developerworks.github.io/rust-supervisor/zh/dashboard.html)

## Design Principles(设计原则)

- 公开 API(接口) 只来自本项目自有模型.
- 不提供 compatibility wrapper(兼容包装函数), deprecated facade(废弃门面) 或 migration layer(迁移层).
- `current_state`(当前状态) 只回答当前真实状态, 不承担 lifecycle event history(生命周期事件历史) 职责.
- 配置必须通过 rust-config-tree(集中配置树) v0.1.9 加载 YAML(数据序列化格式), 运行时可调常量不得散落到模块内部.
- `SupervisorConfig`(监督器配置) 是公开 root configuration struct(根配置结构体), 它同时支持 `confique::Config`(配置派生), `schemars::JsonSchema`(结构模式生成特征), `Serialize`(序列化) 和 `Deserialize`(反序列化).
- dashboard IPC(看板进程间通信) 只属于 target process(目标进程) 本机入口. 当前仓库只实现 Unix domain socket(Unix 域套接字), snapshot(快照), event record(事件记录), log record(日志记录), command mapping(命令映射) 和 shared contract(共享契约).
- shutdown(关闭) 必须执行 request stop(请求停止), graceful drain(优雅排空), abort stragglers(强制中止滞留任务) 和 reconcile(状态对账). `ShutdownTree`(关闭监督树) 会向运行中的 child task(子任务) 发送 `CancellationToken`(取消令牌), 按 shutdown order(关闭顺序) 等待任务返回, 超时后使用 `AbortHandle`(强制中止句柄) 终止滞留任务, 并在 `ShutdownResult`(关闭结果) 中返回 per-child outcome(逐子任务结果) 和 reconcile report(对账报告).
- 关闭术语统一使用 Shutdown Without Orphaned Tasks(关闭后不留下孤儿任务).

## Capability Boundary(能力边界)

- 声明 `ChildSpec`(子任务规格) 和 `SupervisorSpec`(监督器规格).
- 通过 `TaskFactory`(任务工厂) 或 `service_fn`(服务函数) 启动全新的 future(异步任务).
- 使用 `OneForOne`(一对一), `OneForAll`(一对全部) 和 `RestForOne`(后续一组) supervision strategy(监督策略).
- 从 typed failure(类型化失败), backoff(退避), jitter(抖动), fuse rule(熔断规则) 和 policy engine(策略引擎) 生成 `RestartDecision`(重启决策).
- 通过 `SupervisorHandle`(监督器句柄) 控制运行中的树, 包括 `add_child`, `remove_child`, `restart_child`, `pause_child`, `resume_child`, `quarantine_child`, `shutdown_tree`, `current_state`, `subscribe_events`, `is_alive`, `health`, `join` 和 `shutdown`.
- 控制命令必须携带非空 `requested_by`(请求者) 和 `reason`(原因), 公共控制入口和 runtime control loop(运行时控制循环) 都会在执行前校验审计字段.
- `shutdown_tree`(关闭监督树) 成功时返回 `ShutdownResult`(关闭结果). 关闭完成后 `ShutdownResult.report` 携带 `ShutdownPipelineReport`(关闭流水线报告), 其中包含 `ChildShutdownStatus`(子任务关闭状态), `ShutdownReconcileReport`(关闭对账报告) 和 socket status(套接字状态). 核心 runtime(运行时) 不拥有 dashboard IPC socket(看板进程间通信套接字) 时, socket status(套接字状态) 会记录为 `NotOwned`(非运行时拥有).
- `is_alive`(是否存活) 和 `health`(健康报告) 暴露 runtime control plane(运行时控制面)状态. `join`(等待结束) 可以重复读取同一个最终 `RuntimeExitReport`(运行时退出报告). `shutdown`(关闭) 只关闭控制面, 不替代 `shutdown_tree`(监督树关闭).
- 从 `examples/config/supervisor.yaml` 加载主 YAML(数据序列化格式) 配置.
- 复用 `rust_supervisor::config::configurable::SupervisorConfig` 完成 YAML(数据序列化格式) 加载, template generation(模板生成) 和 JSON Schema(JSON 结构模式) 生成.
- 发出 structured log(结构化日志), tracing span(追踪跨度), metrics(指标), audit event(审计事件), event journal entry(事件日志条目) 和 `RunSummary`(运行摘要) diagnostics(诊断信息).
- 通过可选 `ipc` 配置启用 target-side dashboard IPC(目标侧看板进程间通信). target process(目标进程) 只拥有本机 Unix domain socket IPC(Unix 域套接字进程间通信), snapshot(快照) 生成, event conversion(事件转换), command mapping(命令映射) 和 shared JSON contract(共享 JSON 契约).

## 看板

dashboard(看板) 功能固定拆成三个目录.

- [rust-supervisor](https://github.com/developerworks/rust-supervisor) 位于 `~/rust-supervisor`: target process IPC(目标进程进程间通信) 和 shared contract(共享契约).
- [rust-supervisor-relay](https://github.com/developerworks/rust-supervisor-relay) 位于 `~/rust-supervisor-relay`: relay server(中继服务), dynamic registration(动态注册), `wss://`, mTLS(双向传输层安全协议认证), session gating(会话门控) 和 command audit(命令审计).
- [rust-supervisor-ui](https://github.com/developerworks/rust-supervisor-ui) 位于 `~/rust-supervisor-ui`: Vue(网页界面框架), shadcn-vue(组件库), Tailwind(样式框架) dashboard client(看板客户端).

target process(目标进程) 不把 IPC(进程间通信) 暴露到外网. 它只在 `ipc.enabled=true` 时打开本机 Unix domain socket(Unix 域套接字). relay(中继) 可以读取 snapshot(快照), 但是 event(事件) 和 log(日志) subscription(订阅) 必须由已认证 dashboard session(看板会话) 触发.

![rust-supervisor dashboard(看板) screenshot(截图)](docs/screenshot.png)

## Configuration Schema(配置结构模式)

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
    lease_seconds: 30
    registration_heartbeat_interval_seconds: 15
```

当 `ipc.enabled=true` 时, `ipc.path` 和 `ipc.registration.relay_registration_path` 必须是 absolute path(绝对路径). registration(注册) 使用 dynamic registration(动态注册). relay config(中继配置) 不允许写死 target list(目标列表).

## Quick Start(快速开始)

```bash
cargo run --example supervisor_quickstart
```

示例按下面的路径执行.

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

## Examples(示例命令)

```bash
cargo run --example demo -- --config examples/config/supervisor.yaml
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

`cargo run --example demo -- --config examples/config/supervisor.yaml` 是 three-end supervisor demo(三端监督器演示). 它从同一个配置文件启动 library-only supervisor runtime(仅库监督器运行时), 然后在 `examples/demo` 内启动 demo-only dashboard IPC service(仅演示看板进程间通信服务) 和 registration heartbeat(注册心跳). 这个入口不是本 crate(包) 的 production binary(生产二进制程序), 也不会把 demo state(演示状态) 写入核心 `src` 模块.

## Manuals(手册入口)

- [Published manual(已发布手册)](https://developerworks.github.io/rust-supervisor/): generated mdBook site(生成的 mdBook 站点) 的 language selector(语言选择页).
- [English manual(英文手册)](https://developerworks.github.io/rust-supervisor/en/): generated English user manual(生成的英文用户手册).
- [Chinese manual(中文手册)](https://developerworks.github.io/rust-supervisor/zh/): generated Chinese user manual(生成的中文用户手册).
- [Dashboard workflow(看板流程)](https://developerworks.github.io/rust-supervisor/en/dashboard.html): generated three-end dashboard workflow(生成的三端看板流程) 的英文页面.
- [Chinese dashboard workflow(中文看板流程)](https://developerworks.github.io/rust-supervisor/zh/dashboard.html): generated three-end dashboard workflow(生成的三端看板流程) 的中文页面.
- `manual/en/index.md`: English user manual(英文用户手册).
- `manual/zh/index.md`: Chinese user manual(中文用户手册).
- `docs/en/index.md`: English engineering documentation(英文工程文档).
- `docs/zh/index.md`: Chinese engineering documentation(中文工程文档).

## Quality Gates(质量门禁)

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
cargo test --manifest-path ~/rust-supervisor-relay/Cargo.toml
npm --prefix ~/rust-supervisor-ui install
npm --prefix ~/rust-supervisor-ui run test
npm --prefix ~/rust-supervisor-ui run build
npm --prefix ~/rust-supervisor-ui run test:e2e
```

工程门禁详情写在 `docs/en/quality-gates.md` 和 `docs/zh/quality-gates.md`. Parallel implementation governance(并行实现治理) 写在 `docs/en/parallel-governance.md` 和 `docs/zh/parallel-governance.md`.

## License(许可证)

本项目使用 MIT license(MIT 许可证). 详情见 `LICENSE`.
