# Tasks(任务): 监督任务可视化界面

**Input(输入)**: 设计文档来自 `/specs/003-supervisor-dashboard/`
**Prerequisites(前置文档)**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/`, `quickstart.md`

**Tests(测试)**: 本功能改变 IPC(进程间通信), relay(中继), 远程会话, 监督控制和 dashboard client(看板客户端) 行为. 每个用户故事都必须先增加外部测试, 再实现生产代码.

**Organization(组织方式)**: 任务按 User Story(用户故事) 分组. 当前 `rust-supervisor` 仓库只实现目标侧 IPC(进程间通信) 和共享契约. relay(中继) 必须在 `/Users/0x00/Documents/rust-supervisor-relay` 实现. dashboard client(看板客户端) 必须在 `/Users/0x00/Documents/rust-supervisor-ui` 实现.

## Phase 1(阶段一): Setup(共享基础设施)

**Purpose(目的)**: 建立三个目录的最小工程边界, 防止 relay(中继) 或 UI(用户界面) 实现落入当前仓库.

- [X] T001 在 `Cargo.toml` 中为 FR-001, FR-002, FR-007, FR-008, FR-011 和 FR-012 增加目标侧 IPC(进程间通信) 必需依赖, 并只在事件流需要时增加 `tokio-stream`.
- [X] T002 在 `src/dashboard/mod.rs` 中创建目标侧 dashboard(看板) IPC(进程间通信) 模块文档和最小 public API(公开接口), 并保持无 compatibility export(兼容导出).
- [X] T003 [P] 在 `/Users/0x00/Documents/rust-supervisor-relay/Cargo.toml` 和 `/Users/0x00/Documents/rust-supervisor-relay/src/main.rs` 中创建 relay(中继) 工程入口和依赖边界.
- [X] T004 [P] 在 `/Users/0x00/Documents/rust-supervisor-ui/package.json` 和 `/Users/0x00/Documents/rust-supervisor-ui/index.html` 中为 FR-027 创建 Vite(前端构建工具), Vue(网页界面框架), TypeScript(类型脚本语言), shadcn-vue(组件库), Tailwind(样式框架), Vue Flow(流程图组件), Vitest(前端测试工具), Playwright(浏览器测试工具) 和 dashboard client(看板客户端) 根挂载入口.
- [X] T005 [P] 在 `/Users/0x00/Documents/rust-supervisor-ui/tsconfig.json` 和 `/Users/0x00/Documents/rust-supervisor-ui/components.json` 中配置 TypeScript(类型脚本语言), shadcn-vue(组件库) alias(别名) 和组件目录边界.
- [X] T006 [P] 在 `/Users/0x00/Documents/rust-supervisor-ui/vite.config.ts`, `/Users/0x00/Documents/rust-supervisor-ui/tailwind.config.ts`, `/Users/0x00/Documents/rust-supervisor-ui/postcss.config.js` 和 `/Users/0x00/Documents/rust-supervisor-ui/src/assets/main.css` 中配置 dashboard client(看板客户端) 开发入口, Tailwind(样式框架) 和 shadcn-vue(组件库) 样式入口.
- [X] T007 [P] 在 `/Users/0x00/Documents/rust-supervisor-ui/playwright.config.ts` 中配置 browser test(浏览器测试) 服务器和 `wss://` 测试环境.

---

## Phase 2(阶段二): Foundational(阻塞前置基础)

**Purpose(目的)**: 建立配置, 协议, 领域模型和诊断边界. 本阶段完成前, 任何用户故事实现都不能开始.

