# Tasks(任务): 监督任务可视化界面

**Input(输入)**: 设计文档来自 `/specs/003-supervisor-dashboard/`
**Prerequisites(前置文档)**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/`, `quickstart.md`

**Tests(测试)**: 本功能改变 IPC(进程间通信), 远程会话, 监督控制和 dashboard(看板) 行为. 每个用户故事都必须先增加外部测试, 再实现生产代码.

**Organization(组织方式)**: 任务按 User Story(用户故事) 分组, 保证每个故事都能独立实现和独立验证.

## Phase 1(阶段一): Setup(共享基础设施)

**Purpose(目的)**: 增加依赖, 建立 Rust(编程语言) dashboard(看板) 模块边界和前端工程入口.

- [ ] T001 在 `Cargo.toml` 中为 FR-002, FR-013 和 FR-014 增加 `tokio-stream`, `tokio-tungstenite`, `tokio-rustls`, `rustls-pemfile`, `x509-parser`, `futures-util` 和测试依赖.
- [ ] T002 在 `src/dashboard/mod.rs` 中创建 dashboard(看板) 顶层模块文档和最小 public API(公开接口).
- [ ] T003 [P] 在 `dashboard/package.json` 中创建 Vite(前端构建工具), React(网页界面库), TypeScript(类型脚本语言), React Flow(流程图组件), Vitest(前端测试工具) 和 Playwright(浏览器测试工具) 脚本, 并在 `dashboard/index.html` 中创建 dashboard(看板) 根挂载入口.
- [ ] T004 [P] 在 `dashboard/tsconfig.json` 中配置 TypeScript(类型脚本语言) 编译边界.
- [ ] T005 [P] 在 `dashboard/vite.config.ts` 中配置 dashboard(看板) 开发和测试入口.
- [ ] T006 [P] 在 `dashboard/playwright.config.ts` 中配置 browser test(浏览器测试) 服务器和 `wss://` 测试环境.

---

## Phase 2(阶段二): Foundational(阻塞前置基础)

**Purpose(目的)**: 建立配置, 协议, 领域模型和诊断边界. 本阶段完成前, 任何用户故事实现都不能开始.

- [ ] T007 [P] 在 `tests/dashboard_config_test.rs` 中添加 FR-001, FR-004, FR-005 和 SC-009 的配置校验测试, 覆盖 IPC path(进程间通信路径) 外部化, 多目标配置, 重复 target id(目标标识) 和重复 IPC path(进程间通信路径).
- [ ] T008 [P] 在 `tests/dashboard_protocol_shape_test.rs` 中添加 FR-008 到 FR-018 和 FR-022 的 JSON(数据交换格式) shape(形状) 契约测试, 覆盖 snapshot(快照), event(事件), log(日志), command request(命令请求), command result(命令结果), error(错误), audit event(审计事件) 和旧协议别名拒绝.
- [ ] T009 [P] 在 `src/dashboard/error.rs` 中定义结构化 DashboardError(看板错误) 和错误 code(代码), stage(阶段), target id(目标标识), retryable(可重试) 字段.
- [ ] T010 [P] 在 `src/dashboard/model.rs` 中定义 DashboardSession(看板会话), TargetProcessConfig(目标进程配置), TargetProcessConnection(目标进程连接), DashboardSnapshot(看板快照), SupervisorTopology(监督拓扑), SupervisorNode(监督节点), SupervisorEdge(监督边), EventRecord(事件记录), LogRecord(日志记录), ControlCommandRequest(控制命令请求), ControlCommandResult(控制命令结果), RemoteIdentity(远程身份) 和 AuditEvent(审计事件).
- [ ] T011 [P] 在 `src/dashboard/protocol.rs` 中定义 IPC(进程间通信) request(请求), response(响应), server push(服务端主动推送) 和 `wss://` message(消息) 枚举.
- [ ] T012 [P] 在 `src/dashboard/config.rs` 中定义 DashboardSidecarConfig(看板侧车配置), TLSConfig(传输层安全协议配置), TrustedProxyConfig(可信代理配置) 和多目标 registry(注册表) 配置校验.
- [ ] T013 [P] 在 `src/dashboard/diagnostics.rs` 中定义 IPC(进程间通信), mTLS(双向传输层安全协议认证), session(会话), sequence(序号), command(命令) 和 dropped count(丢弃数量) 的 tracing(结构化追踪) 字段.
- [ ] T014 在 `src/config/configurable.rs` 中为 FR-001 增加 optional(可选) `ipc` 配置节和 schema(模式) 字段.
- [ ] T015 在 `src/config/state.rs` 中为 FR-001 增加 IPC path(进程间通信路径), target id(目标标识), permissions(权限) 和 bind mode(绑定模式) 的语义校验.
- [ ] T016 在 `src/lib.rs` 中加入 `pub mod dashboard;` 并保持无 compatibility export(兼容导出).
- [ ] T017 在 `examples/config/supervisor.yaml` 中增加目标进程 IPC(进程间通信) 配置示例.
- [ ] T018 在 `examples/config/dashboard-sidecar.yaml` 中增加 sidecar(侧车进程) 多 IPC path(进程间通信路径) 配置示例.

