# Sync Align Tasks(同步对齐任务)

Generated(生成时间): 2026-05-15T01:30:16+08:00
Based on(基于): `.specify/sync/proposals.json` (interactive approve, `P001`..`P007`)
Approved Proposals(已批准提案): `P001`, `P002`, `P003`, `P004`, `P005`, `P006`, `P007` (Option C 见 `M007`)

---

## Task P001: Align 004-3-child-runtime-state-control/FR-001, FR-003, SC-001..SC-004

**Spec Requirement(规格需求)**: `004-3-child-runtime-state-control` — `FR-001`, `FR-003`, `SC-001`..`SC-004`

**Direction(方向)**: ALIGN(规格到代码)

**Current Code(当前代码)**: `ActiveChildAttempt` 仅服务关闭流水线, `current_state` 仅返回子任务数量与是否已关闭.

**Required Change(需要变更)**: 引入运行时私有 `ChildRuntimeState` 模型, 收敛 `active_attempts`, 关闭流水线从运行状态记录读取活动尝试, 扩展 `current_state` 返回每子任务运行状态摘要, 为暂停, 恢复, 隔离, 移除与关闭结果增加运行状态终态字段.

**Files to Modify(需要修改的文件)**:

- `src/runtime/control_loop.rs`
- `src/runtime/shutdown_pipeline.rs`
- `src/runtime/mod.rs`
- `src/control/command.rs`
- `src/shutdown/report.rs` (若公开状态形状变化)
- `src/tests/supervisor_control_test.rs` 或新增 `src/tests/` 下对齐测试

**Estimated Effort(预估工作量)**: LARGE(大)

### Acceptance Criteria(验收标准)

- [ ] 每个声明子任务在运行时有唯一运行状态记录视图, 含 `generation`, `attempt`, 活动句柄与次数额度字段中与规格一致子集.
- [ ] `ShutdownTree` 关闭路径从运行状态记录读取活动尝试, 不维护第二套并行事实源.
- [ ] `current_state` 或等价查询返回每子任务摘要, 满足 `FR-003` 可测试子集.
- [ ] 新增或更新测试证明运行状态摘要在暂停, 运行与关闭后一致.

---

## Task P002: Align 004-3-child-runtime-state-control/FR-002

**Spec Requirement(规格需求)**: `004-3-child-runtime-state-control` — `FR-002`

**Direction(方向)**: ALIGN(规格到代码)

**Current Code(当前代码)**: `pause_child`, `remove_child`, `quarantine_child` 主要 `set_child_state`, 未停止真实任务.

**Required Change(需要变更)**: 暂停发取消并等待结束或超时, 隔离停止尝试并禁止自动重启, 移除停止尝试并移除或标记运行状态记录, 重复命令幂等.

**Files to Modify(需要修改的文件)**:

- `src/runtime/control_loop.rs`
- `src/control/command.rs`
- `src/tests/supervisor_control_test.rs`

**Estimated Effort(预估工作量)**: LARGE(大)

### Acceptance Criteria(验收标准)

- [ ] 三条命令均触发对真实 `CancellationToken` / `AbortHandle` 路径的可观察动作.
- [ ] 测试断言任务在暂停, 隔离或移除后不再无约束运行.
- [ ] 重复同一停止类命令返回幂等结果且无双重停止风暴.

---

## Task P003: Align 004-4-generation-fencing/FR-001..FR-003, SC-001..SC-004

**Spec Requirement(规格需求)**: `004-4-generation-fencing` — `FR-001`..`FR-003`, `SC-001`..`SC-004`

**Direction(方向)**: ALIGN(规格到代码)

**Current Code(当前代码)**: `spawn_child_attempt` 立即 `abort` 旧尝试并启动新尝试, `record_child_exit` 未校验代次.

**Required Change(需要变更)**: 统一重启流水线: 取消, 等待, 超时中止, 再升代次, 退出记录校验 `(child_id, generation, attempt)`, 旧代次迟到标 stale, `current_state` 暴露代次.

**Files to Modify(需要修改的文件)**:

- `src/runtime/control_loop.rs`
- `src/runtime/shutdown_pipeline.rs`
- `src/tests/` 下代次与重启相关集成测试

**Estimated Effort(预估工作量)**: LARGE(大)

### Acceptance Criteria(验收标准)

- [ ] 连续重启仅最后一代可处于运行中语义有测试覆盖.
- [ ] 旧代次迟到退出不覆盖注册表中当前代次运行状态记录.
- [ ] `current_state` 或等价结构暴露 `generation` 与冲突诊断字段.

---

## Task P004: Align 004-1-runtime-lifecycle-guard/FR-002, SC-002

**Spec Requirement(规格需求)**: `004-1-runtime-lifecycle-guard` — `FR-002`, `SC-002`

**Direction(方向)**: ALIGN(规格到代码)

**Current Code(当前代码)**: `watchdog` 主要字符串 `broadcast`, 未将 `RuntimeControlLoopFailed` 写入 `ObservabilityPipeline`.

