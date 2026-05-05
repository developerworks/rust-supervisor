# Research(研究结论): 监督任务可视化界面

## Decision(决定): IPC(进程间通信) 使用 Unix domain socket(Unix 域套接字) 加 newline-delimited JSON(按行分隔的 JSON 数据)

**Rationale(理由)**: 目标平台是 Linux(操作系统) 和 macOS(操作系统), Unix domain socket(Unix 域套接字) 能把目标进程 IPC(进程间通信) 限制在本机文件系统权限边界内. newline-delimited JSON(按行分隔的 JSON 数据) 易于用 serde(序列化库) 建模, 易于在测试中断言, 也避免第一版引入 HTTP(超文本传输协议) 或 gRPC(远程过程调用协议) 的额外服务栈.

**Alternatives considered(备选方案)**: HTTP(超文本传输协议) over Unix socket(Unix 域套接字) 被拒绝, 因为第一版只需要请求响应和订阅流. gRPC(远程过程调用协议) 被拒绝, 因为需要额外 code generation(代码生成) 和 protobuf(协议缓冲) 工具链. TCP(传输控制协议) 本机端口被拒绝, 因为规格要求目标进程 IPC(进程间通信) 不得直接暴露到外网.

## Decision(决定): 目标进程 IPC path(进程间通信路径) 配置放入公开 supervisor config(监督器配置)

**Rationale(理由)**: `rust-tokio-supervisor` 必须提供外部化 IPC path(进程间通信路径) 配置. 计划在 `SupervisorConfig`(监督器配置) 中增加 optional(可选) 的 `ipc` 配置节, 包含 `enabled`, `path`, `target_id`, `bind_mode` 和 `permissions`. 目标进程只在 `enabled` 为 true(真) 时打开本机 IPC(进程间通信), 并且 path(路径) 由调用方配置.

**Alternatives considered(备选方案)**: 使用环境变量被拒绝, 因为现有配置模型已经集中在 YAML(配置文件格式) 和 schema(模式) 中. 自动生成临时 path(路径) 被拒绝, 因为 sidecar(侧车进程) 必须配置一个或多个明确 IPC path(进程间通信路径).

## Decision(决定): sidecar(侧车进程) 使用静态多目标配置和冲突校验

**Rationale(理由)**: 第一版不做动态注册. `DashboardSidecarConfig`(看板侧车配置) 包含 `listen`, `tls`, `trusted_proxy` 和 `targets`. 每个 target(目标) 包含 `target_id`, `display_name`, `ipc_path`, `authorization_scope` 和 `connect_policy`. 配置加载时拒绝重复 target id(目标标识) 和重复 IPC path(进程间通信路径), 保证 dashboard(看板) 中连接状态和命令目标可以稳定归因.

**Alternatives considered(备选方案)**: 服务发现被拒绝, 因为规格只要求多个静态 IPC path(进程间通信路径). 运行时动态注册被拒绝, 因为它需要持久状态和更复杂的权限模型.

## Decision(决定): 外部远程会话使用 `wss://` 加 mTLS(双向传输层安全协议认证)

**Rationale(理由)**: WebSocket(网络套接字协议) 必须和 mTLS(双向传输层安全协议认证) 通过 `wss://` 协同工作. sidecar(侧车进程) 使用 `tokio-rustls` 完成 TLS(传输层安全协议) 握手和 client certificate(客户端证书) 验证, 再使用 `tokio-tungstenite` 完成 WebSocket(网络套接字协议) upgrade(升级). control session(控制会话) 建立后, sidecar(侧车进程) 先发送 target process list(目标进程列表) 和 authorization scope(授权范围), 再按授权触发 IPC(进程间通信) 连接.

**Alternatives considered(备选方案)**: `ws://` 被拒绝, 因为规格禁止其访问完整控制能力. Bearer token(持有者令牌) 被拒绝作为第一身份来源, 因为规格要求双方身份认证. TLS(传输层安全协议) 默认由 sidecar(侧车进程) 终止, 可信代理模式只作为明确配置分支.

## Decision(决定): 事件和日志由目标进程在 IPC(进程间通信) 建立后主动推送

