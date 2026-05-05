# Feature Specification(功能规格): 监督任务可视化界面

**Feature Branch(功能分支)**: `003-supervisor-dashboard`
**Created(创建日期)**: 2026-05-05
**Status(状态)**: Draft(草稿)
**Input(输入)**: 用户描述:"依据 `tmp/supervisor-dashboard-plan.md` 创建 feature 003(功能 003). 目标进程必须打开 IPC(进程间通信), 中继必须通过 IPC(进程间通信) 和目标进程通信, 并读取监督树, 状态, 事件和日志. 中继必须对外提供服务器接口, 例如 WebSocket(网络套接字协议). mTLS(双向传输层安全协议认证) 和 WebSocket(网络套接字协议) 必须通过 `wss://` 协同工作. 规格修订要求: 中继可以和多个 IPC(进程间通信) 进行通信. `rust-tokio-supervisor` 必须提供外部化 IPC path(进程间通信路径) 配置, 目标进程使用该配置打开 IPC(进程间通信). 中继也必须配置一个或多个 IPC path(进程间通信路径), 并指向多个目标进程. 中继和目标进程 IPC(进程间通信) 建立连接后, 目标进程必须主动发送事件. 远程客户端必须先完成和中继的控制会话建立, 然后才能触发中继与目标进程 IPC(进程间通信) 建立或绑定通信."

## User Scenarios & Testing(用户场景和测试) *(mandatory(必填))*

### User Story 1(用户故事一) - 远程查看监督树和状态 (Priority(优先级): P1)

operator(操作者) 需要从远程安全连接进入 dashboard(看板), 直接查看一个或多个目标进程中的 supervisor tree(监督树) 结构, child task(子任务) 状态, health(健康状态), readiness(就绪状态), restart(重启) 信息和 shutdown(关闭) 状态. 操作者不应该登录目标机器或读取进程内部调试输出后再手工拼接状态.

**Why this priority(为什么是这个优先级)**: 查看监督树结构和当前状态是后续日志分析和控制操作的入口. 如果操作者不能先确认哪个 child task(子任务) 出现异常, 完整控制能力就没有可靠上下文.

**Independent Test(独立测试)**: 启动两个带多个 child task(子任务) 的目标进程, 并让它们使用不同 IPC path(进程间通信路径). 再通过远程安全连接打开 dashboard(看板). 测试必须证明界面展示每个目标进程的 root supervisor(根监督器), 所有 child task(子任务), 依赖关系, 当前状态和生成时间.

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** 多个目标进程已经按各自 IPC path(进程间通信路径) 启动本机 IPC(进程间通信), 并且 sidecar(侧车进程) 已经配置这些路径, **When(当)** 已认证操作者打开 dashboard(看板), **Then(则)** 系统必须展示每个目标进程对应的 snapshot(快照), 并且每份 snapshot(快照) 都必须包含监督树结构和当前状态.
2. **Given(假设)** 某个 child task(子任务) 处于 paused(暂停), quarantined(隔离), failed(失败) 或 restarting(重启中) 状态, **When(当)** 操作者查看监督树节点, **Then(则)** 该节点必须用可区分状态展示, 并且详情区域必须说明状态, 最近事件和关联策略决定.

---

### User Story 2(用户故事二) - 观测事件和日志流 (Priority(优先级): P2)

operator(操作者) 需要在 dashboard(看板) 中持续观测 supervisor event(监督器事件), log record(日志记录) 和 command audit(命令审计) 结果. 操作者需要按 child task(子任务), event type(事件类型), severity(严重程度), sequence(序号) 和 correlation id(关联标识) 过滤, 以便快速定位故障来源.

**Why this priority(为什么是这个优先级)**: 监督系统的主要价值来自可解释的失败信号. 没有连续事件和日志, 操作者只能看到静态状态, 无法解释失败, 重启, 熔断和关闭过程.