- [X] T008 [P] 在 `tests/dashboard_config_test.rs` 中添加 FR-001, FR-023, FR-024 和 FR-025 的配置与目录边界测试, 覆盖 IPC path(进程间通信路径) 外部化, dynamic registration(动态注册) 配置, 当前仓库无 relay binary(中继二进制入口) 和当前仓库无同仓前端目录.
- [X] T009 [P] 在 `tests/dashboard_protocol_shape_test.rs` 中添加 FR-008 到 FR-012, FR-015 到 FR-018 和 FR-022 的 JSON(数据交换格式) shape(形状) 契约测试, 覆盖 snapshot(快照), event(事件), log(日志), command request(命令请求), command result(命令结果), error(错误), audit event(审计事件) 和旧协议别名拒绝.
- [X] T010 [P] 在 `src/dashboard/error.rs` 中定义目标侧结构化 DashboardError(看板错误) 和错误 code(代码), stage(阶段), target id(目标标识), retryable(可重试) 字段.
- [X] T011 [P] 在 `src/dashboard/model.rs` 中定义 TargetProcessConfig(目标进程配置), TargetProcessRegistration(目标进程注册), DashboardSnapshot(看板快照), SupervisorTopology(监督拓扑), SupervisorNode(监督节点), SupervisorEdge(监督边), EventRecord(事件记录), LogRecord(日志记录), ControlCommandRequest(控制命令请求), ControlCommandResult(控制命令结果) 和 AuditEvent(审计事件) 共享模型.
- [X] T012 [P] 在 `src/dashboard/protocol.rs` 中定义目标侧 IPC(进程间通信) request(请求), response(响应), server push(服务端主动推送) 和拒绝旧协议别名的解析规则.
- [X] T013 [P] 在 `src/dashboard/config.rs` 中定义目标进程 IPC(进程间通信) 配置模型和 path(路径), permissions(权限), bind mode(绑定模式) 校验.
- [X] T014 [P] 在 `src/dashboard/diagnostics.rs` 中定义目标侧 IPC(进程间通信), sequence(序号), command(命令) 和 dropped count(丢弃数量) 的 tracing(结构化追踪) 字段.
- [X] T015 在 `src/config/configurable.rs` 中为 FR-001 增加 optional(可选) `ipc` 配置节和 schema(模式) 字段.
- [X] T016 在 `src/config/state.rs` 中为 FR-001 增加 IPC path(进程间通信路径), target id(目标标识), permissions(权限), bind mode(绑定模式), registration(注册) 入口和 authorization scope(授权范围) 的语义校验.
- [X] T017 在 `src/lib.rs` 中加入 `pub mod dashboard;` 并保持无 compatibility export(兼容导出).
- [X] T018 在 `examples/config/supervisor.yaml` 中增加目标进程 IPC(进程间通信) 配置示例.
- [X] T019 [P] 在 `/Users/0x00/Documents/rust-supervisor-relay/tests/relay_config_test.rs` 中添加 FR-004, FR-005, FR-013, FR-014, FR-023, FR-026 和 SC-009 的 relay(中继) 配置与注册测试, 覆盖 registration(注册) 入口, 重复 target id(目标标识), 重复 IPC path(进程间通信路径), 无效租约和固定目录边界.
- [X] T020 [P] 在 `/Users/0x00/Documents/rust-supervisor-relay/tests/relay_session_contract_test.rs` 中添加 FR-006, FR-007, FR-013, FR-014 和 SC-005 的 control session(控制会话) 建立顺序测试, 验证 session(会话) 建立前不触发 IPC(进程间通信) 连接, 绑定或事件日志 subscription(订阅).
- [X] T021 [P] 在 `/Users/0x00/Documents/rust-supervisor-relay/src/config.rs` 中定义 DashboardRelayConfig(看板中继配置), TLSConfig(传输层安全协议配置), TrustedProxyConfig(可信代理配置), RegistrationPolicy(注册策略) 和 authorization defaults(授权默认规则) 校验.
- [X] T022 [P] 在 `/Users/0x00/Documents/rust-supervisor-relay/src/error.rs` 和 `/Users/0x00/Documents/rust-supervisor-relay/src/diagnostics.rs` 中定义 relay(中继) 结构化错误, tracing(结构化追踪) 字段和可观察失败分类.
- [X] T023 [P] 在 `/Users/0x00/Documents/rust-supervisor-relay/src/registry.rs` 中实现 TargetProcessRegistry(目标进程注册表), active registration(活动注册), 租约续期, 多连接状态和 partial availability(部分可用) 汇总.
- [X] T024 [P] 在 `/Users/0x00/Documents/rust-supervisor-relay/src/registration.rs` 和 `/Users/0x00/Documents/rust-supervisor-relay/src/ipc_client.rs` 中建立 relay(中继) registration(注册) 接收, 已注册目标 IPC(进程间通信) 连接, handshake(握手) 和基础请求响应边界.
- [X] T025 [P] 在 `/Users/0x00/Documents/rust-supervisor-relay/src/auth.rs` 中实现 mTLS(双向传输层安全协议认证) client certificate(客户端证书) 解析, RemoteIdentity(远程身份) 派生, trusted proxy(可信代理) 校验和授权范围判断.
- [X] T026 [P] 在 `/Users/0x00/Documents/rust-supervisor-ui/src/types/protocol.ts` 中定义和 `contracts/wss-session.md` 对齐的 TypeScript(类型脚本语言) 消息类型, 并确保 Vue(网页界面框架) 状态层可以直接消费.

