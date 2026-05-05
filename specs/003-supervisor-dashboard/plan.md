# Implementation Plan(实现计划): 监督任务可视化界面

**Branch(分支)**: `003-supervisor-dashboard` | **Date(日期)**: 2026-05-06 | **Spec(规格)**: `specs/003-supervisor-dashboard/spec.md`
**Input(输入)**: 功能规格来自 `/specs/003-supervisor-dashboard/spec.md`

## Summary(摘要)

本功能交付一个 target process IPC(目标进程进程间通信) 加独立 relay(中继) 加独立 dashboard client(看板客户端) 的远程可视化方案. 当前 `rust-supervisor` 仓库只负责外部化 IPC path(进程间通信路径) 配置, 目标侧 Unix domain socket(Unix 域套接字) 服务端, dynamic registration(动态注册) 上报数据, snapshot(快照) 生成, 客户端会话触发后的事件主动推送和共享协议契约. `/Users/0x00/Documents/rust-supervisor-relay` 负责 relay(中继), 它维护目标进程 active registration(活动注册), 并在远程客户端完成 mTLS(双向传输层安全协议认证) 和 control session(控制会话) 后, 才能连接或绑定一个或多个已注册目标进程 IPC(进程间通信), 再触发事件日志 subscription(订阅), 并通过 `wss://` WebSocket(网络套接字协议) 对外分发状态和命令结果. `/Users/0x00/Documents/rust-supervisor-ui` 负责 dashboard client(看板客户端), 它通过 `wss://` 接收 target process list(目标进程列表), snapshot(快照), event(事件), log(日志), state delta(状态增量), command result(命令结果) 和 error(错误), 并使用 Vue(网页界面框架), shadcn-vue(组件库), Tailwind(样式框架) 和 Vue Flow(流程图组件) 提供树形状态查看, 流式过滤和受审计控制命令入口.

## Technical Context(技术背景)

**Language/Version(语言和版本)**: 当前 `rust-supervisor` 使用 Rust(编程语言) 2024 和 rust-version(编译器版本) 1.88. `/Users/0x00/Documents/rust-supervisor-relay` 使用 Rust(编程语言) 2024. `/Users/0x00/Documents/rust-supervisor-ui` 使用 TypeScript(类型脚本语言) 5 和 Vue(网页界面框架) 3.
**Primary Dependencies(主要依赖)**: 当前 `rust-supervisor` 复用已有 `tokio`, `tokio-util`, `serde`, `serde_json`, `thiserror`, `tracing`, `uuid`, `confique` 和 `schemars`, 并只在目标侧 IPC(进程间通信) 事件流需要时新增 `tokio-stream`. `/Users/0x00/Documents/rust-supervisor-relay` 新增 `tokio-tungstenite` 用于 WebSocket(网络套接字协议), `tokio-rustls` 和 `rustls-pemfile` 用于 `wss://` 和 mTLS(双向传输层安全协议认证), `x509-parser` 用于 certificate identity(证书身份) 解析, `futures-util` 用于异步 stream(流) 和 sink(写入端) 组合. `/Users/0x00/Documents/rust-supervisor-ui` 使用 `Vite(前端构建工具)`, `Vue(网页界面框架)`, `TypeScript(类型脚本语言)`, `shadcn-vue(组件库)`, `Tailwind(样式框架)`, `Vue Flow(流程图组件)`, `Vitest(前端测试工具)` 和 `Playwright(浏览器测试工具)`.
**Storage(存储)**: N/A(不适用). 第一版不引入持久化数据库, snapshot(快照), event(事件) 和 log(日志) 来自目标进程内存状态和 EventJournal(事件日志缓冲区).
**Testing(测试)**: `cargo test`, `cargo test --manifest-path /Users/0x00/Documents/rust-supervisor-relay/Cargo.toml`, `npm --prefix /Users/0x00/Documents/rust-supervisor-ui test`, `npm --prefix /Users/0x00/Documents/rust-supervisor-ui run test:e2e`.
**Target Platform(目标平台)**: Linux(操作系统) 和 macOS(操作系统) 服务端, 浏览器 dashboard(看板). Windows(操作系统) named pipe(命名管道) 不进入第一版范围.
**Project Type(项目类型)**: 三目录交付. 当前 `rust-supervisor` 是 Rust(编程语言) library(库) 和目标侧 IPC(进程间通信) 能力. `/Users/0x00/Documents/rust-supervisor-relay` 是 relay binary(中继二进制入口). `/Users/0x00/Documents/rust-supervisor-ui` 是浏览器 dashboard client(看板客户端).
**Performance Goals(性能目标)**: 已认证操作者 2 秒内看到首个 target process list(目标进程列表) 和至少一个 snapshot(快照); 5 个目标进程和 200 个 child task(子任务) 在 5 秒内首次可用展示; IPC(进程间通信) 断开后 10 秒内显示 unavailable(不可用) 或 reconnecting(重连中); 同一目标进程 sequence(序号) 展示倒序次数为 0.
**Constraints(约束)**: relay(中继) 生产代码必须位于 `/Users/0x00/Documents/rust-supervisor-relay`; dashboard client(看板客户端) 生产代码必须位于 `/Users/0x00/Documents/rust-supervisor-ui`; dashboard client(看板客户端) 必须使用 Vue(网页界面框架), shadcn-vue(组件库) 和 Tailwind(样式框架), 不得使用 React(网页界面库) 组件体系; 当前 `rust-supervisor` 仓库不得新增 relay binary(中继二进制入口) 或同仓前端目录; 外网控制只能通过 `wss://` 和 mTLS(双向传输层安全协议认证); `ws://` 不允许完整控制; 未建立 control session(控制会话) 的远程客户端不得触发 IPC(进程间通信) 连接, 绑定或事件日志 subscription(订阅); 目标进程 dynamic registration(动态注册) 只能把目标放入 relay(中继) registry(注册表), 不得直接触发事件日志主动推送; relay(中继) 不直接持有 SupervisorHandle(监督器句柄); 目标进程 IPC(进程间通信) 不得暴露到外网; trusted proxy(可信代理) 身份头只能来自可信代理地址; 不提供 compatibility export(兼容导出), 旧协议别名或历史控制命令别名.
**Scale/Scope(规模和范围)**: 第一版至少支持 5 个 active registration(活动注册), 总计 200 个 child task(子任务), 多个已认证 dashboard session(看板会话), 最近事件和日志以内存 ring buffer(环形缓冲区) 为准.