**Independent Test(独立测试)**: 让多个目标进程产生启动, 失败, 重启, 控制命令和关闭事件, 再通过 dashboard(看板) 观察实时流. 测试必须证明目标进程在 IPC(进程间通信) 连接建立后主动发送事件, 事件和日志按目标进程分组后顺序追加, 过滤器生效, 连接断开后能重新获得最新 snapshot(快照).

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** 目标进程 IPC(进程间通信) 已经和 sidecar(侧车进程) 建立连接, **When(当)** 目标进程持续产生 supervisor event(监督器事件), **Then(则)** 目标进程必须主动把新事件推送给 sidecar(侧车进程), sidecar(侧车进程) 必须把事件转发给已认证操作者的 dashboard(看板).
2. **Given(假设)** 多个目标进程同时产生 supervisor event(监督器事件), **When(当)** 已认证操作者保持 dashboard(看板) 打开, **Then(则)** 新事件必须按目标进程和 sequence(序号) 自动进入事件列表, 并且同一目标进程内的顺序不能倒置.
3. **Given(假设)** event journal(事件日志缓冲区) 因容量限制丢弃旧事件, **When(当)** 操作者查看日志区域, **Then(则)** 系统必须显示对应目标进程的 dropped count(丢弃数量), 并且保留最近可用事件.

---

### User Story 3(用户故事三) - 安全执行完整控制命令 (Priority(优先级): P3)

operator(操作者) 需要在完成身份认证后从 dashboard(看板) 执行 restart child(重启子任务), pause child(暂停子任务), resume child(恢复子任务), quarantine child(隔离子任务), remove child(移除子任务), add child(添加子任务) 和 shutdown tree(关闭监督树). 所有命令必须绑定操作者身份, 目标对象和 reason(原因), 并且必须形成 audit event(审计事件).

**Why this priority(为什么是这个优先级)**: 远程完整控制会改变目标进程生命周期. 该能力必须建立在可观察状态和可审计身份之上, 否则会增加误操作和不可追踪风险.

**Independent Test(独立测试)**: 使用已授权远程身份先和 sidecar(侧车进程) 建立控制会话, 再选择目标进程和目标 child task(子任务) 执行每一种控制命令. 测试必须证明命令在控制会话建立前不会触发目标进程 IPC(进程间通信) 通信, 命令结果返回到当前连接, 目标状态更新到 snapshot(快照) 或 state delta(状态增量), 并且 audit event(审计事件) 包含操作者身份, 目标进程, 命令, 目标, reason(原因) 和结果.

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** 已授权操作者已经和 sidecar(侧车进程) 建立控制会话, 并选择一个目标进程中的 child task(子任务), **When(当)** 他提交 pause child(暂停子任务) 并填写 reason(原因), **Then(则)** 系统必须返回 command result(命令结果), 更新该目标进程中的节点状态, 并记录 audit event(审计事件).
2. **Given(假设)** 未认证连接或未授权身份尝试执行 shutdown tree(关闭监督树), **When(当)** 请求到达 sidecar(侧车进程), **Then(则)** 系统必须拒绝命令, 不得转发到目标进程 IPC(进程间通信), 并记录拒绝原因.
3. **Given(假设)** 操作者尝试执行 shutdown tree(关闭监督树), remove child(移除子任务) 或 add child(添加子任务), **When(当)** 他没有完成二次确认或没有填写 reason(原因), **Then(则)** dashboard(看板) 必须阻止提交.

### Edge Cases(边界情况)

- 目标进程 IPC(进程间通信) 不可达时, sidecar(侧车进程) 必须把连接状态标记为 unavailable(不可用), 并向远程客户端返回可理解错误.
- sidecar(侧车进程) 与目标进程 IPC(进程间通信) 断开后重新连接时, 远程客户端必须收到新的 snapshot(快照), 而不是继续显示过期状态.
- 多个 IPC path(进程间通信路径) 中只有部分目标进程可达时, sidecar(侧车进程) 必须继续展示可达目标进程, 并单独标记不可达目标进程.
- 多个目标进程使用重复 target id(目标标识) 或重复 IPC path(进程间通信路径) 时, sidecar(侧车进程) 必须拒绝该配置并说明冲突项.
- 远程客户端未完成与 sidecar(侧车进程) 的控制会话时, 系统不得为该客户端触发目标进程 IPC(进程间通信) 建立或绑定通信.
- 远程客户端未提供有效 client certificate(客户端证书) 时, 系统不得建立可用控制会话.
- 远程连接使用 `ws://` 时, 系统不得允许访问完整控制能力.
- 外部客户端尝试绕过 sidecar(侧车进程) 直接访问目标进程 IPC(进程间通信) 时, 系统不得提供外网可达入口, 并且远程控制能力必须保持不可用.
- TLS(传输层安全协议) 在可信代理层终止时, sidecar(侧车进程) 必须只接受可信代理传入的已验证身份, 并拒绝普通客户端伪造身份字段.
- 控制命令目标不存在, 已移除或状态已经完成时, 系统必须返回明确 command error(命令错误), 并保持命令幂等边界.
- event journal(事件日志缓冲区) 溢出时, 系统必须展示 dropped count(丢弃数量), 并继续保留最近事件.
- 客户端发送旧协议别名或历史控制命令别名时, 系统必须返回明确拒绝错误, 不得把别名映射为本功能的有效协议或控制命令.