**Checkpoint(检查点)**: 三个目录的配置和协议模型已经可测试, 用户故事实现可以开始.

---

## Phase 3(阶段三): User Story 1(用户故事一) - 远程查看监督树和状态 (Priority(优先级): P1) MVP(最小可用产品)

**Goal(目标)**: 已认证操作者通过 dashboard client(看板客户端) 看到一个或多个目标进程的 target process list(目标进程列表), connection state(连接状态), snapshot(快照), supervisor topology(监督拓扑) 和 runtime state(运行时状态).

**Independent Test(独立测试)**: 启动两个使用不同 IPC path(进程间通信路径) 的目标进程, 并让它们完成 dynamic registration(动态注册). 通过 relay(中继) 暴露的 `wss://` 打开 dashboard client(看板客户端), 验证每个已注册目标进程显示 root supervisor(根监督器), 所有 child task(子任务), 依赖关系, 当前状态和 generated time(生成时间).

### Tests for User Story 1(用户故事一的测试)

- [X] T027 [P] [US1] 在 `tests/dashboard_snapshot_test.rs` 中添加 FR-002, FR-008, FR-009, FR-010, SC-001 和 SC-002 的 snapshot(快照) 集成测试.
- [X] T028 [P] [US1] 在 `/Users/0x00/Documents/rust-supervisor-relay/tests/relay_session_contract_test.rs` 中添加 active registration(活动注册) 形成的 target process list(目标进程列表), authorization scope(授权范围), snapshot(快照) 首包和授权后 IPC(进程间通信) 绑定测试.
- [X] T029 [P] [US1] 在 `/Users/0x00/Documents/rust-supervisor-ui/tests/snapshot-view.spec.ts` 中添加 dashboard client(看板客户端) 首屏渲染测试, 覆盖 target list(目标列表), topology canvas(拓扑画布), node detail(节点详情) 和 unavailable(不可用) 状态.

### Implementation for User Story 1(用户故事一的实现)

