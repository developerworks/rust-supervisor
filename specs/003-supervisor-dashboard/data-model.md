# Data Model(数据模型): 监督任务可视化界面

## Workspace Ownership(工作区所有权)

- `/Users/0x00/Documents/rust-supervisor`: 拥有目标进程 IPC(进程间通信) 配置, 目标侧 IPC(进程间通信) 服务端, snapshot(快照), EventRecord(事件记录), LogRecord(日志记录), ControlCommandRequest(控制命令请求) 和 ControlCommandResult(控制命令结果) 的共享契约.
- `/Users/0x00/Documents/rust-supervisor-relay`: 拥有 DashboardSession(看板会话), RemoteIdentity(远程身份), TargetProcessRegistration(目标进程注册), TargetProcessRegistry(目标进程注册表), TargetProcessConnection(目标进程连接), relay(中继) 配置, audit event(审计事件) 和 `wss://` 分发状态.
- `/Users/0x00/Documents/rust-supervisor-ui`: 拥有 dashboard client(看板客户端) 的 Vue(网页界面框架) 界面状态, shadcn-vue(组件库) 组件展示, Tailwind(样式框架) 样式令牌和浏览器交互模型. 它只消费 relay(中继) 通过 `wss://` 暴露的契约, 不直接访问目标进程 IPC(进程间通信).

## DashboardSession(看板会话)

**Purpose(目的)**: 表达已认证远程连接和它能访问的目标范围.

**Fields(字段)**:
- `session_id`: UUID(通用唯一标识), 由 relay(中继) 生成.
- `remote_identity`: RemoteIdentity(远程身份), 来自 mTLS(双向传输层安全协议认证) 或可信代理身份.
- `authorization_scopes`: 授权范围集合, 决定可见 target process(目标进程) 和可执行命令.
- `connection_state`: `handshaking`, `established`, `closing`, `closed`.
- `control_state`: `not_established`, `established`, `revoked`.
- `last_sync`: 每个 target process(目标进程) 的最近 sequence(序号) 和 snapshot generation(快照代次).
- `created_at` 和 `last_seen_at`: 会话时间.

**Validation(校验)**: 没有有效 RemoteIdentity(远程身份) 时不得进入 `established`. `control_state` 未建立时不得触发 IPC(进程间通信) 连接或命令转发.

## RemoteIdentity(远程身份)

**Purpose(目的)**: 把 mTLS(双向传输层安全协议认证) 证书或可信代理验证结果转换为授权和审计身份.

**Fields(字段)**:
- `subject`: 证书 subject(主体) 或代理传递的已验证 subject(主体).
- `issuer`: 证书 issuer(签发者).
- `serial_number`: 证书序列号.
- `principal`: relay(中继) 派生的操作者或服务身份.
- `source`: `mtls` 或 `trusted_proxy`.
- `not_before` 和 `not_after`: 证书有效期.

**Validation(校验)**: 证书缺失, 过期, 主体不可解析或来自非可信代理的身份头必须拒绝.

## TargetProcessConfig(目标进程配置)

**Purpose(目的)**: 配置目标进程打开 IPC(进程间通信) 入口, 并定义目标进程注册到 relay(中继) 时需要上报的信息.

**Fields(字段)**:
- `target_id`: 目标进程稳定标识.
- `display_name`: dashboard(看板) 显示名称.
- `ipc_path`: Unix domain socket(Unix 域套接字) path(路径).
- `enabled`: 是否打开目标进程 IPC(进程间通信).
- `permissions`: socket(套接字) 文件权限.
- `config_version`: 配置版本.
- `authorization_scope`: 远程身份需要具备的授权范围.
- `registration`: relay(中继) 注册入口, 租约时长和心跳策略.

**Validation(校验)**: 目标进程启用 IPC(进程间通信) 时 `ipc_path` 必须非空, 且不得是相对路径. `authorization_scope` 必须非空, 否则目标进程不得注册到 relay(中继).

## TargetProcessRegistration(目标进程注册)

**Purpose(目的)**: 表达目标进程在 IPC(进程间通信) 就绪后提交给 relay(中继) 的运行时注册记录.

**Fields(字段)**:
- `target_id`: 目标进程稳定标识.
- `display_name`: dashboard(看板) 显示名称.
- `ipc_path`: 目标进程已经打开的 Unix domain socket(Unix 域套接字) path(路径).
- `authorization_scope`: 远程身份需要具备的授权范围.
- `lease_seconds`: 注册租约有效时长.
- `registered_at`, `renewed_at` 和 `expires_at`: 注册和续约时间.
- `registration_state`: `pending`, `active`, `rejected`, `expired`.
- `last_rejection`: 最近结构化拒绝原因.

