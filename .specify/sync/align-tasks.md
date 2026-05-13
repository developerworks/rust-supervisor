# Sync Align Tasks(同步对齐任务)

Generated(生成时间): 2026-05-08T01:55:02+08:00
Based on(基于): `.specify/sync/proposals.json`
Approved Proposals(已批准提案): `P001`, `P002`

## Task A001: Align 001-create-supervisor-core/FR-063

**Spec Requirement(规格需求)**: `FR-063`

**Direction(方向)**: ALIGN(对齐, Spec to Code(规格到代码))

**Current Code(当前代码)**:

当前主仓已经把 dashboard(看板) 模型类型改为 `DashboardState`, 但是主仓协议层, 相邻 relay(中继) 仓和 UI(用户界面) 仓仍然保留 snapshot(快照) 代码标识或 wire protocol(线协议) 字面量. 用户已经明确要求 `Snapshot` 代码标识不保留, `snapshot` wire protocol(线协议) 字面量也不保留.

**Required Change(需要变更)**:

把 dashboard(看板), IPC(进程间通信), relay(中继) 和 UI(用户界面) 的协议语义统一改为 state(状态). 不得保留 compatibility export(兼容导出), 历史别名或旧协议别名.

**Files to Modify(需要修改的文件)**:

- `/Users/0x00/Documents/rust-supervisor/src/dashboard/protocol.rs`
- `/Users/0x00/Documents/rust-supervisor/src/dashboard/ipc_server.rs`
- `/Users/0x00/Documents/rust-supervisor/src/dashboard/state.rs`
- `/Users/0x00/Documents/rust-supervisor/tests/dashboard_protocol_shape_test.rs`
- `/Users/0x00/Documents/rust-supervisor/tests/dashboard_snapshot_test.rs`
- `/Users/0x00/Documents/rust-supervisor/tests/dashboard_performance_test.rs`
- `/Users/0x00/Documents/rust-supervisor/specs/003-supervisor-dashboard/contracts/ipc-protocol.md`
- `/Users/0x00/Documents/rust-supervisor/specs/003-supervisor-dashboard/contracts/wss-session.md`
- `/Users/0x00/Documents/rust-supervisor/specs/003-supervisor-dashboard/spec.md`
- `/Users/0x00/Documents/rust-supervisor/specs/003-supervisor-dashboard/plan.md`
- `/Users/0x00/Documents/rust-supervisor/specs/003-supervisor-dashboard/data-model.md`
- `/Users/0x00/Documents/rust-supervisor/specs/003-supervisor-dashboard/tasks.md`
- `/Users/0x00/Documents/rust-supervisor-relay/src/ipc_client.rs`
- `/Users/0x00/Documents/rust-supervisor-relay/src/session.rs`
- `/Users/0x00/Documents/rust-supervisor-relay/src/registry.rs`
- `/Users/0x00/Documents/rust-supervisor-relay/tests/relay_session_contract_test.rs`
- `/Users/0x00/Documents/rust-supervisor-relay/README.md`
- `/Users/0x00/Documents/rust-supervisor-relay/manual/dashboard-relay.md`
- `/Users/0x00/Documents/rust-supervisor-ui/src/types/protocol.ts`
- `/Users/0x00/Documents/rust-supervisor-ui/src/App.vue`
- `/Users/0x00/Documents/rust-supervisor-ui/src/state/snapshotStore.ts`
- `/Users/0x00/Documents/rust-supervisor-ui/src/state/eventStore.ts`
- `/Users/0x00/Documents/rust-supervisor-ui/src/mock/dashboardData.ts`
- `/Users/0x00/Documents/rust-supervisor-ui/src/components/TopologyCanvas.vue`
- `/Users/0x00/Documents/rust-supervisor-ui/tests/unit/eventStore.test.ts`
- `/Users/0x00/Documents/rust-supervisor-ui/tests/unit/snapshotStore.test.ts`
- `/Users/0x00/Documents/rust-supervisor-ui/README.md`
- `/Users/0x00/Documents/rust-supervisor-ui/FINAL_REPORT.md`

**Implementation Notes(实现说明)**:

- 把 IPC(进程间通信) method(方法) 从 `"snapshot"` 改为 `"state"`.
- 把 WSS(WebSocket 安全协议) message(消息) 从 `type: "snapshot"` 改为 `type: "state"`.
- 把 payload(载荷) 字段从 `"snapshot"` 改为 `"state"`.
- 把 `snapshot_generation` 改为 `state_generation`.
- 把 relay(中继) 中的 `DashboardSnapshot`, `ServerMessage::Snapshot`, `connect_snapshot`, `last_snapshot_generation` 等标识改为 state(状态) 命名.
- 把 UI(用户界面) 中的 `DashboardSnapshot`, `SnapshotStoreState`, `snapshotStore`, `applySnapshot`, `selectedSnapshot`, `paymentsSnapshot`, `billingSnapshot`, `searchSnapshot`, `mockSnapshots` 等标识改为 state(状态) 命名.
- 更新测试名称, 断言文本, mock(模拟数据), protocol contract(协议契约) 和 003 规格文档中的线协议示例.