- [X] T030 [P] [US1] 在 `src/dashboard/snapshot.rs` 中实现从 SupervisorHandle(监督器句柄), SupervisorTree(监督树), SupervisorState(监督器状态) 和 EventJournal(事件日志缓冲区) 构建 DashboardSnapshot(看板快照).
- [X] T031 [US1] 在 `src/dashboard/ipc_server.rs` 和 `src/dashboard/registration.rs` 中实现目标进程 Unix domain socket(Unix 域套接字) listener(监听器), dynamic registration(动态注册) payload(载荷), `hello` 方法和 `snapshot` 方法.
- [X] T032 [P] [US1] 在 `/Users/0x00/Documents/rust-supervisor-relay/src/ipc_client.rs` 中实现 relay(中继) 到目标进程 IPC(进程间通信) 的 snapshot(快照) 读取.
- [X] T033 [P] [US1] 在 `/Users/0x00/Documents/rust-supervisor-relay/src/registry.rs` 中实现 registered(已注册), connected(已连接), reconnecting(重连中), unavailable(不可用), expired(已过期) 状态汇总和可见目标过滤.
- [X] T034 [US1] 在 `/Users/0x00/Documents/rust-supervisor-relay/src/session.rs` 中实现 `wss://` session(会话) 建立, active registration(活动注册) target process list(目标进程列表) 首包发送和授权后 IPC(进程间通信) 绑定.
- [X] T035 [US1] 在 `/Users/0x00/Documents/rust-supervisor-relay/src/main.rs` 中实现 relay(中继) 配置加载, registration(注册) 入口, TLS(传输层安全协议) 监听和 session(会话) 入口连接.
- [X] T036 [P] [US1] 在 `/Users/0x00/Documents/rust-supervisor-ui/src/state/snapshotStore.ts` 中实现 target process list(目标进程列表), snapshot(快照) 和 connection state(连接状态) 的状态存储.
- [X] T037 [P] [US1] 在 `/Users/0x00/Documents/rust-supervisor-ui/src/components/TargetList.vue` 中使用 shadcn-vue(组件库) 和 Tailwind(样式框架) 实现多目标进程列表和 connected(已连接), reconnecting(重连中), unavailable(不可用) 状态展示.
- [X] T038 [P] [US1] 在 `/Users/0x00/Documents/rust-supervisor-ui/src/components/TopologyCanvas.vue` 中使用 Vue Flow(流程图组件) 渲染 SupervisorTopology(监督拓扑), SupervisorNode(监督节点) 和 SupervisorEdge(监督边), 并使用 Tailwind(样式框架) 保持画布布局稳定.
- [X] T039 [P] [US1] 在 `/Users/0x00/Documents/rust-supervisor-ui/src/components/NodeDetailsPanel.vue` 中使用 shadcn-vue(组件库) 实现 lifecycle state(生命周期状态), health(健康状态), readiness(就绪状态), restart count(重启次数), last failure(最近失败), last policy decision(最近策略决定) 和 shutdown state(关闭状态) 详情.
- [X] T040 [US1] 在 `/Users/0x00/Documents/rust-supervisor-ui/src/main.ts` 和 `/Users/0x00/Documents/rust-supervisor-ui/src/App.vue` 中集成 `wss://` 连接, snapshot store(快照存储), TargetList(目标列表), TopologyCanvas(拓扑画布) 和 NodeDetailsPanel(节点详情面板).

**Checkpoint(检查点)**: 用户故事一可以作为 MVP(最小可用产品) 独立交付.

---

## Phase 4(阶段四): User Story 2(用户故事二) - 观测事件和日志流 (Priority(优先级): P2)

**Goal(目标)**: dashboard client(看板客户端) 持续显示目标进程主动推送的 supervisor event(监督器事件), log record(日志记录) 和 command audit(命令审计), 并支持过滤和 dropped count(丢弃数量) 诊断.

**Independent Test(独立测试)**: 让多个已注册目标进程产生启动, 失败, 重启, 控制命令和关闭事件, 验证注册本身不会触发事件日志推送. 已认证 dashboard session(看板会话) 建立并绑定目标后, 事件和日志经 relay(中继) 按 target process(目标进程) 分组并按 sequence(序号) 追加, 过滤器生效, IPC(进程间通信) 重连后获得新 snapshot(快照).

### Tests for User Story 2(用户故事二的测试)

- [X] T041 [P] [US2] 在 `tests/dashboard_stream_test.rs` 中添加 FR-007, FR-011, FR-012, SC-007 和 SC-008 的客户端会话触发主动事件推送, 日志关联, sequence(序号) 单调和重连 snapshot(快照) 测试.
- [X] T042 [P] [US2] 在 `/Users/0x00/Documents/rust-supervisor-relay/tests/relay_stream_test.rs` 中添加注册后不推送, session(会话) 绑定后 event(事件), log(日志), state delta(状态增量), dropped count(丢弃数量), sequence gap(序号缺口) 和 reconnect timeout(重连超时) 转发测试.
- [X] T043 [P] [US2] 在 `/Users/0x00/Documents/rust-supervisor-ui/tests/events-filter.spec.ts` 中添加 FR-020 和 FR-021 的事件日志过滤, dropped count(丢弃数量) 和诊断展示测试.

### Implementation for User Story 2(用户故事二的实现)