## Requirements(需求) *(mandatory(必填))*

### Functional Requirements(功能需求)

- **FR-001**: `rust-tokio-supervisor` 必须提供外部化 IPC path(进程间通信路径) 配置, 目标进程必须使用该配置打开本机 IPC(进程间通信) 入口.
- **FR-002**: 目标进程必须通过本机 IPC(进程间通信) 入口供 sidecar(侧车进程) 读取 supervisor topology(监督拓扑), runtime state(运行时状态), event stream(事件流), log stream(日志流) 和 command result(命令结果).
- **FR-003**: 目标进程 IPC(进程间通信) 不得直接暴露到外网, 外网访问必须经过 sidecar(侧车进程).
- **FR-004**: sidecar(侧车进程) 必须支持配置多个 IPC path(进程间通信路径), 并且每个 IPC path(进程间通信路径) 必须指向一个明确 target process(目标进程).
- **FR-005**: sidecar(侧车进程) 必须能同时维护多个目标进程 IPC(进程间通信) 连接, 并把每个连接的 connected(已连接), reconnecting(重连中), unavailable(不可用) 状态暴露给远程客户端.
- **FR-006**: 远程客户端必须先完成与 sidecar(侧车进程) 的控制会话建立, 然后才能触发 sidecar(侧车进程) 与目标进程 IPC(进程间通信) 建立或绑定通信.
- **FR-007**: sidecar(侧车进程) 和目标进程 IPC(进程间通信) 建立连接后, 目标进程必须主动向 sidecar(侧车进程) 发送 supervisor event(监督器事件), log record(日志记录) 和可用状态变化.
- **FR-008**: 系统必须提供 snapshot(快照), 它必须包含 target process identity(目标进程身份), supervisor topology(监督拓扑), runtime state(运行时状态), recent events(最近事件), recent logs(最近日志), dropped event count(丢弃事件数量), config version(配置版本) 和 generated time(生成时间).
- **FR-009**: supervisor topology(监督拓扑) 必须至少表达 root supervisor(根监督器), child task(子任务), child path(子任务路径), dependencies(依赖), tags(标签), criticality(关键程度) 和 declaration order(声明顺序).
- **FR-010**: runtime state(运行时状态) 必须至少表达 child lifecycle state(子任务生命周期状态), health(健康状态), readiness(就绪状态), generation(代次), attempt(尝试次数), restart count(重启次数), last failure(最近失败), last policy decision(最近策略决定) 和 shutdown state(关闭状态).
- **FR-011**: event stream(事件流) 必须保留 target process identity(目标进程身份), sequence(序号), correlation id(关联标识), event type(事件类型), target path(目标路径), child id(子任务标识), occurred time(发生时间) 和 config version(配置版本).
- **FR-012**: log stream(日志流) 必须能和 event stream(事件流) 通过 target process identity(目标进程身份), sequence(序号) 或 correlation id(关联标识) 关联.
- **FR-013**: sidecar(侧车进程) 必须对外提供远程 secure session(安全会话), 并且远程控制会话必须在双方身份认证完成后才能建立.
- **FR-014**: 远程 secure session(安全会话) 必须在建立后先发送可见 target process list(目标进程列表) 和授权范围, 然后才能发送 snapshot(快照), event(事件), log(日志), state delta(状态增量), command result(命令结果) 和 error(错误).
- **FR-015**: 系统必须支持 restart child(重启子任务), pause child(暂停子任务), resume child(恢复子任务), quarantine child(隔离子任务), remove child(移除子任务), add child(添加子任务) 和 shutdown tree(关闭监督树) 控制命令.
- **FR-016**: 每个控制命令必须包含 command id(命令标识), target process identity(目标进程身份), target(目标), reason(原因) 和由认证身份派生的 requested by(请求者). 客户端不得覆盖 requested by(请求者).
- **FR-017**: shutdown tree(关闭监督树), remove child(移除子任务) 和 add child(添加子任务) 必须要求二次确认和非空 reason(原因).
- **FR-018**: 每个被接受, 被拒绝和已完成的控制命令都必须产生 audit event(审计事件), 并记录身份, 目标进程, 命令, 目标, reason(原因), 时间和结果.
- **FR-019**: 未认证身份, 未授权身份, 证书身份不可解析或控制会话未建立时, 系统必须拒绝远程会话或控制命令, 并不得把控制请求转发到目标进程 IPC(进程间通信).
- **FR-020**: dashboard(看板) 必须支持按 target process identity(目标进程身份), child task(子任务), lifecycle state(生命周期状态), event type(事件类型), severity(严重程度), sequence(序号) 和 correlation id(关联标识) 过滤事件和日志.
- **FR-021**: dashboard(看板) 必须在连接断开, 目标进程不可用, 认证失败, 控制命令失败和事件丢失时显示可理解诊断.
- **FR-022**: 系统不得提供 compatibility export(兼容导出), 旧协议别名或历史控制命令别名来表达本功能.