**Checkpoint(检查点)**: 配置和协议模型已经可测试, 用户故事实现可以开始.

---

## Phase 3(阶段三): User Story 1(用户故事一) - 远程查看监督树和状态 (Priority(优先级): P1) MVP(最小可用产品)

**Goal(目标)**: 已认证操作者通过 dashboard(看板) 看到一个或多个目标进程的 target process list(目标进程列表), connection state(连接状态), snapshot(快照), supervisor topology(监督拓扑) 和 runtime state(运行时状态).

**Independent Test(独立测试)**: 启动两个使用不同 IPC path(进程间通信路径) 的目标进程, 通过 `wss://` 打开 dashboard(看板), 验证每个目标进程显示 root supervisor(根监督器), 所有 child task(子任务), 依赖关系, 当前状态和 generated time(生成时间).

### Tests for User Story 1(用户故事一的测试)

- [ ] T019 [P] [US1] 在 `tests/dashboard_snapshot_test.rs` 中添加 FR-002, FR-008, FR-009, FR-010, SC-001 和 SC-002 的 snapshot(快照) 集成测试.
- [ ] T020 [P] [US1] 在 `tests/dashboard_session_contract_test.rs` 中添加 FR-006, FR-013 和 FR-014 的 control session(控制会话) 建立顺序测试, 验证 session(会话) 建立前不触发 IPC(进程间通信) 连接.
- [ ] T021 [P] [US1] 在 `dashboard/tests/snapshot-view.spec.ts` 中添加 dashboard(看板) 首屏渲染测试, 覆盖 target list(目标列表), topology canvas(拓扑画布), node detail(节点详情) 和 unavailable(不可用) 状态.

### Implementation for User Story 1(用户故事一的实现)

- [ ] T022 [P] [US1] 在 `src/dashboard/snapshot.rs` 中实现从 SupervisorHandle(监督器句柄), SupervisorTree(监督树), SupervisorState(监督器状态) 和 EventJournal(事件日志缓冲区) 构建 DashboardSnapshot(看板快照).
- [ ] T023 [US1] 在 `src/dashboard/ipc_server.rs` 中实现目标进程 Unix domain socket(Unix 域套接字) listener(监听器), `hello` 方法和 `snapshot` 方法.
- [ ] T024 [P] [US1] 在 `src/dashboard/ipc_client.rs` 中实现 sidecar(侧车进程) 到目标进程 IPC(进程间通信) 的连接, handshake(握手) 和 snapshot(快照) 读取.
- [ ] T025 [P] [US1] 在 `src/dashboard/registry.rs` 中实现 TargetProcessRegistry(目标进程注册表), 多连接状态和 partial availability(部分可用) 汇总.
- [ ] T026 [US1] 在 `src/dashboard/session.rs` 中实现 `wss://` session(会话) 建立, target process list(目标进程列表) 首包发送和授权后 IPC(进程间通信) 绑定.
- [ ] T027 [US1] 在 `src/bin/rust-supervisor-dashboard-sidecar.rs` 中实现 sidecar(侧车进程) 配置加载, TLS(传输层安全协议) 监听和 session(会话) 入口连接.
- [ ] T028 [P] [US1] 在 `dashboard/src/types/protocol.ts` 中定义和 `contracts/wss-session.md` 对齐的 TypeScript(类型脚本语言) 消息类型.
- [ ] T029 [P] [US1] 在 `dashboard/src/state/snapshotStore.ts` 中实现 target process list(目标进程列表), snapshot(快照) 和 connection state(连接状态) 的状态存储.
- [ ] T030 [P] [US1] 在 `dashboard/src/components/TargetList.tsx` 中实现多目标进程列表和 connected(已连接), reconnecting(重连中), unavailable(不可用) 状态展示.
- [ ] T031 [P] [US1] 在 `dashboard/src/components/TopologyCanvas.tsx` 中使用 React Flow(流程图组件) 渲染 SupervisorTopology(监督拓扑), SupervisorNode(监督节点) 和 SupervisorEdge(监督边).
- [ ] T032 [P] [US1] 在 `dashboard/src/components/NodeDetailsPanel.tsx` 中实现 lifecycle state(生命周期状态), health(健康状态), readiness(就绪状态), restart count(重启次数), last failure(最近失败), last policy decision(最近策略决定) 和 shutdown state(关闭状态) 详情.
- [ ] T033 [US1] 在 `dashboard/src/main.tsx` 中集成 `wss://` 连接, snapshot store(快照存储), TargetList(目标列表), TopologyCanvas(拓扑画布) 和 NodeDetailsPanel(节点详情面板).