- [X] T044 [P] [US2] 在 `src/dashboard/events.rs` 中实现 EventJournal(事件日志缓冲区) 到 EventRecord(事件记录), LogRecord(日志记录), dropped count(丢弃数量) 和 sequence gap(序号缺口) 的转换.
- [X] T045 [US2] 在 `src/dashboard/ipc_server.rs` 中实现 `events.subscribe`, `logs.tail` 和客户端会话触发 IPC(进程间通信) subscription(订阅) 后的目标进程主动推送循环.
- [X] T046 [US2] 在 `/Users/0x00/Documents/rust-supervisor-relay/src/ipc_client.rs` 中实现 relay(中继) 只在 session(会话) 绑定目标后建立事件日志订阅, reconnect(重连) 和重连后 snapshot(快照) 刷新.
- [X] T047 [P] [US2] 在 `/Users/0x00/Documents/rust-supervisor-relay/src/relay.rs` 中实现按 target process(目标进程) 和 session(会话) 授权范围 fan out(分发) event(事件), log(日志), state delta(状态增量) 和 error(错误).
- [X] T048 [US2] 在 `/Users/0x00/Documents/rust-supervisor-relay/src/session.rs` 中实现 `wss://` server message(服务端消息) 顺序规则, dropped count(丢弃数量) 消息和 connection state(连接状态) 更新.
- [X] T049 [P] [US2] 在 `/Users/0x00/Documents/rust-supervisor-ui/src/state/eventStore.ts` 中实现事件, 日志, dropped count(丢弃数量), sequence(序号) 和 correlation id(关联标识) 状态管理.
- [X] T050 [P] [US2] 在 `/Users/0x00/Documents/rust-supervisor-ui/src/components/EventLogPanel.vue` 中使用 shadcn-vue(组件库) 和 Tailwind(样式框架) 实现事件日志列表, command audit(命令审计) 记录和 dropped count(丢弃数量) 展示.
- [X] T051 [P] [US2] 在 `/Users/0x00/Documents/rust-supervisor-ui/src/components/FilterBar.vue` 中使用 shadcn-vue(组件库) 实现 target process identity(目标进程身份), child task(子任务), lifecycle state(生命周期状态), event type(事件类型), severity(严重程度), sequence(序号) 和 correlation id(关联标识) 过滤控件.
- [X] T052 [US2] 在 `/Users/0x00/Documents/rust-supervisor-ui/src/App.vue` 中集成 EventLogPanel(事件日志面板), FilterBar(过滤器栏) 和流式更新.

**Checkpoint(检查点)**: 用户故事二可以独立展示实时事件, 日志和过滤诊断.

---

## Phase 5(阶段五): User Story 3(用户故事三) - 安全执行完整控制命令 (Priority(优先级): P3)

**Goal(目标)**: 已授权操作者在 dashboard client(看板客户端) 中执行全部控制命令, 每个命令绑定身份, 目标和 reason(原因), 并产生 audit event(审计事件).

**Independent Test(独立测试)**: 使用已授权远程身份先建立 control session(控制会话), 再对目标 child task(子任务) 执行每一种控制命令. 验证未认证, 未授权或会话未建立时命令不触发 IPC(进程间通信), 命令结果回到当前连接, 状态更新, audit event(审计事件) 完整.

### Tests for User Story 3(用户故事三的测试)

- [X] T053 [P] [US3] 在 `/Users/0x00/Documents/rust-supervisor-relay/tests/relay_control_security_test.rs` 中添加 FR-003, FR-006, FR-013, FR-016, FR-019, FR-023, SC-005 和 SC-006 的 mTLS(双向传输层安全协议认证), 授权, `ws://` 完整控制拒绝, 目标进程 IPC(进程间通信) 外网不可达, trusted proxy(可信代理) 伪造身份拒绝, session gating(会话门控), requested by(请求者) 派生, 注册后无会话不得推送事件日志和 IPC(进程间通信) 禁止转发测试.
- [X] T054 [P] [US3] 在 `/Users/0x00/Documents/rust-supervisor-relay/tests/relay_command_contract_test.rs` 中添加 FR-015, FR-017, FR-018, FR-022 和 SC-004 的全部控制命令, 历史控制命令别名拒绝, 二次确认, 非空 reason(原因), command result(命令结果) 和 audit event(审计事件) 测试.
- [X] T055 [P] [US3] 在 `/Users/0x00/Documents/rust-supervisor-ui/tests/control-commands.spec.ts` 中添加危险命令二次确认, reason(原因) 必填, command result(命令结果) 和错误提示测试.

