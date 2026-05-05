<proposed_plan>
# 监督任务可视化界面方案

## Summary(摘要)

方案改为 `target process IPC(目标进程进程间通信) + sidecar relay(侧车中继) + wss WebSocket(基于传输层安全协议的网络套接字)`.

目标进程只启动本机 IPC(进程间通信) 服务, 它持有 `SupervisorHandle(监督器句柄)` 并提供监督树, 状态, 事件和日志读取能力. 独立 sidecar(侧车进程) 连接目标进程 IPC(进程间通信), 再对外提供 `wss://` WebSocket(网络套接字协议) 服务. mTLS(双向传输层安全协议认证) 在 WebSocket(网络套接字协议) 升级前完成, 认证通过后才允许建立长连接和执行控制命令.

## Key Changes(关键变更)

- 在 `rust-supervisor` 中新增 IPC(进程间通信) inspection(检查) 层. 目标进程通过可选功能启用, 默认只监听 Unix domain socket(Unix 域套接字), 路径由配置指定, 例如 `/run/rust-supervisor/<instance>.sock`.
- IPC(进程间通信) 协议使用 newline-delimited JSON(按行分隔的 JSON 数据), 请求包含 `request_id`, `method`, `params`, 响应包含 `request_id`, `ok`, `result` 或 `error`. 第一版不使用 HTTP(超文本传输协议) 作为目标进程内部协议.
- IPC(进程间通信) 方法固定为 `snapshot`, `events.subscribe`, `logs.tail`, `command.restart_child`, `command.pause_child`, `command.resume_child`, `command.quarantine_child`, `command.remove_child`, `command.add_child`, `command.shutdown_tree`.
- 当前库需要先补类型化观测模型, 因为现有 `current_state` 只返回子任务数量和关闭状态, 现有 `subscribe_events` 只返回文本. 新增 `DashboardSnapshot(看板快照)`, `SupervisorTopology(监督拓扑)`, `SupervisorNode(监督节点)`, `SupervisorEdge(监督边)` 和类型化事件流.
- sidecar(侧车进程) 对外只提供 `wss://` WebSocket(网络套接字协议) 主入口. 握手顺序是 `TCP(传输控制协议) -> TLS(传输层安全协议) + client certificate(客户端证书) -> HTTP Upgrade(HTTP 升级) -> WebSocket(网络套接字协议)`.
- WebSocket(网络套接字协议) 外部消息也使用 JSON(数据交换格式). 服务端启动后先推送 `snapshot` 消息, 后续推送 `event`, `log`, `state_delta`, `command_result`, `error`. 客户端控制命令必须包含 `command_id`, `target`, `reason`; `requested_by` 从 mTLS(双向传输层安全协议认证) 的客户端证书身份派生, 不接受前端自填.
- 前端使用 `Vite(前端构建工具) + React(网页界面库) + TypeScript(类型脚本语言) + React Flow(交互式流程图组件)`. 第一屏包含监督树画布, 节点详情, 控制面板, 事件日志, 连接状态和过滤器.
- 危险操作必须在 UI(用户界面) 二次确认. `shutdown_tree`, `remove_child`, `add_child` 必须填写 reason(原因), 并在 sidecar(侧车进程) 和目标进程审计事件中保留证书身份, 命令, 目标和结果.

## Test Plan(测试计划)

- `rust-supervisor` 增加 IPC(进程间通信) 协议测试, 覆盖 `snapshot`, `events.subscribe`, `logs.tail` 和全部控制命令的请求响应形状.
- `rust-supervisor` 增加 runtime(运行时) 测试, 验证 `DashboardSnapshot(看板快照)` 包含监督树拓扑, 完整状态, 最近事件和日志摘要.
- sidecar(侧车进程) 增加集成测试, 使用测试 Unix domain socket(Unix 域套接字) 连接目标进程假服务, 验证 WebSocket(网络套接字协议) 消息会正确转发到 IPC(进程间通信).
- sidecar(侧车进程) 增加 mTLS(双向传输层安全协议认证) 测试, 无客户端证书不能建立 `wss://` 连接, 非授权证书不能执行控制命令.
- 前端增加 Playwright(浏览器测试), 验证监督树渲染, 事件追加, 日志过滤, 节点详情, 控制命令确认和错误提示.

## Assumptions(假设和默认值)

- 第一版目标平台是 Linux(操作系统) 和 macOS(操作系统), IPC(进程间通信) 使用 Unix domain socket(Unix 域套接字). Windows(操作系统) named pipe(命名管道) 不进入第一版.
- `ws://` 不允许用于远程控制. 外网远程连接只能使用 `wss://` 加 mTLS(双向传输层安全协议认证).
- TLS(传输层安全协议) 默认由 sidecar(侧车进程) 自己终止. 如果部署在 Nginx(反向代理服务器) 或 Envoy(代理服务器) 后面, 必须只信任来自代理的证书身份头, 并在文档里明确禁止公网直接伪造该请求头.
- sidecar(侧车进程) 不直接持有 `SupervisorHandle(监督器句柄)`. 它只通过目标进程 IPC(进程间通信) 与目标进程交互.
- 第一版不做持久化数据库. 日志和事件来自目标进程内存中的 event journal(事件日志缓冲区) 和 IPC(进程间通信) 流.
</proposed_plan>
