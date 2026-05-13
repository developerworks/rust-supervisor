# Research(研究结论): 监督任务可视化界面

## Decision(决定): IPC(进程间通信) 使用 Unix domain socket(Unix 域套接字) 加 newline-delimited JSON(按行分隔的 JSON 数据)

**Rationale(理由)**: 目标平台是 Linux(操作系统) 和 macOS(操作系统), Unix domain socket(Unix 域套接字) 能把目标进程 IPC(进程间通信) 限制在本机文件系统权限边界内. newline-delimited JSON(按行分隔的 JSON 数据) 易于用 serde(序列化库) 建模, 易于在测试中断言, 也避免第一版引入 HTTP(超文本传输协议) 或 gRPC(远程过程调用协议) 的额外服务栈.

**Alternatives considered(备选方案)**: HTTP(超文本传输协议) over Unix socket(Unix 域套接字) 被拒绝, 因为第一版只需要请求响应和订阅流. gRPC(远程过程调用协议) 被拒绝, 因为需要额外 code generation(代码生成) 和 protobuf(协议缓冲) 工具链. TCP(传输控制协议) 本机端口被拒绝, 因为规格要求目标进程 IPC(进程间通信) 不得直接暴露到外网.

## Decision(决定): 目标进程 IPC path(进程间通信路径) 和注册配置放入公开 supervisor config(监督器配置)

**Rationale(理由)**: `rust-tokio-supervisor` 必须提供外部化 IPC path(进程间通信路径) 配置, 并且目标进程必须能把自身注册到 relay(中继). 计划在 `SupervisorConfig`(监督器配置) 中增加 optional(可选) 的 `ipc` 配置节, 包含 `enabled`, `path`, `target_id`, `bind_mode`, `permissions` 和 registration(注册) 信息. 目标进程只在 `enabled` 为 true(真) 时打开本机 IPC(进程间通信), 并在 IPC(进程间通信) 就绪后向 relay(中继) 提交 target id(目标标识), display name(显示名称), IPC path(进程间通信路径), registration lease(注册租约), heartbeat interval(心跳间隔) 和 supported commands(支持的命令).

**Alternatives considered(备选方案)**: 使用环境变量被拒绝, 因为现有配置模型已经集中在 YAML(配置文件格式) 和 schema(模式) 中. 自动生成临时 path(路径) 被拒绝, 因为目标进程注册到 relay(中继) 时必须上报明确 IPC path(进程间通信路径), 否则 dashboard(看板) 无法稳定归因.

## Decision(决定): relay(中继) 使用独立目录和 dynamic registration(动态注册)

**Rationale(理由)**: 第一版采用 dynamic registration(动态注册). relay(中继) 生产实现固定放在 `/Users/0x00/Documents/rust-supervisor-relay`, 当前 `rust-supervisor` 仓库不承载 relay server(中继服务器) 或 relay binary(中继二进制入口). `DashboardRelayConfig`(看板中继配置) 属于 relay(中继) 目录, 它包含 `listen`, `tls`, `trusted_proxy` 和 `registration`, 但不包含静态 `targets` 列表. 目标进程启动 IPC(进程间通信) 后向 relay(中继) 注册 `target_id`, `display_name`, `ipc_path`, `lease_seconds` 和 `supported_commands`. relay(中继) 只在内存 registry(注册表) 中保存 active registration(活动注册), 拒绝 owner identity(所有者身份) 不匹配的覆盖, 重复 IPC path(进程间通信路径), 无效租约和过期租约, 保证 dashboard(看板) 中连接状态和命令目标可以稳定归因.

**Alternatives considered(备选方案)**: 静态多目标配置被拒绝, 因为用户要求采用 dynamic registration(动态注册), 目标进程数量和 IPC path(进程间通信路径) 需要在运行时进入 relay(中继) registry(注册表). 外部服务发现被拒绝, 因为第一版只需要目标进程主动注册到 relay(中继), 不需要接入额外 discovery service(服务发现组件). 把 relay(中继) 放回当前 `rust-supervisor` 仓库被拒绝, 因为用户明确要求中继在单独目录实现.

## Decision(决定): 外部远程会话使用 `wss://` 加 mTLS(双向传输层安全协议认证)

**Rationale(理由)**: WebSocket(网络套接字协议) 必须和 mTLS(双向传输层安全协议认证) 通过 `wss://` 协同工作. relay(中继) 使用 `tokio-rustls` 完成 TLS(传输层安全协议) 握手和 client certificate(客户端证书) 验证, 再使用 `tokio-tungstenite` 完成 WebSocket(网络套接字协议) upgrade(升级). control session(控制会话) 建立后, relay(中继) 先发送 `server_hello`(服务端握手), 等待 `client_hello`(客户端握手), 再发送当前 active registration(活动注册) 形成的 target process list(目标进程列表), 并自动绑定全部 active target(活跃目标).

**Alternatives considered(备选方案)**: `ws://` 被拒绝, 因为规格禁止其访问完整控制能力. Bearer token(持有者令牌) 被拒绝作为第一身份来源, 因为规格要求双方身份认证. TLS(传输层安全协议) 默认由 relay(中继) 终止, 可信代理模式只作为明确配置分支.

## Decision(决定): 事件和日志由客户端会话触发后主动推送