### Implementation for User Story 3(用户故事三的实现)

- [X] T056 [US3] 在 `src/dashboard/ipc_server.rs` 中实现 `command.restart_child`, `command.pause_child`, `command.resume_child`, `command.quarantine_child`, `command.remove_child`, `command.add_child` 和 `command.shutdown_tree` 到 SupervisorHandle(监督器句柄) 控制边界的映射.
- [X] T057 [P] [US3] 在 `/Users/0x00/Documents/rust-supervisor-relay/src/command.rs` 中实现 ControlCommandRequest(控制命令请求) 校验, requested by(请求者) 覆盖保护, dangerous command(危险命令) 二次确认和 reason(原因) 非空规则.
- [X] T058 [P] [US3] 在 `/Users/0x00/Documents/rust-supervisor-relay/src/audit.rs` 中实现 accepted(已接受), rejected(已拒绝) 和 completed(已完成) 命令 audit event(审计事件) 生成.
- [X] T059 [US3] 在 `/Users/0x00/Documents/rust-supervisor-relay/src/session.rs` 中实现未认证, 未授权, certificate identity(证书身份) 不可解析和 control session(控制会话) 未建立时的拒绝路径.
- [X] T060 [US3] 在 `/Users/0x00/Documents/rust-supervisor-relay/src/ipc_client.rs` 中实现 relay(中继) 命令转发, command result(命令结果) 读取, timeout(超时) 和目标不存在错误处理.
- [X] T061 [P] [US3] 在 `/Users/0x00/Documents/rust-supervisor-ui/src/components/ControlPanel.vue` 中使用 shadcn-vue(组件库) 实现 restart child(重启子任务), pause child(暂停子任务), resume child(恢复子任务), quarantine child(隔离子任务), remove child(移除子任务), add child(添加子任务) 和 shutdown tree(关闭监督树) 控件.
- [X] T062 [P] [US3] 在 `/Users/0x00/Documents/rust-supervisor-ui/src/components/ConfirmCommandDialog.vue` 中使用 shadcn-vue(组件库) 实现 shutdown tree(关闭监督树), remove child(移除子任务) 和 add child(添加子任务) 的二次确认和 reason(原因) 必填校验.
- [X] T063 [US3] 在 `/Users/0x00/Documents/rust-supervisor-ui/src/api/session.ts` 中实现 `wss://` command(命令), filter update(过滤更新), command result(命令结果) 和 error(错误) 客户端协议处理.

**Checkpoint(检查点)**: 所有用户故事都可以独立工作, 控制命令具备身份绑定和审计边界.

---

## Phase 6(阶段六): Polish & Cross-Cutting Concerns(收尾和横向事项)

**Purpose(目的)**: 覆盖性能, 文档, 格式化, 目录边界和端到端验证.