**Validation(校验)**: relay(中继) 必须拒绝重复 `target_id`, 重复 `ipc_path`, 非绝对 `ipc_path`, 空 `authorization_scope` 和无效 `lease_seconds`. 注册过期后, relay(中继) 必须停止把该目标作为可绑定目标展示.

## TargetProcessRegistry(目标进程注册表)

**Purpose(目的)**: relay(中继) 保存 active registration(活动注册), IPC path(进程间通信路径), 连接状态, 租约状态和授权范围.

**Fields(字段)**:
- `registrations`: target id(目标标识) 到 TargetProcessRegistration(目标进程注册) 的映射.
- `connections`: target id(目标标识) 到 TargetProcessConnection(目标进程连接) 的映射.
- `registration_policy_version`: relay(中继) 注册策略版本.

**Relationships(关系)**: 一个 registry(注册表) 拥有多个 active registration(活动注册), 每个 active registration(活动注册) 最多拥有一个 active connection(活动连接). 注册本身不建立事件日志流, 只有已认证客户端会话触发绑定后才允许连接和订阅.

## TargetProcessConnection(目标进程连接)

**Purpose(目的)**: 表达 relay(中继) 与一个目标进程 IPC(进程间通信) 的生命周期.

**Fields(字段)**:
- `target_id`: 目标进程标识.
- `ipc_path`: IPC path(进程间通信路径).
- `state`: `registered`, `disconnected`, `connecting`, `connected`, `reconnecting`, `unavailable`, `expired`.
- `last_error`: 最近结构化错误.
- `last_snapshot_generation`: 最近 snapshot(快照) 代次.
- `last_sequence`: 最近接收 sequence(序号).
- `connected_at` 和 `updated_at`: 连接时间.

**State transitions(状态转换)**:
- `registered -> disconnected`: 目标进程完成 dynamic registration(动态注册), 但没有已认证客户端会话绑定.
- `disconnected -> connecting`: 已授权 control session(控制会话) 触发连接和 subscription(订阅).
- `connecting -> connected`: IPC(进程间通信) 握手成功.
- `connecting -> unavailable`: path(路径) 不存在, 权限不足或握手失败.
- `connected -> reconnecting`: 读写失败或目标进程关闭连接.
- `reconnecting -> connected`: 重连成功并接收新 snapshot(快照).
- `reconnecting -> unavailable`: 重连预算耗尽或超过 10 秒诊断阈值.
- `registered -> expired`: 注册租约过期或心跳中断.
- `connected -> expired`: 注册租约过期, 连接必须降级并停止继续推送.

## DashboardSnapshot(看板快照)

**Purpose(目的)**: 打开 dashboard(看板), 重连或命令后返回完整当前视图.

**Fields(字段)**:
- `target`: target process identity(目标进程身份).
- `topology`: SupervisorTopology(监督拓扑).
- `runtime_state`: RuntimeState(运行时状态) 集合.
- `recent_events`: EventRecord(事件记录) 列表.
- `recent_logs`: LogRecord(日志记录) 列表.
- `dropped_event_count`: 丢弃事件数量.
- `dropped_log_count`: 丢弃日志数量.
- `config_version`: 配置版本.
- `generated_at`: 生成时间.
- `snapshot_generation`: 单调增长快照代次.

**Validation(校验)**: 每个 child task(子任务) 必须在 topology(监督拓扑) 和 runtime_state(运行时状态) 中可关联. `generated_at` 必须存在. `snapshot_generation` 在同一目标进程内单调增长.

## SupervisorTopology(监督拓扑)

**Purpose(目的)**: 表达 root supervisor(根监督器), child task(子任务), 父子关系和依赖关系.

**Fields(字段)**:
- `root`: SupervisorNode(监督节点).
- `nodes`: SupervisorNode(监督节点) 列表.
- `edges`: SupervisorEdge(监督边) 列表.
- `declaration_order`: 节点声明顺序.

**Validation(校验)**: 必须有一个 root supervisor(根监督器). child path(子任务路径) 必须唯一. dependency edge(依赖边) 的两端必须存在.

## SupervisorNode(监督节点)

**Purpose(目的)**: dashboard(看板) 中显示的 root supervisor(根监督器) 或 child task(子任务).