**Rationale(理由)**: 规格要求事件和日志由目标进程主动发送, 但主动推送必须由远程客户端完成 control session(控制会话) 后触发. 目标进程完成 dynamic registration(动态注册) 后只进入可见目标列表, 不因为注册本身开始事件日志推送. 当已认证客户端建立 control session(控制会话) 并选择可见目标后, relay(中继) 才连接目标进程 IPC(进程间通信), 建立 subscription(订阅), 请求 recent event(最近事件), 然后目标进程把新的 SupervisorEvent(监督器事件), LogRecord(日志记录) 和 availability change(可用状态变化) 写入同一连接. 每个 target process(目标进程) 内 sequence(序号) 必须单调展示.

**Alternatives considered(备选方案)**: 注册后立即推送被拒绝, 因为它会让没有客户端会话的目标进程产生不必要连接和日志流. relay(中继) polling(轮询) state(状态) 被拒绝, 因为它无法满足主动推送和顺序要求. 只推送文本事件被拒绝, 因为 dashboard(看板) 需要按 target identity(目标身份), child task(子任务), severity(严重程度), sequence(序号) 和 correlation id(关联标识) 过滤.

## Decision(决定): state(状态) 使用单一 typed model(类型化模型)

**Rationale(理由)**: DashboardState(看板状态) 聚合 target process identity(目标进程身份), SupervisorTopology(监督拓扑), RuntimeState(运行时状态), recent events(最近事件), recent logs(最近日志), dropped count(丢弃数量), config version(配置版本) 和 generated time(生成时间). 该模型同时用于 IPC(进程间通信) response(响应), `wss://` message(消息) 和 dashboard(看板) 前端 TypeScript(类型脚本语言) 类型生成或手工同步, 并由 Vue(网页界面框架) 状态层消费.

**Alternatives considered(备选方案)**: 分散多个接口被拒绝, 因为重连必须获得新的完整 state(状态). 让前端自行拼接监督树被拒绝, 因为操作者不能手工拼接状态.

## Decision(决定): dashboard client(看板客户端) 放在独立 UI(用户界面) 目录

**Rationale(理由)**: 规格要求远程 dashboard(看板) 可视化监督树, 状态, 事件, 日志和控制命令. dashboard client(看板客户端) 生产实现固定放在 `/Users/0x00/Documents/rust-supervisor-ui`, 当前 `rust-supervisor` 仓库不新增同仓 `dashboard/` 前端目录. 用户明确要求前端使用 shadcn-vue(组件库) 和 Tailwind(样式框架), 因此前端基线为 `Vite(前端构建工具)`, `Vue(网页界面框架)`, `TypeScript(类型脚本语言)`, `shadcn-vue(组件库)`, `Tailwind(样式框架)` 和 `Vue Flow(流程图组件)`. shadcn-vue(组件库) 负责常规控制, 表单, 对话框, 提示和布局组件, Tailwind(样式框架) 负责设计 token(设计令牌) 和布局样式, Vue Flow(流程图组件) 负责监督拓扑画布. 前端只依赖 `wss://` contract(契约), 不直接访问 IPC(进程间通信).

**Alternatives considered(备选方案)**: 纯命令行 dashboard(看板) 被拒绝, 因为用户故事要求远程可视化界面. React(网页界面库) 和 React Flow(流程图组件) 被拒绝, 因为用户明确要求 shadcn-vue(组件库) 和 Tailwind(样式框架). 服务端渲染被拒绝, 因为实时事件和交互式拓扑更适合浏览器长连接. 把前端放在当前仓库 `dashboard/` 目录被拒绝, 因为用户明确要求前端在单独目录实现.

## Decision(决定): 第一版不引入持久化数据库

**Rationale(理由)**: 规格假设第一版以目标进程内存 recent data(最近数据) 和实时流为准. Command audit(命令审计) 进入 EventJournal(事件日志缓冲区) 和实时流, 不落库. 这保持功能边界小, 也避免处理数据保留策略.

**Alternatives considered(备选方案)**: SQLite(嵌入式数据库) 和 PostgreSQL(关系数据库) 被拒绝, 因为规格没有持久审计保留需求, 且第一版目标是实时可观测和可控制.

## Decision(决定): 旧协议别名和历史控制命令别名必须显式拒绝

**Rationale(理由)**: 宪章禁止 compatibility export(兼容导出), 规格也禁止旧协议别名和历史控制命令别名. 因此 IPC(进程间通信) 和 `wss://` message(消息) 解析必须只接受本功能契约列出的 method(方法), message type(消息类型) 和 command(命令). 未知字段可以按 serde(序列化库) 的兼容读取策略保留, 但未知 method(方法), message type(消息类型), command(命令), 旧别名和历史别名必须返回结构化拒绝错误.

**Alternatives considered(备选方案)**: 自动把旧名称映射到新命令被拒绝, 因为它会形成隐式 compatibility layer(兼容层). 静默忽略未知命令被拒绝, 因为操作者需要明确诊断.

## Decision(决定): 安全负向路径必须作为第一版测试边界

**Rationale(理由)**: `ws://` 完整控制拒绝, 目标进程 IPC(进程间通信) 外网绕过拒绝和 trusted proxy(可信代理) 伪造身份拒绝都属于远程控制平面的基线安全边界. 这些行为必须进入控制安全测试, 而不是只写在部署说明中.

**Alternatives considered(备选方案)**: 只依赖文档说明被拒绝, 因为它无法证明未认证客户端不会触发 IPC(进程间通信) 连接或控制命令转发.