- [X] T064 [P] 在 `tests/dashboard_performance_test.rs` 中添加 SC-001, SC-002, SC-008, SC-010 和 SC-011 的 2 秒首包, 5 秒 200 child task(子任务), 10 秒断连诊断和目录边界测试.
- [X] T065 [P] 在 `/Users/0x00/Documents/rust-supervisor-relay/tests/relay_performance_test.rs` 中添加 SC-005, SC-008 和 SC-009 的 session gating(会话门控), 断连诊断和 5 个 active registration(活动注册) 测试.
- [X] T066 [P] 在 `/Users/0x00/Documents/rust-supervisor-ui/tests/dashboard-performance.spec.ts` 中添加 SC-003 和 SC-012 的 failed(失败), quarantined(隔离), restarting(重启中) child task(子任务) 定位流程测试, 以及 Vue(网页界面框架), shadcn-vue(组件库), Tailwind(样式框架) 基线检查.
- [X] T067 [P] 在 `manual/dashboard.md` 中编写当前仓库目标侧 IPC(进程间通信), 共享契约, 目录边界和验证命令说明.
- [X] T068 [P] 在 `/Users/0x00/Documents/rust-supervisor-relay/manual/dashboard-relay.md` 中编写 relay(中继), `wss://`, mTLS(双向传输层安全协议认证), trusted proxy(可信代理), 控制命令和诊断运行说明.
- [X] T069 [P] 在 `/Users/0x00/Documents/rust-supervisor-relay/examples/config/dashboard-relay.yaml` 中为 FR-004, FR-013 和 FR-026 编写 dynamic registration(动态注册), `wss://`, mTLS(双向传输层安全协议认证), trusted proxy(可信代理), allowed IPC path prefixes(允许的进程间通信路径前缀) 和 authorization defaults(授权默认规则) 示例配置.
- [X] T070 [P] 在 `/Users/0x00/Documents/rust-supervisor-ui/README.md` 中编写 dashboard client(看板客户端), Vue(网页界面框架), shadcn-vue(组件库), Tailwind(样式框架), 启动, 测试和证书使用说明.
- [X] T071 [P] 在 `README.zh.md` 中增加 dashboard(看板) 功能入口, 三目录边界, 配置文件和验证命令说明.
- [X] T072 运行 `cargo fmt` 并确认 `src/dashboard/mod.rs` 和所有 Rust(编程语言) 新文件格式化.
- [X] T073 运行 `cargo test` 并确认当前仓库 dashboard(看板) 目标侧测试通过.
- [X] T074 运行 `cargo test --manifest-path /Users/0x00/Documents/rust-supervisor-relay/Cargo.toml` 并确认 relay(中继) 测试通过.
- [X] T075 运行 `npm --prefix /Users/0x00/Documents/rust-supervisor-ui test` 并确认 Vitest(前端测试工具) 脚本通过.
- [X] T076 运行 `npm --prefix /Users/0x00/Documents/rust-supervisor-ui run test:e2e` 并确认 browser test(浏览器测试) 通过.

---

## Dependencies & Execution Order(依赖和执行顺序)

### Phase Dependencies(阶段依赖)

- **Setup(阶段一)**: 没有依赖, 可以立即开始.
- **Foundational(阶段二)**: 依赖 Setup(阶段一) 完成, 并阻塞所有用户故事.
- **User Story 1(用户故事一, P1)**: 依赖 Foundational(阶段二), 是 MVP(最小可用产品).
- **User Story 2(用户故事二, P2)**: 依赖 Foundational(阶段二). 执行顺序建议在 US1 之后, 但可以用 mock session(模拟会话) 和 mock IPC(模拟进程间通信) 独立测试.
- **User Story 3(用户故事三, P3)**: 依赖 Foundational(阶段二). 执行顺序建议在 US1 和 US2 之后, 但可以用 mock target process(模拟目标进程) 独立测试认证, 授权和命令契约.
- **Polish(阶段六)**: 依赖所有选定用户故事完成.

### User Story Dependencies(用户故事依赖)

- **US1(用户故事一)**: 提供远程可视化的最小闭环, 不依赖 US2 或 US3.
- **US2(用户故事二)**: 共享 Foundational(基础) 的协议和 session(会话), 不依赖控制命令.
- **US3(用户故事三)**: 共享 Foundational(基础) 的身份, 协议和错误模型, 不依赖日志过滤.

### Within Each User Story(每个用户故事内部)

- 先写当前仓库 `tests/`, `/Users/0x00/Documents/rust-supervisor-relay/tests/` 和 `/Users/0x00/Documents/rust-supervisor-ui/tests/` 中的测试, 并确认实现前失败.
- 先完成当前仓库目标侧 model(模型), protocol(协议) 和 IPC(进程间通信) 服务端, 再完成 relay(中继) IPC(进程间通信) 客户端和 `wss://` session(会话).
- 先完成 relay(中继) contract(契约), 再完成 dashboard client(看板客户端) 前端集成.
- 完成每个用户故事后运行该故事相关测试, 再进入下一个优先级.