**Estimated Effort(预估工作量)**: LARGE(大)

### Acceptance Criteria(验收标准)

- [X] 当前主仓, relay(中继) 仓和 UI(用户界面) 仓中不再出现 `DashboardSnapshot`, `ServerMessage::Snapshot`, `SnapshotStoreState`, `snapshotStore`, `applySnapshot`, `selectedSnapshot`, `paymentsSnapshot`, `billingSnapshot`, `searchSnapshot`, `mockSnapshots` 等代码标识.
- [X] IPC(进程间通信) request(请求) 使用 `"method": "state"`, 不再使用 `"method": "snapshot"`.
- [X] WSS(WebSocket 安全协议) message(消息) 使用 `type: "state"` 和 `"state"` payload(载荷) 字段, 不再使用 `type: "snapshot"` 或 `"snapshot"` 字段.
- [X] 线协议字段使用 `state_generation`, 不再使用 `snapshot_generation`.
- [X] 003 dashboard(看板) spec(规格), plan(计划), data model(数据模型), task(任务) 和 contract(契约) 文档全部改为 state(状态) 语义.
- [X] 不新增 compatibility export(兼容导出), 历史别名或旧协议别名.

## Task A002: Align 001-create-supervisor-core/SC-031

**Spec Requirement(规格需求)**: `SC-031`

**Direction(方向)**: ALIGN(对齐, Spec to Code(规格到代码))

**Current Code(当前代码)**:

当前主仓 `naming_contract_test` 已经覆盖主仓源码, 但是还没有覆盖相邻 relay(中继) 仓和 UI(用户界面) 仓. 现有检查也没有把 `type: "snapshot"`, `"snapshot"` 字段名和 `snapshot_generation` 这类 wire literal(线协议字面量) 纳入失败条件.

**Required Change(需要变更)**:

扩展 naming check(命名检查), 让它覆盖当前主仓, relay(中继) 仓, UI(用户界面) 仓和协议文档. 检查必须同时拒绝 code identifier(代码标识) 和 wire literal(线协议字面量) 中的禁用命名.

**Files to Modify(需要修改的文件)**:

- `/Users/0x00/Documents/rust-supervisor/src/tests/naming_contract_test.rs`
- `/Users/0x00/Documents/rust-supervisor/specs/001-create-supervisor-core/spec.md`
- `/Users/0x00/Documents/rust-supervisor/specs/001-create-supervisor-core/quickstart.md`
- `/Users/0x00/Documents/rust-supervisor/specs/003-supervisor-dashboard/contracts/ipc-protocol.md`
- `/Users/0x00/Documents/rust-supervisor/specs/003-supervisor-dashboard/contracts/wss-session.md`
- `/Users/0x00/Documents/rust-supervisor-relay/src/`
- `/Users/0x00/Documents/rust-supervisor-relay/tests/`
- `/Users/0x00/Documents/rust-supervisor-relay/manual/`
- `/Users/0x00/Documents/rust-supervisor-relay/README.md`
- `/Users/0x00/Documents/rust-supervisor-ui/src/`
- `/Users/0x00/Documents/rust-supervisor-ui/tests/`
- `/Users/0x00/Documents/rust-supervisor-ui/README.md`

**Implementation Notes(实现说明)**:

- 检查范围必须排除 `target`, `node_modules`, `dist`, `build`, `coverage`, `.specify/sync/backups`, `Cargo.lock`, `package-lock.json` 和其他第三方 generated artifact(生成产物).
- 检查必须拒绝以 `Snapshot` 或 `View` 结尾的代码标识.
- 检查必须拒绝 `snapshot()` 查询方法.
- 检查必须拒绝 `state_view` 模块名, 文件名, 方法名或字段名.
- 检查必须拒绝 wire literal(线协议字面量) 中的 `"snapshot"`, `type: "snapshot"`, `"method": "snapshot"`, `"snapshot_generation"` 和 `"snapshot"` payload(载荷) 字段.
- 检查可以保留为了构造禁用词而拆分字符串的测试代码, 但不得允许真实业务代码, 契约或文档继续使用禁用命名.

**Estimated Effort(预估工作量)**: MEDIUM(中)

### Acceptance Criteria(验收标准)

- [X] 在主仓运行 `cargo test --test naming_contract_test` 时, 检查会覆盖当前主仓, relay(中继) 仓和 UI(用户界面) 仓.
- [X] 如果任一受检路径出现 `DashboardSnapshot`, `SnapshotStoreState`, `ServerMessage::Snapshot`, `paymentsSnapshot`, `*View`, `snapshot()`, `state_view`, `"method": "snapshot"`, `type: "snapshot"`, `"snapshot"` 字段或 `snapshot_generation`, 测试必须失败.
- [X] 如果协议和代码全部使用 state(状态) 命名, 并且第三方依赖锁文件与生成产物被正确排除, 测试必须通过.
- [X] 检查结果可以定位违规文件路径和违规词, 便于后续修复.