**Checkpoint(检查点)**: 用户故事一可以作为 MVP(最小可用产品) 独立交付.

---

## Phase 4(阶段四): User Story 2(用户故事二) - 观测事件和日志流 (Priority(优先级): P2)

**Goal(目标)**: dashboard(看板) 持续显示目标进程主动推送的 supervisor event(监督器事件), log record(日志记录) 和 command audit(命令审计), 并支持过滤和 dropped count(丢弃数量) 诊断.

**Independent Test(独立测试)**: 让多个目标进程产生启动, 失败, 重启, 控制命令和关闭事件, 验证事件和日志按 target process(目标进程) 分组并按 sequence(序号) 追加, 过滤器生效, IPC(进程间通信) 重连后获得新 snapshot(快照).

### Tests for User Story 2(用户故事二的测试)

- [ ] T034 [P] [US2] 在 `tests/dashboard_stream_test.rs` 中添加 FR-007, FR-011, FR-012, SC-007 和 SC-008 的主动事件推送, 日志关联, sequence(序号) 单调和重连 snapshot(快照) 测试.
- [ ] T035 [P] [US2] 在 `dashboard/tests/events-filter.spec.ts` 中添加 FR-020 和 FR-021 的事件日志过滤, dropped count(丢弃数量) 和诊断展示测试.

### Implementation for User Story 2(用户故事二的实现)

- [ ] T036 [P] [US2] 在 `src/dashboard/events.rs` 中实现 EventJournal(事件日志缓冲区) 到 EventRecord(事件记录), LogRecord(日志记录), dropped count(丢弃数量) 和 sequence gap(序号缺口) 的转换.
- [ ] T037 [US2] 在 `src/dashboard/ipc_server.rs` 中实现 `events.subscribe`, `logs.tail` 和 IPC(进程间通信) 建立后的目标进程主动推送循环.
- [ ] T038 [US2] 在 `src/dashboard/ipc_client.rs` 中实现 sidecar(侧车进程) 事件日志订阅, reconnect(重连) 和重连后 snapshot(快照) 刷新.
- [ ] T039 [P] [US2] 在 `src/dashboard/relay.rs` 中实现按 target process(目标进程) 和 session(会话) 授权范围 fan out(分发) event(事件), log(日志), state delta(状态增量) 和 error(错误).
- [ ] T040 [US2] 在 `src/dashboard/session.rs` 中实现 `wss://` server message(服务端消息) 顺序规则, dropped count(丢弃数量) 消息和 connection state(连接状态) 更新.
- [ ] T041 [P] [US2] 在 `dashboard/src/state/eventStore.ts` 中实现事件, 日志, dropped count(丢弃数量), sequence(序号) 和 correlation id(关联标识) 状态管理.
- [ ] T042 [P] [US2] 在 `dashboard/src/components/EventLogPanel.tsx` 中实现事件日志列表, command audit(命令审计) 记录和 dropped count(丢弃数量) 展示.
- [ ] T043 [P] [US2] 在 `dashboard/src/components/FilterBar.tsx` 中实现 target process identity(目标进程身份), child task(子任务), lifecycle state(生命周期状态), event type(事件类型), severity(严重程度), sequence(序号) 和 correlation id(关联标识) 过滤控件.
- [ ] T044 [US2] 在 `dashboard/src/main.tsx` 中集成 EventLogPanel(事件日志面板), FilterBar(过滤器栏) 和流式更新.