### Key Entities(关键实体) *(include if feature involves data(涉及数据时填写))*

- **DashboardSession(看板会话)**: 已认证远程连接, 表达操作者身份, 权限范围, 连接状态和最近同步位置.
- **TargetProcessConfig(目标进程配置)**: `rust-tokio-supervisor` 和 sidecar(侧车进程) 使用的 IPC path(进程间通信路径) 配置, 用于确定目标进程监听路径和中继连接路径.
- **TargetProcessConnection(目标进程连接)**: sidecar(侧车进程) 与一个目标进程 IPC(进程间通信) 之间的本机连接, 用于接收目标进程主动推送的事件和日志, 并转发控制命令.
- **TargetProcessRegistry(目标进程注册表)**: sidecar(侧车进程) 中保存多个 target process identity(目标进程身份), IPC path(进程间通信路径), 连接状态和授权范围的集合.
- **DashboardSnapshot(看板快照)**: 打开 dashboard(看板) 或重连后返回的完整视图, 包含目标进程身份, 监督拓扑, 当前状态, 最近事件, 最近日志和丢弃数量.
- **SupervisorTopology(监督拓扑)**: 监督树的可视化结构, 包含节点, 边, 路径, 依赖关系, 标签和声明顺序.
- **SupervisorNode(监督节点)**: 一个 root supervisor(根监督器) 或 child task(子任务) 的可视化单元, 包含身份, 名称, 路径, 当前状态和关键诊断字段.
- **SupervisorEdge(监督边)**: 监督树父子关系或 child task(子任务) 依赖关系, 用于解释启动顺序, 关闭顺序和重启范围.
- **EventRecord(事件记录)**: 目标进程在 IPC(进程间通信) 连接建立后主动发送的生命周期事实, 用于实时列表, 节点详情和诊断回放.
- **LogRecord(日志记录)**: 与监督事件关联的日志事实, 用于排查启动, 重启, 关闭和控制命令问题.
- **ControlCommandRequest(控制命令请求)**: 远程操作者在控制会话建立后发起的控制意图, 包含目标进程, 目标, 命令, reason(原因) 和由系统派生的身份.
- **ControlCommandResult(控制命令结果)**: 目标进程执行控制命令后的结果, 用于 UI(用户界面) 反馈, 状态刷新和 audit event(审计事件).
- **RemoteIdentity(远程身份)**: 由 mTLS(双向传输层安全协议认证) 验证后的操作者或服务身份, 用于权限判断和审计归因.

## Constitution Alignment(宪章对齐) *(mandatory(必填))*

### Supervision Contract(监督契约)

- **Lifecycle impact(生命周期影响)**: 本功能会监控目标进程中的监督树, 并允许远程操作者触发添加, 移除, 暂停, 恢复, 重启, 隔离和关闭行为. 所有行为必须通过目标进程已有监督控制边界执行.
- **Failure behavior(失败行为)**: 目标进程 IPC(进程间通信) 不可用, 远程身份无效, 控制命令非法或目标不存在时, 系统必须返回结构化错误, 并说明失败阶段和真实原因.
- **Shutdown behavior(关闭行为)**: shutdown tree(关闭监督树) 必须保持已有 Shutdown Without Orphaned Tasks(关闭后不留下孤儿任务) 语义. dashboard(看板) 和 sidecar(侧车进程) 在目标进程关闭时必须关闭或降级事件流, 并向操作者显示最终状态.

### Rust Boundary and Observability Requirements(Rust 边界和可观察性需求)