**Fields(字段)**:
- `node_id`: 节点标识.
- `child_id`: 子任务标识, root supervisor(根监督器) 可为空.
- `path`: child path(子任务路径).
- `name`: 显示名称.
- `kind`: `root_supervisor` 或 `child_task`.
- `tags`: 标签集合.
- `criticality`: `critical`, `standard`, `best_effort`.
- `state_summary`: 当前状态摘要.
- `diagnostics`: 关键诊断字段.

**Validation(校验)**: `path` 在同一 target process(目标进程) 内必须唯一.

## SupervisorEdge(监督边)

**Purpose(目的)**: 表达父子关系和 child task(子任务) 依赖关系.

**Fields(字段)**:
- `edge_id`: 边标识.
- `source_path`: 来源 path(路径).
- `target_path`: 目标 path(路径).
- `kind`: `parent_child` 或 `dependency`.
- `order`: 声明顺序或依赖顺序.

## RuntimeState(运行时状态)

**Purpose(目的)**: 表达每个 child task(子任务) 的生命周期和监督诊断.

**Fields(字段)**:
- `child_path`: child path(子任务路径).
- `lifecycle_state`: `starting`, `running`, `paused`, `quarantined`, `failed`, `restarting`, `stopping`, `stopped`, `completed`.
- `health`: `unknown`, `healthy`, `stale`, `unhealthy`.
- `readiness`: `unknown`, `ready`, `not_ready`.
- `generation`: 代次.
- `attempt`: 尝试次数.
- `restart_count`: 重启次数.
- `last_failure`: 最近失败.
- `last_policy_decision`: 最近策略决定.
- `shutdown_state`: 关闭状态.

## EventRecord(事件记录)

**Purpose(目的)**: 目标进程主动发送的监督事实.

**Fields(字段)**:
- `target_id`: 目标进程标识.
- `sequence`: 目标进程内单调序号.
- `correlation_id`: 关联标识.
- `event_type`: 事件类型.
- `severity`: 严重程度.
- `target_path`: 目标 path(路径).
- `child_id`: 子任务标识.
- `occurred_at`: 发生时间.
- `config_version`: 配置版本.
- `payload`: 类型化载荷.

**Validation(校验)**: 同一 TargetProcessConnection(目标进程连接) 内 `sequence` 必须单调递增. `target_id` 必须匹配连接目标.

## LogRecord(日志记录)

**Purpose(目的)**: 与 EventRecord(事件记录) 可关联的日志事实.

**Fields(字段)**:
- `target_id`: 目标进程标识.
- `sequence`: 可选日志序号.
- `correlation_id`: 可选关联标识.
- `severity`: 严重程度.
- `message`: 日志消息.
- `fields`: 结构化字段.
- `occurred_at`: 发生时间.

**Validation(校验)**: 必须能通过 `target_id` 加 `sequence` 或 `correlation_id` 与事件流关联.

## ControlCommandRequest(控制命令请求)

**Purpose(目的)**: 远程操作者发起控制命令的系统内表示.

**Fields(字段)**:
- `command_id`: 命令标识.
- `target_id`: 目标进程标识.
- `command`: `restart_child`, `pause_child`, `resume_child`, `quarantine_child`, `remove_child`, `add_child`, `shutdown_tree`.
- `target`: 命令目标.
- `reason`: 原因.
- `requested_by`: 从 RemoteIdentity(远程身份) 派生.
- `confirmed`: 是否完成二次确认.
- `requested_at`: 请求时间.

**Validation(校验)**: `reason` 必须非空. 客户端不得提供或覆盖 `requested_by`. `shutdown_tree`, `remove_child` 和 `add_child` 必须 `confirmed=true`.

## ControlCommandResult(控制命令结果)

**Purpose(目的)**: 目标进程执行控制命令后的结果.

**Fields(字段)**:
- `command_id`: 命令标识.
- `target_id`: 目标进程标识.
- `accepted`: 是否接受.
- `status`: `accepted`, `rejected`, `completed`, `failed`.
- `error`: 失败时的结构化错误.
- `state_delta`: 可选状态增量.
- `completed_at`: 完成时间.

## AuditEvent(审计事件)

**Purpose(目的)**: 记录每个控制命令被接受, 拒绝和完成的事实.

**Fields(字段)**:
- `audit_id`: 审计标识.
- `identity`: RemoteIdentity(远程身份) 摘要.
- `target_id`: 目标进程标识.
- `command_id`: 命令标识.
- `command`: 命令名称.
- `target`: 命令目标.
- `reason`: 原因.
- `result`: 命令结果摘要.
- `occurred_at`: 审计时间.

**Validation(校验)**: 每个 accepted(已接受), rejected(已拒绝) 和 completed(已完成) 命令都必须有 audit event(审计事件).