### Parallel Opportunities(并行机会)

- T003 到 T007 可以并行, 因为它们修改不同目录.
- T008 到 T014 可以并行, 因为它们修改当前仓库不同测试和模块文件.
- T019 到 T026 可以并行, 因为它们修改 relay(中继) 和 UI(用户界面) 不同文件.
- US1 中 T027 到 T029 可以并行, T032 到 T039 可以并行.
- US2 中 T041 到 T043 可以并行, T044, T047, T049, T050 和 T051 可以并行.
- US3 中 T053 到 T055 可以并行, T057, T058, T061 和 T062 可以并行.
- T064 到 T071 可以并行, 因为它们修改不同测试, 配置示例和文档文件.

---

## Parallel Example(并行示例)

### User Story 1(用户故事一)

```bash
Task(任务): "T027 在 tests/dashboard_snapshot_test.rs 中添加 snapshot(快照) 集成测试"
Task(任务): "T028 在 /Users/0x00/Documents/rust-supervisor-relay/tests/relay_session_contract_test.rs 中添加 session(会话) 建立和首包测试"
Task(任务): "T029 在 /Users/0x00/Documents/rust-supervisor-ui/tests/snapshot-view.spec.ts 中添加首屏渲染测试"
```

### User Story 2(用户故事二)

```bash
Task(任务): "T041 在 tests/dashboard_stream_test.rs 中添加目标侧事件流测试"
Task(任务): "T042 在 /Users/0x00/Documents/rust-supervisor-relay/tests/relay_stream_test.rs 中添加 relay(中继) 转发测试"
Task(任务): "T043 在 /Users/0x00/Documents/rust-supervisor-ui/tests/events-filter.spec.ts 中添加过滤测试"
```

### User Story 3(用户故事三)

```bash
Task(任务): "T053 在 /Users/0x00/Documents/rust-supervisor-relay/tests/relay_control_security_test.rs 中添加控制安全测试"
Task(任务): "T054 在 /Users/0x00/Documents/rust-supervisor-relay/tests/relay_command_contract_test.rs 中添加命令契约测试"
Task(任务): "T055 在 /Users/0x00/Documents/rust-supervisor-ui/tests/control-commands.spec.ts 中添加控制命令浏览器测试"
```

---

## Implementation Strategy(实现策略)

### MVP First(先做最小可用产品)

1. 完成 Phase 1(阶段一) 和 Phase 2(阶段二).
2. 完成 Phase 3(阶段三) User Story 1(用户故事一).
3. 运行 `cargo test dashboard_snapshot_test`, `cargo test --manifest-path /Users/0x00/Documents/rust-supervisor-relay/Cargo.toml relay_session_contract_test` 和 `npm --prefix /Users/0x00/Documents/rust-supervisor-ui run test:e2e -- snapshot-view`.
4. 验证已认证操作者可以看到 target process list(目标进程列表), snapshot(快照), supervisor topology(监督拓扑) 和 runtime state(运行时状态).

### Incremental Delivery(增量交付)

1. US1(用户故事一) 交付远程查看监督树和状态.
2. US2(用户故事二) 增加实时事件, 日志, dropped count(丢弃数量) 和过滤器.
3. US3(用户故事三) 增加 mTLS(双向传输层安全协议认证) 身份绑定, 授权, 控制命令和审计.
4. Phase 6(阶段六) 完成性能, 文档和端到端验证.

### Format Validation(格式校验)

- 所有任务使用 `- [ ] T###` markdown checkbox(复选框) 格式.
- 所有用户故事阶段任务带 `[US1]`, `[US2]` 或 `[US3]` 标签.
- 所有 `[P]` 任务修改不同文件, 或者只在前置阶段完成后修改独立路径.
- 所有 relay(中继) 任务路径必须以 `/Users/0x00/Documents/rust-supervisor-relay` 开头.
- 所有 dashboard client(看板客户端) 任务路径必须以 `/Users/0x00/Documents/rust-supervisor-ui` 开头.