- **Module ownership(模块所有权)**: dashboard inspection(看板检查), IPC(进程间通信) 路径配置, IPC(进程间通信) 协议, 多目标进程连接边界和目标进程主动事件推送属于清晰的新模块. 它们必须复用现有 `SupervisorHandle`(监督器句柄), `SupervisorTree`(监督树), `SupervisorState`(监督器状态), `SupervisorEvent`(监督器事件) 和 `EventJournal`(事件日志缓冲区), 不得绕过运行时控制循环.
- **Compatibility exports(兼容导出)**: None(无).
- **Diagnostics(诊断)**: IPC(进程间通信), IPC path(进程间通信路径) 配置, 多目标连接, 远程会话, 事件丢失, 认证失败, 命令拒绝和命令完成都必须提供结构化诊断, 并能指向目标进程, 连接, 命令和操作者身份.
- **Dependency impact(依赖影响)**: 本功能涉及 IPC(进程间通信), secure remote session(安全远程会话), event streaming(事件流), dashboard rendering(看板渲染) 和 mTLS(双向传输层安全协议认证). 新依赖必须在 plan(计划) 阶段逐项说明理由, 并保持可选边界.

### Chinese Writing(中文写作)

- **Writing language(写作语言)**: 本规格使用中文写作.
- **Term format(术语格式)**: 英文术语以 `English(中文说明)` 形式出现.
- **Forbidden style(禁止风格)**: 本规格不使用非中文正文, 片段式语言, 生僻词或方言.

## Success Criteria(成功标准) *(mandatory(必填))*

### Measurable Outcomes(可衡量结果)

- **SC-001**: 已认证操作者在打开 dashboard(看板) 后 2 秒内看到首个 target process list(目标进程列表) 和至少一个 snapshot(快照), 每个可达目标进程的 snapshot(快照) 必须覆盖 100% 已声明 child task(子任务).
- **SC-002**: 对 5 个目标进程且总计包含 200 个 child task(子任务) 的监督树集合, dashboard(看板) 必须在 5 秒内完成首次可用展示.
- **SC-003**: 95% 操作者必须能在 30 秒内从 dashboard(看板) 找到一个 failed(失败), quarantined(隔离) 或 restarting(重启中) 的 child task(子任务) 及其最近事件.
- **SC-004**: 100% 接受, 拒绝和完成的控制命令都必须产生 audit event(审计事件).
- **SC-005**: 100% 未认证远程连接, 未授权控制请求和未建立控制会话的远程客户端不得触发目标进程 IPC(进程间通信) 建立, 绑定或命令转发.
- **SC-006**: 100% 控制命令必须携带非空 reason(原因), 并且 requested by(请求者) 必须来自已认证远程身份.
- **SC-007**: 每个目标进程主动发送的事件和日志在同一连接内必须按 sequence(序号) 单调展示, 顺序错误次数必须为 0.
- **SC-008**: 任一目标进程 IPC(进程间通信) 断开后, sidecar(侧车进程) 必须在 10 秒内向远程客户端显示该目标进程的 unavailable(不可用) 或 reconnecting(重连中) 状态.
- **SC-009**: sidecar(侧车进程) 必须能同时配置并区分至少 5 个 IPC path(进程间通信路径), 并且重复路径或重复目标身份的接受次数必须为 0.

## Assumptions(假设)

- 第一版目标平台是 Linux(操作系统) 和 macOS(操作系统), Windows(操作系统) named pipe(命名管道) 不进入本功能范围.
- `rust-tokio-supervisor` 的目标进程 IPC path(进程间通信路径) 配置属于公开配置输入的一部分, 但具体字段名称在 plan(计划) 阶段确定.
- sidecar(侧车进程) 至少支持多个静态 IPC path(进程间通信路径) 配置, 动态注册目标进程不进入第一版范围.
- 外网远程连接必须使用 `wss://` 和 mTLS(双向传输层安全协议认证), `ws://` 不进入完整控制范围.
- TLS(传输层安全协议) 默认由 sidecar(侧车进程) 终止. 如果部署在可信代理后面, 身份传递规则必须在后续 plan(计划) 中写清楚.
- sidecar(侧车进程) 不直接持有 `SupervisorHandle`(监督器句柄), 它只能在远程控制会话建立后通过目标进程 IPC(进程间通信) 读取状态和提交控制命令.
- 第一版不引入持久化数据库. 事件和日志以目标进程内存中的 recent data(最近数据) 和实时流为准.
- 浏览器使用 mTLS(双向传输层安全协议认证) 时, 客户端证书由操作系统或浏览器证书库管理, dashboard(看板) 不直接从网页脚本选择证书.
