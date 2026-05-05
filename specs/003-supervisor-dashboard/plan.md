# Implementation Plan(实现计划): 监督任务可视化界面

**Branch(分支)**: `003-supervisor-dashboard` | **Date(日期)**: 2026-05-06 | **Spec(规格)**: `specs/003-supervisor-dashboard/spec.md`
**Input(输入)**: 功能规格来自 `/specs/003-supervisor-dashboard/spec.md`

## Summary(摘要)

本功能交付一个 target process IPC(目标进程进程间通信) 加 sidecar relay(侧车中继) 加 `wss://` WebSocket(网络套接字协议) 的 dashboard(看板) 方案. 目标进程通过外部化 IPC path(进程间通信路径) 配置打开本机 Unix domain socket(Unix 域套接字), 并在 IPC(进程间通信) 连接建立后主动推送 supervisor event(监督器事件), log record(日志记录) 和可用状态变化. sidecar(侧车进程) 在远程客户端完成 mTLS(双向传输层安全协议认证) 和 control session(控制会话) 之后, 才能连接或绑定一个或多个目标进程 IPC(进程间通信). dashboard(看板) 通过 `wss://` 接收 target process list(目标进程列表), snapshot(快照), event(事件), log(日志), state delta(状态增量), command result(命令结果) 和 error(错误), 并提供树形状态查看, 流式过滤和受审计控制命令.

## Technical Context(技术背景)

**Language/Version(语言和版本)**: Rust(编程语言) 2024, rust-version(编译器版本) 1.88, TypeScript(类型脚本语言) 5 用于 dashboard(看板) 前端.
**Primary Dependencies(主要依赖)**: 已有 `tokio`, `tokio-util`, `serde`, `serde_json`, `thiserror`, `tracing`, `uuid`, `confique`, `schemars`; 新增 `tokio-stream` 用于事件流适配, `tokio-tungstenite` 用于 WebSocket(网络套接字协议), `tokio-rustls` 和 `rustls-pemfile` 用于 `wss://` 和 mTLS(双向传输层安全协议认证), `x509-parser` 用于 certificate identity(证书身份) 解析, `futures-util` 用于异步 stream(流) 和 sink(写入端) 组合. dashboard(看板) 前端使用 `Vite(前端构建工具)`, `React(网页界面库)`, `TypeScript(类型脚本语言)`, `React Flow(流程图组件)`, `Vitest(前端测试工具)` 和 `Playwright(浏览器测试工具)`.
**Storage(存储)**: N/A(不适用). 第一版不引入持久化数据库, snapshot(快照), event(事件) 和 log(日志) 来自目标进程内存状态和 EventJournal(事件日志缓冲区).
**Testing(测试)**: `cargo test`, `npm --prefix dashboard test`, `npm --prefix dashboard run test:e2e`.
**Target Platform(目标平台)**: Linux(操作系统) 和 macOS(操作系统) 服务端, 浏览器 dashboard(看板). Windows(操作系统) named pipe(命名管道) 不进入第一版范围.
**Project Type(项目类型)**: Rust(编程语言) library(库) 加 sidecar binary(侧车二进制入口) 加 colocated web dashboard(同仓网页看板).
**Performance Goals(性能目标)**: 已认证操作者 2 秒内看到首个 target process list(目标进程列表) 和至少一个 snapshot(快照); 5 个目标进程和 200 个 child task(子任务) 在 5 秒内首次可用展示; IPC(进程间通信) 断开后 10 秒内显示 unavailable(不可用) 或 reconnecting(重连中); 同一目标进程 sequence(序号) 展示倒序次数为 0.
**Constraints(约束)**: 外网控制只能通过 `wss://` 和 mTLS(双向传输层安全协议认证); `ws://` 不允许完整控制; 未建立 control session(控制会话) 的远程客户端不得触发 IPC(进程间通信) 连接或绑定; sidecar(侧车进程) 不直接持有 SupervisorHandle(监督器句柄); 目标进程 IPC(进程间通信) 不得暴露到外网; trusted proxy(可信代理) 身份头只能来自可信代理地址; 不提供 compatibility export(兼容导出), 旧协议别名或历史控制命令别名.
**Scale/Scope(规模和范围)**: 第一版至少支持 5 个静态配置 IPC path(进程间通信路径), 总计 200 个 child task(子任务), 多个已认证 dashboard session(看板会话), 最近事件和日志以内存 ring buffer(环形缓冲区) 为准.

## Constitution Check(宪章检查)

*GATE(关口): Phase 0(研究阶段) 前通过. Phase 1(设计阶段) 后重新检查.*