## Constitution Check(宪章检查)

*GATE(关口): Phase 0(研究阶段) 前通过. Phase 1(设计阶段) 后重新检查.*

- **Module Ownership(模块所有权)**: 当前 `rust-supervisor` 仓库通过 `src/dashboard/` 只拥有目标侧 IPC protocol(进程间通信协议), snapshot model(快照模型), 目标侧 IPC server(进程间通信服务端), 目标进程 registration payload(注册载荷), 事件转换, 目标侧配置和诊断. `/Users/0x00/Documents/rust-supervisor-relay` 拥有 target registry(目标注册表), dynamic registration(动态注册), relay session(中继会话), mTLS identity(双向传输层安全协议认证身份), command audit(命令审计), IPC client(进程间通信客户端), fan out(分发) 和 relay binary(中继二进制入口). `/Users/0x00/Documents/rust-supervisor-ui` 拥有 Vue(网页界面框架) 浏览器 UI(用户界面), shadcn-vue(组件库) 组件, Tailwind(样式框架) 样式入口, 状态存储和浏览器测试. `src/lib.rs` 只新增 `pub mod dashboard;`, 不添加 compatibility export(兼容导出).
- **Supervision Contract(监督契约)**: 本功能读取监督树, 状态, 事件和日志, 并允许 restart child(重启子任务), pause child(暂停子任务), resume child(恢复子任务), quarantine child(隔离子任务), remove child(移除子任务), add child(添加子任务) 和 shutdown tree(关闭监督树). 所有命令必须通过目标进程内已有 SupervisorHandle(监督器句柄) 和 control loop(控制循环) 执行. shutdown tree(关闭监督树) 必须保持 Shutdown Without Orphaned Tasks(关闭后不留下孤儿任务) 语义. 取消, 超时, 目标不存在, 命令非法, 认证失败和授权失败都返回结构化错误, 并产生诊断或 audit event(审计事件).
- **Test Gate(测试关口)**: 每个用户故事先列当前仓库 `tests/`, relay(中继) 仓库 `/Users/0x00/Documents/rust-supervisor-relay/tests/` 或 UI(用户界面) 仓库 `/Users/0x00/Documents/rust-supervisor-ui/tests/` 中的契约测试, 集成测试或浏览器测试, 再列实现任务. 最终验证范围为 `cargo test`, `cargo test --manifest-path /Users/0x00/Documents/rust-supervisor-relay/Cargo.toml`, `npm --prefix /Users/0x00/Documents/rust-supervisor-ui test` 和 `npm --prefix /Users/0x00/Documents/rust-supervisor-ui run test:e2e`.
- **Observable Failures(可观察失败)**: IPC path(进程间通信路径) 配置冲突, dynamic registration(动态注册) 被拒绝, registration lease(注册租约) 过期, IPC(进程间通信) 不可达, target id(目标标识) 重复, `ws://` 完整控制拒绝, 目标进程 IPC(进程间通信) 外网绕过拒绝, mTLS(双向传输层安全协议认证) 证书缺失, certificate identity(证书身份) 不可解析, trusted proxy(可信代理) 伪造身份拒绝, unauthorized(未授权), command rejected(命令拒绝), 旧协议别名拒绝, 历史控制命令别名拒绝, event dropped(事件丢弃), sequence gap(序号缺口) 和 reconnect timeout(重连超时) 都必须返回结构化错误, tracing(结构化追踪) 字段和 dashboard(看板) 诊断.
- **Small Increment(小增量)**: 新依赖按目录服务三个边界: 当前 `rust-supervisor` 只服务 Unix domain socket(Unix 域套接字) IPC(进程间通信), 目标进程注册载荷和客户端会话触发后的事件推送, relay(中继) 只服务 `wss://` WebSocket(网络套接字协议), mTLS(双向传输层安全协议认证), dynamic registration(动态注册), 多目标连接和流式转发, UI(用户界面) 只服务 Vue(网页界面框架), shadcn-vue(组件库), Tailwind(样式框架) 和 dashboard(看板) 可视化. 不引入数据库, 外部服务发现或旧协议兼容层. dynamic registration(动态注册), 多目标连接和流式转发作为 relay(中继) 独立模块实现, 避免进入现有 runtime(运行时) 控制循环内部.
- **Chinese Writing(中文写作)**: 本计划, research(研究结论), data model(数据模型), contracts(契约), quickstart(快速开始) 和 tasks(任务) 使用中文写作. 英文术语写成 `English(中文说明)`, 文件路径, crate(库) 名称, 命令和协议字段保持原样.