**Required Change(需要变更)**: 启动路径持有观测入口, 失败路径构造类型化事件并经 `ObservabilityPipeline::emit` 落 journal, metrics, audit, 保留字符串广播为兼容旁路.

**Files to Modify(需要修改的文件)**:

- `src/runtime/watchdog.rs`
- `src/runtime/lifecycle.rs` 或 `Supervisor` 启动绑定
- `src/event/payload.rs`
- `src/observe/pipeline.rs`
- `src/observe/metrics.rs`
- `src/tests/` 下控制循环失败可观测性测试

**Estimated Effort(预估工作量)**: MEDIUM(中)

### Acceptance Criteria(验收标准)

- [ ] 控制循环异常退出时, 测试能断言至少一条类型化监督器事件进入观测测试夹具.
- [ ] metrics 或 audit 中至少一项出现与失败对应的低基数计数或事实记录.

---

## Task P005: Align 004-2-real-shutdown-pipeline/FR-003

**Spec Requirement(规格需求)**: `004-2-real-shutdown-pipeline` — `FR-003`

**Direction(方向)**: ALIGN(规格到代码)

**Current Code(当前代码)**: `ShutdownReconcileReport` 中 journal, metrics 状态来自 `core_runtime_completed()` 默认常量.

**Required Change(需要变更)**: `execute_shutdown` 各阶段发出类型化关闭事件, 对账状态由 emit 结果或流水线可观测状态推导, 套接字保持 `NotOwned`.

**Files to Modify(需要修改的文件)**:

- `src/runtime/control_loop.rs`
- `src/shutdown/report.rs`
- `src/event/payload.rs`
- `src/observe/pipeline.rs`
- `src/tests/supervisor_real_shutdown_pipeline_test.rs`

**Estimated Effort(预估工作量)**: MEDIUM(中)

### Acceptance Criteria(验收标准)

- [ ] 测试或观测夹具证明关闭完成路径上 journal, metrics 状态非无条件 `Recorded` 盲填.
- [ ] 契约测试仍通过, 且不引入 compatibility export.

---

## Task P006: Align 003-supervisor-dashboard/SC-003, SC-012

**Spec Requirement(规格需求)**: `003-supervisor-dashboard` — `SC-003`, `SC-012`

**Direction(方向)**: ALIGN(规格到代码, 跨仓)

**Current Code(当前代码)**: `specs/003-supervisor-dashboard/tasks.md` 中 `T066` 等仍未闭合.

**Required Change(需要变更)**: 在相邻 `rust-supervisor-ui` 仓库实现 Playwright 定位验收与 `package.json` / `src` 下无 React 组件体系证明.

**Files to Modify(需要修改的文件)**:

- `../rust-supervisor-ui/` (仓库根与 `tests/` 或 CI 配置, 本任务不强制修改当前主仓)
- `specs/003-supervisor-dashboard/tasks.md` (闭合勾选与证据链接)

**Estimated Effort(预估工作量)**: LARGE(大)

### Acceptance Criteria(验收标准)

- [ ] `T066` 及相关成功标准在 `tasks.md` 可勾选并附运行命令或 CI 链接.
- [ ] 自动化脚本证明无 React 运行时依赖与无 `.tsx`/`.jsx` 业务源码.

---

## Task M007: Apply P007 Option C — drift supersession index

**Spec Requirement(规格需求)**: 工作流元任务 — `001-create-supervisor-core` 宽口径条款与 `004` 系列关系

**Direction(方向)**: META(元, 不改 `001` 正文)

**Current Code(当前代码)**: 无代码变更.

**Required Change(需要变更)**: 维护 `.specify/sync/drift-supersession.md`, 并在下次 `speckit.sync.analyze` 人工 triage 或脚本增强时引用该表, 将表中左列需求在已选右列规格活跃时的漂移标为 **SUPERSEDED**.

**Files to Modify(需要修改的文件)**:

- `.specify/sync/drift-supersession.md` (已创建, 后续迭代补行)
- 可选: 生成 `drift-report` 的脚本或技能说明

**Estimated Effort(预估工作量)**: SMALL(小)

### Acceptance Criteria(验收标准)

- [ ] 评审承认 `001` 正文未改, 但同步工作流有可查 supersession 表.
- [ ] `apply-report` 与本任务交叉引用一致.

---

以下为历史 apply 产物 (2026-05-08), 其中绝对路径可能指向旧工作区克隆, **保留供审计**, 不作为当前仓库路径规范.

---

## Historical: 2026-05-08 apply(历史应用)

Generated(生成时间): 2026-05-08T01:55:02+08:00
Based on(基于): `.specify/sync/proposals.json`
Approved Proposals(已批准提案): `P001`, `P002`

### Task A001: Align 001-create-supervisor-core/FR-063

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

### Task A002: Align 001-create-supervisor-core/SC-031

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