- **Module Ownership(模块所有权)**: 通过 `src/dashboard/` 拥有 dashboard inspection(看板检查), IPC protocol(进程间通信协议), snapshot model(快照模型), target registry(目标注册表), sidecar session(侧车会话), mTLS identity(双向传输层安全协议认证身份), command audit(命令审计) 和 diagnostics(诊断). `src/bin/rust-supervisor-dashboard-sidecar.rs` 只保留入口连接和配置加载逻辑. `src/lib.rs` 只新增 `pub mod dashboard;`, 不添加 compatibility export(兼容导出).
- **Supervision Contract(监督契约)**: 本功能读取监督树, 状态, 事件和日志, 并允许 restart child(重启子任务), pause child(暂停子任务), resume child(恢复子任务), quarantine child(隔离子任务), remove child(移除子任务), add child(添加子任务) 和 shutdown tree(关闭监督树). 所有命令必须通过目标进程内已有 SupervisorHandle(监督器句柄) 和 control loop(控制循环) 执行. shutdown tree(关闭监督树) 必须保持 Shutdown Without Orphaned Tasks(关闭后不留下孤儿任务) 语义. 取消, 超时, 目标不存在, 命令非法, 认证失败和授权失败都返回结构化错误, 并产生诊断或 audit event(审计事件).
- **Test Gate(测试关口)**: 每个用户故事先列 `tests/` 或 `dashboard/tests/` 中的契约测试, 集成测试或浏览器测试, 再列实现任务. 最终验证范围为 `cargo test`, `npm --prefix dashboard test`, `npm --prefix dashboard run test:e2e`.
- **Observable Failures(可观察失败)**: IPC path(进程间通信路径) 配置冲突, IPC(进程间通信) 不可达, target id(目标标识) 重复, `ws://` 完整控制拒绝, 目标进程 IPC(进程间通信) 外网绕过拒绝, mTLS(双向传输层安全协议认证) 证书缺失, certificate identity(证书身份) 不可解析, trusted proxy(可信代理) 伪造身份拒绝, unauthorized(未授权), command rejected(命令拒绝), 旧协议别名拒绝, 历史控制命令别名拒绝, event dropped(事件丢弃), sequence gap(序号缺口) 和 reconnect timeout(重连超时) 都必须返回结构化错误, tracing(结构化追踪) 字段和 dashboard(看板) 诊断.
- **Small Increment(小增量)**: 新依赖只服务三个边界: Unix domain socket(Unix 域套接字) IPC(进程间通信), `wss://` WebSocket(网络套接字协议) mTLS(双向传输层安全协议认证), dashboard(看板) 可视化. 不引入数据库, 动态服务发现或旧协议兼容层. 多目标连接和流式转发作为独立模块实现, 避免进入现有 runtime(运行时) 控制循环内部.
- **Chinese Writing(中文写作)**: 本计划, research(研究结论), data model(数据模型), contracts(契约), quickstart(快速开始) 和 tasks(任务) 使用中文写作. 英文术语写成 `English(中文说明)`, 文件路径, crate(库) 名称, 命令和协议字段保持原样.

**Post-Design Check(设计后检查)**: Phase 1(设计阶段) 产物已经把模块所有权, 监督契约, 测试关口, 可观察失败, 小增量和中文写作要求映射到 `research.md`, `data-model.md`, `contracts/` 和 `quickstart.md`. 未发现需要 Complexity Tracking(复杂度跟踪) 的宪章违反项.

## Project Structure(项目结构)

### Documentation(文档, 本功能)

```text
specs/003-supervisor-dashboard/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── config-schema.md
│   ├── ipc-protocol.md
│   └── wss-session.md
├── checklists/
│   └── requirements.md
└── tasks.md
```

### Source Code(源代码, 仓库根目录)

```text
src/
├── bin/
│   └── rust-supervisor-dashboard-sidecar.rs
├── dashboard/
│   ├── mod.rs
│   ├── audit.rs
│   ├── auth.rs
│   ├── command.rs
│   ├── config.rs
│   ├── diagnostics.rs
│   ├── error.rs
│   ├── events.rs
│   ├── ipc_client.rs
│   ├── ipc_server.rs
│   ├── model.rs
│   ├── protocol.rs
│   ├── registry.rs
│   ├── relay.rs
│   ├── session.rs
│   └── snapshot.rs
├── config/
│   ├── configurable.rs
│   └── state.rs
└── lib.rs

tests/
├── dashboard_command_contract_test.rs
├── dashboard_config_test.rs
├── dashboard_control_security_test.rs
├── dashboard_performance_test.rs
├── dashboard_protocol_shape_test.rs
├── dashboard_session_contract_test.rs
├── dashboard_snapshot_test.rs
└── dashboard_stream_test.rs

dashboard/
├── package.json
├── tsconfig.json
├── vite.config.ts
├── playwright.config.ts
├── index.html
├── src/
│   ├── main.tsx
│   ├── api/
│   │   └── session.ts
│   ├── components/
│   │   ├── ConfirmCommandDialog.tsx
│   │   ├── ControlPanel.tsx
│   │   ├── EventLogPanel.tsx
│   │   ├── FilterBar.tsx
│   │   ├── NodeDetailsPanel.tsx
│   │   ├── TargetList.tsx
│   │   └── TopologyCanvas.tsx
│   ├── state/
│   │   ├── eventStore.ts
│   │   └── snapshotStore.ts
│   └── types/
│       └── protocol.ts
└── tests/
    ├── control-commands.spec.ts
    ├── dashboard-performance.spec.ts
    ├── events-filter.spec.ts
    └── snapshot-view.spec.ts

examples/config/
├── dashboard-sidecar.yaml
└── supervisor.yaml

manual/
└── dashboard.md
```

**Structure Decision(结构决定)**: 本功能使用当前 Rust(编程语言) 单 crate(包) 作为运行时和 sidecar(侧车进程) 所有权边界, 并在同一仓库增加 `dashboard/` 前端目录. Rust(编程语言) 生产行为在 `src/dashboard/` 和 `src/bin/` 中实现, dashboard(看板) 前端只通过 `wss://` contract(契约) 和 sidecar(侧车进程) 通信.

## Complexity Tracking(复杂度跟踪)

无宪章违反项. 本功能新增 sidecar binary(侧车二进制入口), IPC(进程间通信) 模块和 dashboard(看板) 前端, 这些是规格要求的最小可交付边界, 不是例外复杂度.