**Post-Design Check(设计后检查)**: Phase 1(设计阶段) 产物已经把三目录模块所有权, 监督契约, 测试关口, 可观察失败, 小增量和中文写作要求映射到 `research.md`, `data-model.md`, `contracts/` 和 `quickstart.md`. 未发现需要 Complexity Tracking(复杂度跟踪) 的宪章违反项.

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

### Source Code(源代码, 三个目录)

```text
/Users/0x00/Documents/rust-supervisor/
src/
├── dashboard/
│   ├── mod.rs
│   ├── config.rs
│   ├── diagnostics.rs
│   ├── error.rs
│   ├── events.rs
│   ├── ipc_server.rs
│   ├── model.rs
│   ├── protocol.rs
│   ├── registration.rs
│   └── snapshot.rs
├── config/
│   ├── configurable.rs
│   └── state.rs
└── lib.rs

tests/
├── dashboard_config_test.rs
├── dashboard_performance_test.rs
├── dashboard_protocol_shape_test.rs
├── dashboard_snapshot_test.rs
└── dashboard_stream_test.rs

/Users/0x00/Documents/rust-supervisor-relay/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── audit.rs
│   ├── auth.rs
│   ├── command.rs
│   ├── config.rs
│   ├── diagnostics.rs
│   ├── error.rs
│   ├── ipc_client.rs
│   ├── registration.rs
│   ├── registry.rs
│   ├── relay.rs
│   └── session.rs
├── tests/
│   ├── relay_command_contract_test.rs
│   ├── relay_config_test.rs
│   ├── relay_control_security_test.rs
│   ├── relay_performance_test.rs
│   ├── relay_session_contract_test.rs
│   └── relay_stream_test.rs
├── examples/config/
│   └── dashboard-relay.yaml
└── manual/
    └── dashboard-relay.md

/Users/0x00/Documents/rust-supervisor-ui/
├── package.json
├── tsconfig.json
├── vite.config.ts
├── components.json
├── tailwind.config.ts
├── postcss.config.js
├── playwright.config.ts
├── index.html
├── src/
│   ├── main.ts
│   ├── App.vue
│   ├── api/
│   │   └── session.ts
│   ├── assets/
│   │   └── main.css
│   ├── components/
│   │   ├── ui/
│   │   ├── ConfirmCommandDialog.vue
│   │   ├── ControlPanel.vue
│   │   ├── EventLogPanel.vue
│   │   ├── FilterBar.vue
│   │   ├── NodeDetailsPanel.vue
│   │   ├── TargetList.vue
│   │   └── TopologyCanvas.vue
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
└── supervisor.yaml

manual/
└── dashboard.md
```

**Structure Decision(结构决定)**: 本功能使用三个固定目录. 当前 `rust-supervisor` 只实现目标侧 IPC(进程间通信) 和共享契约. `/Users/0x00/Documents/rust-supervisor-relay` 单独实现 relay(中继), 它只通过 IPC(进程间通信) 契约和目标进程通信. `/Users/0x00/Documents/rust-supervisor-ui` 单独实现 dashboard client(看板客户端), 它使用 Vue(网页界面框架), shadcn-vue(组件库) 和 Tailwind(样式框架), 并且只通过 `wss://` contract(契约) 和 relay(中继) 通信.

## Complexity Tracking(复杂度跟踪)

无宪章违反项. 本功能新增目标侧 IPC(进程间通信) 模块, 独立 relay binary(中继二进制入口) 和独立 dashboard client(看板客户端), 这些是规格要求的最小可交付边界, 不是例外复杂度.