**Rationale(理由)**: 规格要求 IPC(进程间通信) 建立后目标进程主动发送事件. `ipc_server`(进程间通信服务端) 在连接认证和订阅建立后, 从现有 EventJournal(事件日志缓冲区) 提供 recent event(最近事件), 再把新的 SupervisorEvent(监督器事件), LogRecord(日志记录) 和 availability change(可用状态变化) 写入同一连接. 每个 target process(目标进程) 内 sequence(序号) 必须单调展示.

**Alternatives considered(备选方案)**: sidecar(侧车进程) polling(轮询) snapshot(快照) 被拒绝, 因为它无法满足主动推送和顺序要求. 只推送文本事件被拒绝, 因为 dashboard(看板) 需要按 target identity(目标身份), child task(子任务), severity(严重程度), sequence(序号) 和 correlation id(关联标识) 过滤.

## Decision(决定): snapshot(快照) 使用单一 typed model(类型化模型)

**Rationale(理由)**: DashboardSnapshot(看板快照) 聚合 target process identity(目标进程身份), SupervisorTopology(监督拓扑), RuntimeState(运行时状态), recent events(最近事件), recent logs(最近日志), dropped count(丢弃数量), config version(配置版本) 和 generated time(生成时间). 该模型同时用于 IPC(进程间通信) response(响应), `wss://` message(消息) 和 dashboard(看板) 前端 TypeScript(类型脚本语言) 类型生成或手工同步.

**Alternatives considered(备选方案)**: 分散多个接口被拒绝, 因为重连必须获得新的完整 snapshot(快照). 让前端自行拼接监督树被拒绝, 因为操作者不能手工拼接状态.

## Decision(决定): dashboard(看板) 前端放在仓库内 `dashboard/` 目录

**Rationale(理由)**: 规格要求远程 dashboard(看板) 可视化监督树, 状态, 事件, 日志和控制命令. `Vite(前端构建工具)`, `React(网页界面库)`, `TypeScript(类型脚本语言)` 和 `React Flow(流程图组件)` 能快速交付交互式监督树画布, 节点详情, 过滤器和控制面板. 前端只依赖 `wss://` contract(契约), 不直接访问 IPC(进程间通信).

**Alternatives considered(备选方案)**: 纯命令行 dashboard(看板) 被拒绝, 因为用户故事要求远程可视化界面. 服务端渲染被拒绝, 因为实时事件和交互式拓扑更适合浏览器长连接.

## Decision(决定): 第一版不引入持久化数据库

**Rationale(理由)**: 规格假设第一版以目标进程内存 recent data(最近数据) 和实时流为准. Command audit(命令审计) 进入 EventJournal(事件日志缓冲区) 和实时流, 不落库. 这保持功能边界小, 也避免处理数据保留策略.

**Alternatives considered(备选方案)**: SQLite(嵌入式数据库) 和 PostgreSQL(关系数据库) 被拒绝, 因为规格没有持久审计保留需求, 且第一版目标是实时可观测和可控制.

## Decision(决定): 旧协议别名和历史控制命令别名必须显式拒绝

**Rationale(理由)**: 宪章禁止 compatibility export(兼容导出), 规格也禁止旧协议别名和历史控制命令别名. 因此 IPC(进程间通信) 和 `wss://` message(消息) 解析必须只接受本功能契约列出的 method(方法), message type(消息类型) 和 command(命令). 未知字段可以按 serde(序列化库) 的兼容读取策略保留, 但未知 method(方法), message type(消息类型), command(命令), 旧别名和历史别名必须返回结构化拒绝错误.

**Alternatives considered(备选方案)**: 自动把旧名称映射到新命令被拒绝, 因为它会形成隐式 compatibility layer(兼容层). 静默忽略未知命令被拒绝, 因为操作者需要明确诊断.

## Decision(决定): 安全负向路径必须作为第一版测试边界

**Rationale(理由)**: `ws://` 完整控制拒绝, 目标进程 IPC(进程间通信) 外网绕过拒绝和 trusted proxy(可信代理) 伪造身份拒绝都属于远程控制平面的基线安全边界. 这些行为必须进入控制安全测试, 而不是只写在部署说明中.

**Alternatives considered(备选方案)**: 只依赖文档说明被拒绝, 因为它无法证明未认证客户端不会触发 IPC(进程间通信) 连接或控制命令转发.