**Checkpoint(检查点)**: 用户故事二可以独立展示实时事件, 日志和过滤诊断.

---

## Phase 5(阶段五): User Story 3(用户故事三) - 安全执行完整控制命令 (Priority(优先级): P3)

**Goal(目标)**: 已授权操作者在 dashboard(看板) 中执行全部控制命令, 每个命令绑定身份, 目标和 reason(原因), 并产生 audit event(审计事件).

**Independent Test(独立测试)**: 使用已授权远程身份先建立 control session(控制会话), 再对目标 child task(子任务) 执行每一种控制命令. 验证未认证, 未授权或会话未建立时命令不触发 IPC(进程间通信), 命令结果回到当前连接, 状态更新, audit event(审计事件) 完整.

### Tests for User Story 3(用户故事三的测试)

- [ ] T045 [P] [US3] 在 `tests/dashboard_control_security_test.rs` 中添加 FR-003, FR-006, FR-013, FR-016, FR-019, SC-005 和 SC-006 的 mTLS(双向传输层安全协议认证), 授权, `ws://` 完整控制拒绝, 目标进程 IPC(进程间通信) 外网不可达, trusted proxy(可信代理) 伪造身份拒绝, session gating(会话门控), requested by(请求者) 派生和 IPC(进程间通信) 禁止转发测试.
- [ ] T046 [P] [US3] 在 `tests/dashboard_command_contract_test.rs` 中添加 FR-015, FR-017, FR-018, FR-022 和 SC-004 的全部控制命令, 历史控制命令别名拒绝, 二次确认, 非空 reason(原因), command result(命令结果) 和 audit event(审计事件) 测试.
- [ ] T047 [P] [US3] 在 `dashboard/tests/control-commands.spec.ts` 中添加危险命令二次确认, reason(原因) 必填, command result(命令结果) 和错误提示测试.

### Implementation for User Story 3(用户故事三的实现)

- [ ] T048 [P] [US3] 在 `src/dashboard/auth.rs` 中实现 mTLS(双向传输层安全协议认证) client certificate(客户端证书) 解析, RemoteIdentity(远程身份) 派生, trusted proxy(可信代理) 校验和授权范围判断.
- [ ] T049 [P] [US3] 在 `src/dashboard/command.rs` 中实现 ControlCommandRequest(控制命令请求) 校验, requested by(请求者) 覆盖保护, dangerous command(危险命令) 二次确认和 reason(原因) 非空规则.
- [ ] T050 [US3] 在 `src/dashboard/ipc_server.rs` 中实现 `command.restart_child`, `command.pause_child`, `command.resume_child`, `command.quarantine_child`, `command.remove_child`, `command.add_child` 和 `command.shutdown_tree` 到 SupervisorHandle(监督器句柄) 控制边界的映射.
- [ ] T051 [US3] 在 `src/dashboard/ipc_client.rs` 中实现 sidecar(侧车进程) 命令转发, command result(命令结果) 读取, timeout(超时) 和目标不存在错误处理.
- [ ] T052 [P] [US3] 在 `src/dashboard/audit.rs` 中实现 accepted(已接受), rejected(已拒绝) 和 completed(已完成) 命令 audit event(审计事件) 生成.
- [ ] T053 [US3] 在 `src/dashboard/session.rs` 中实现未认证, 未授权, certificate identity(证书身份) 不可解析和 control session(控制会话) 未建立时的拒绝路径.
- [ ] T054 [P] [US3] 在 `dashboard/src/components/ControlPanel.tsx` 中实现 restart child(重启子任务), pause child(暂停子任务), resume child(恢复子任务), quarantine child(隔离子任务), remove child(移除子任务), add child(添加子任务) 和 shutdown tree(关闭监督树) 控件.
- [ ] T055 [P] [US3] 在 `dashboard/src/components/ConfirmCommandDialog.tsx` 中实现 shutdown tree(关闭监督树), remove child(移除子任务) 和 add child(添加子任务) 的二次确认和 reason(原因) 必填校验.
- [ ] T056 [US3] 在 `dashboard/src/api/session.ts` 中实现 `wss://` command(命令), filter update(过滤更新), command result(命令结果) 和 error(错误) 客户端协议处理.

**Checkpoint(检查点)**: 所有用户故事都可以独立工作, 控制命令具备身份绑定和审计边界.

---

## Phase 6(阶段六): Polish & Cross-Cutting Concerns(收尾和横向事项)

**Purpose(目的)**: 覆盖性能, 文档, 格式化和端到端验证.

- [ ] T057 [P] 在 `tests/dashboard_performance_test.rs` 中添加 SC-001, SC-002, SC-008 和 SC-009 的 2 秒首包, 5 秒 200 child task(子任务), 10 秒断连诊断和 5 个 IPC path(进程间通信路径) 配置测试.
- [ ] T058 [P] 在 `dashboard/tests/dashboard-performance.spec.ts` 中添加 SC-003 的 failed(失败), quarantined(隔离) 和 restarting(重启中) child task(子任务) 定位流程测试.
- [ ] T059 [P] 在 `manual/dashboard.md` 中编写 IPC(进程间通信), sidecar(侧车进程), `wss://`, mTLS(双向传输层安全协议认证), trusted proxy(可信代理), 控制命令和诊断运行说明.
- [ ] T060 [P] 在 `README.zh.md` 中增加 dashboard(看板) 功能入口, 配置文件和验证命令说明.
- [ ] T061 运行 `cargo fmt` 并确认 `src/dashboard/mod.rs` 和所有 Rust(编程语言) 新文件格式化.
- [ ] T062 运行 `cargo test` 并确认 `tests/dashboard_snapshot_test.rs` 等 Rust(编程语言) dashboard(看板) 测试通过.
- [ ] T063 运行 `npm --prefix dashboard test` 并确认 `dashboard/package.json` 中的 Vitest(前端测试工具) 脚本通过.
- [ ] T064 运行 `npm --prefix dashboard run test:e2e` 并确认 `dashboard/playwright.config.ts` 中的 browser test(浏览器测试) 通过.

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

- 先写 `tests/` 和 `dashboard/tests/` 中的测试, 并确认实现前失败.
- 先完成 Rust(编程语言) model(模型) 和 protocol(协议), 再完成 IPC(进程间通信) 和 sidecar(侧车进程).
- 先完成 sidecar(侧车进程) contract(契约), 再完成 dashboard(看板) 前端集成.
- 完成每个用户故事后运行该故事相关测试, 再进入下一个优先级.

### Parallel Opportunities(并行机会)

- T003 到 T006 可以并行, 因为它们修改不同 dashboard(看板) 配置文件.
- T007 和 T008 可以并行, 因为它们修改不同测试文件.
- T009 到 T013 可以并行, 因为它们修改不同 `src/dashboard/` 文件.
- US1 中 T019 到 T021 可以并行, T028 到 T032 可以并行.
- US2 中 T034 和 T035 可以并行, T036, T039, T041, T042 和 T043 可以并行.
- US3 中 T045 到 T047 可以并行, T048, T049, T052, T054 和 T055 可以并行.
- T057 到 T060 可以并行, 因为它们修改不同测试和文档文件.

---

## Parallel Example(并行示例)

### User Story 1(用户故事一)

```bash
Task(任务): "T019 在 tests/dashboard_snapshot_test.rs 中添加 snapshot(快照) 集成测试"
Task(任务): "T020 在 tests/dashboard_session_contract_test.rs 中添加 session(会话) 建立顺序测试"
Task(任务): "T021 在 dashboard/tests/snapshot-view.spec.ts 中添加首屏渲染测试"
```

### User Story 2(用户故事二)

```bash
Task(任务): "T034 在 tests/dashboard_stream_test.rs 中添加事件流测试"
Task(任务): "T035 在 dashboard/tests/events-filter.spec.ts 中添加过滤测试"
Task(任务): "T036 在 src/dashboard/events.rs 中实现事件日志转换"
```

### User Story 3(用户故事三)

```bash
Task(任务): "T045 在 tests/dashboard_control_security_test.rs 中添加控制安全测试"
Task(任务): "T046 在 tests/dashboard_command_contract_test.rs 中添加命令契约测试"
Task(任务): "T047 在 dashboard/tests/control-commands.spec.ts 中添加控制命令浏览器测试"
```

---

## Implementation Strategy(实现策略)

### MVP First(先做最小可用产品)

1. 完成 Phase 1(阶段一) 和 Phase 2(阶段二).
2. 完成 Phase 3(阶段三) User Story 1(用户故事一).
3. 运行 `cargo test dashboard_snapshot_test dashboard_session_contract_test` 和 `npm --prefix dashboard run test:e2e -- snapshot-view`.
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
- 所有任务描述包含明确文件路径.
