# Tasks(任务): 创建监督器核心

**Input(输入)**: 设计文档来自 `/specs/001-create-supervisor-core/`
**Prerequisites(前置文档)**: plan.md, spec.md, research.md, data-model.md, contracts/public-api.md, quickstart.md

**Tests(测试)**: 必需. 功能规格和宪章要求行为变化先有外部测试任务, 再有实现任务. 单元测试, 契约测试和集成测试都必须放在 `tests/` 目录, 不得写入 `src/` 模块文件.

**Organization(组织方式)**: 任务按用户故事分组, 使每个故事都可以作为独立增量实现和测试. `[P]` 只用于修改不同文件且没有未完成依赖的任务.

## Format(格式): `[ID] [P?] [Story] Description(描述)`

- **[P]**: 任务可以并行, 因为它修改不同文件, 并且不依赖未完成任务.
- **[Story]**: 用户故事标签, 只用于故事阶段.
- 每个任务都写出它修改或验证的准确文件路径.
- 测试任务必须写入 `tests/` 目录, 并在描述中说明被测模块或行为.

## Phase 1(阶段一): Setup(共享基础设施)

**Purpose(目的)**: 建立 crate(包) 依赖, 模块外壳和外部测试文件位置.

- [ ] T001 更新 `Cargo.toml`, 加入计划依赖和功能: `tokio-util`, `metrics`, `thiserror`, `serde`, `serde_json`, `uuid`, `rand` 和 Tokio(异步运行时) `test-util`.
- [ ] T002 创建 `src/supervision/mod.rs`, 并在 `src/supervision/` 下创建模块外壳文件, 包含 `readiness.rs`, `journal.rs` 和 `summary.rs`.
- [ ] T003 [P] 创建 `tests/supervisor_id.rs`, `tests/supervisor_error.rs`, `tests/supervisor_defaults.rs`, `tests/supervisor_lifecycle.rs`, `tests/supervisor_readiness.rs`, `tests/supervisor_policy.rs`, `tests/supervisor_shutdown.rs`, `tests/supervisor_blocking.rs`, `tests/supervisor_tree.rs`, `tests/supervisor_observe.rs`, `tests/supervisor_diagnostics.rs`, `tests/supervisor_control.rs` 和 `tests/supervisor_api.rs`.
- [ ] T004 更新 `src/lib.rs`, 暴露项目自有 `supervision` 模块.
- [ ] T005 保持 `src/main.rs` 为轻量 demo(演示) 入口, 不放入 supervisor runtime(监督器运行时) 逻辑.

---

## Phase 2(阶段二): Foundational(阻塞前置基础)

**Purpose(目的)**: 定义所有故事都会使用的身份, 错误, 事件, 策略默认值和测试支持.

**Critical(关键要求)**: 本阶段完成前, 不得开始任何用户故事实现.

### Tests First(先写测试)

- [ ] T006 [P] 在 `tests/supervisor_api.rs` 中添加公开模块 compile-contract(编译契约) 测试.
- [ ] T007 [P] 在 `tests/supervisor_id.rs` 中添加身份和路径校验的外部单元测试, 覆盖 `src/supervision/id.rs`.
- [ ] T008 [P] 在 `tests/supervisor_error.rs` 中添加 typed failure classification(类型化失败分类) 外部单元测试, 覆盖 `src/supervision/error.rs`.
- [ ] T009 [P] 在 `tests/supervisor_defaults.rs` 中添加默认 restart(重启), backoff(退避), health(健康), readiness(就绪), shutdown(关闭) 和 meltdown(熔断) 常量测试.

### Implementation(实现)

- [ ] T010 [P] 在 `src/supervision/id.rs` 中实现 `ChildId`, `SupervisorId`, `SupervisorPath`, `Generation` 和 `Attempt`.
- [ ] T011 [P] 在 `src/supervision/error.rs` 中实现 `SupervisorError`, `TaskFailure` 和 `TaskFailureKind`.
- [ ] T012 [P] 在 `src/supervision/event.rs` 中实现 event sequence(事件序号), correlation id(关联标识) 和 base time(基础时间) 帮助函数.
- [ ] T013 [P] 在 `src/supervision/policy.rs` 和 `src/supervision/backoff/mod.rs` 中实现默认 restart(重启), backoff(退避), health(健康), readiness(就绪), shutdown(关闭) 和 meltdown(熔断) 常量.
- [ ] T014 [P] 在 `src/supervision/test_support.rs` 中实现 paused time(暂停时间), fake task factory(假任务工厂), readiness control(就绪控制), event collection(事件收集) 和 deterministic jitter(确定性抖动) 帮助函数.
- [ ] T015 在 `src/supervision/mod.rs` 中连接基础模块.
- [ ] T016 通过 `tests/supervisor_api.rs` 重新运行基础 API(接口) 编译检查.

**Checkpoint(检查点)**: 身份, 错误, 基础事件, 策略默认值和测试支持已经可以编译.

---

## Phase 3(阶段三): User Story 1(用户故事一) - 声明并运行子任务 (Priority(优先级): P1)

**Goal(目标)**: 维护者可以定义 `ChildSpec`(子任务规格), 通过 `TaskFactory`(任务工厂) 构建 fresh task attempt(新任务尝试), 启动 worker(工作任务), 控制 readiness(就绪), 观察 `ChildStarting`, `ChildRunning` 和 `ChildReady`, 并查询 snapshot(快照).

**Independent Test(独立测试)**: 定义一个 worker child(工作子任务), 启动 supervisor(监督器), 断言 running state(运行状态), ready state(就绪状态) 和启动事件, 然后关闭它.

### Tests for User Story 1(用户故事一的测试)

- [ ] T017 [US1] 在 `tests/supervisor_lifecycle.rs` 中添加 `ChildSpec`(子任务规格), `TaskFactory`(任务工厂), `Service trait`(服务特征) 和 `service_fn`(函数适配器) 契约测试.
- [ ] T018 [P] [US1] 在 `tests/supervisor_readiness.rs` 中添加 explicit readiness(显式就绪) 测试, 验证报告 ready(已就绪) 前 snapshot(快照) 和 event(事件) 不显示 ready(已就绪).
- [ ] T019 [P] [US1] 在 `tests/supervisor_api.rs` 中添加 `ChildSpec`(子任务规格) 校验外部单元测试.
- [ ] T020 [P] [US1] 在 `tests/supervisor_lifecycle.rs` 中添加 `TaskContext`(任务上下文) fresh-attempt(新尝试) 外部单元测试.

### Implementation for User Story 1(用户故事一的实现)

- [ ] T021 [P] [US1] 在 `src/supervision/spec.rs` 中实现 `ChildSpec`, `SupervisorSpec`, child kind(子任务种类), tags(标签), dependencies(依赖), criticality(关键程度) 和 readiness policy(就绪策略) 字段.
- [ ] T022 [P] [US1] 在 `src/supervision/task.rs` 中实现 `TaskFactory`, `TaskContext`, `TaskResult`, `Service trait`(服务特征), `service_fn`(函数适配器), readiness reporter(就绪报告器) 和 boxed task future(装箱任务异步值) 别名.
- [ ] T023 [P] [US1] 在 `src/supervision/readiness.rs` 中实现 `ReadinessPolicy`(就绪策略), immediate readiness(立即就绪) 和 explicit readiness(显式就绪) 信号.
- [ ] T024 [US1] 在 `src/supervision/registry.rs` 中实现 single-child registry(单子任务注册表), `ChildRuntime` 启动状态和 readiness(就绪) 状态.
- [ ] T025 [US1] 在 `src/supervision/runtime.rs` 和 `src/supervision/child_runner.rs` 中实现最小 `Supervisor::start` worker(工作任务) 启动路径.
- [ ] T026 [US1] 在 `src/supervision/event.rs` 中发送 `ChildStarting`, `ChildRunning` 和 `ChildReady` 事件.
- [ ] T027 [US1] 在 `src/supervision/snapshot.rs` 中实现 running-child(运行中子任务) 和 ready-child(已就绪子任务) 最新快照输出.
- [ ] T028 [US1] 在 `src/supervision/mod.rs` 中只暴露项目自有的用户故事一公开类型.

**Checkpoint(检查点)**: 用户故事一可以通过 `cargo test --test supervisor_lifecycle` 和 `cargo test --test supervisor_readiness` 独立运行.

---

## Phase 4(阶段四): User Story 2(用户故事二) - 构建监督树 (Priority(优先级): P2)

**Goal(目标)**: root supervisor(根监督器) 可以包含子 supervisor(监督器) 和 worker(工作任务), 保持定义顺序, 并在快照和事件中暴露稳定路径.

**Independent Test(独立测试)**: 构建包含嵌套 supervisor(监督器) 和 worker(工作任务) 的 root supervisor(根监督器), 启动它, 并断言路径, 父子关系和定义顺序.

### Tests for User Story 2(用户故事二的测试)

- [ ] T029 [US2] 在 `tests/supervisor_tree.rs` 中添加 supervisor tree(监督树) 启动, 路径和快照测试.
- [ ] T030 [P] [US2] 在 `tests/supervisor_tree.rs` 中添加 parent/child path(父子路径) 帮助函数外部单元测试, 覆盖 `src/supervision/id.rs`.

### Implementation for User Story 2(用户故事二的实现)

- [ ] T031 [P] [US2] 在 `src/supervision/tree.rs` 中实现 `SupervisorTree`(监督树) 和嵌套 supervisor spec(监督器规格).
- [ ] T032 [P] [US2] 在 `src/supervision/id.rs` 中实现稳定路径构造, parent lookup(父级查找) 和 child path joining(子路径拼接).
- [ ] T033 [P] [US2] 在 `src/supervision/registry.rs` 中扩展 nested supervisor(嵌套监督器) 和 worker node(工作节点).
- [ ] T034 [US2] 在 `src/supervision/runtime.rs` 中实现按定义顺序启动树.
- [ ] T035 [US2] 在 `src/supervision/event.rs` 中把 parent id(父标识), child id(子任务标识) 和 supervisor path(监督器路径) 加入事件位置数据.
- [ ] T036 [US2] 在 `src/supervision/snapshot.rs` 中扩展快照输出, 加入树结构和父子关系.

**Checkpoint(检查点)**: 用户故事二可以通过 `cargo test --test supervisor_tree` 独立运行.

---

## Phase 5(阶段五): User Story 3(用户故事三) - 应用重启, 退避和熔断策略 (Priority(优先级): P3)

**Goal(目标)**: policy engine(策略引擎) 可以分类退出, 应用 restart policy(重启策略), backoff(退避), strategy scope(策略范围), child quarantine(子任务隔离) 和 supervisor meltdown(监督器熔断).

**Independent Test(独立测试)**: 使用 paused time(暂停时间) 驱动失败, 并断言 restart decision(重启决策), quarantine(隔离), meltdown escalation(熔断升级) 和不同策略的 restart scope(重启范围).

### Tests for User Story 3(用户故事三的测试)

- [ ] T037 [US3] 在 `tests/supervisor_policy.rs` 中添加 restart policy(重启策略) 和 restart decision(重启决策) 测试.
- [ ] T038 [US3] 在 `tests/supervisor_policy.rs` 中添加 panic restart(恐慌重启), quarantine(隔离), meltdown(熔断), `OneForAll`(一对全部) 和 `RestForOne`(从失败处开始) paused-time(暂停时间) 测试.
- [ ] T039 [P] [US3] 在 `tests/supervisor_policy.rs` 中添加 exponential backoff(指数退避), jitter disabled(关闭抖动) 和 reset-after(稳定后重置) 外部单元测试, 覆盖 `src/supervision/backoff/mod.rs`.

### Implementation for User Story 3(用户故事三的实现)

- [ ] T040 [P] [US3] 在 `src/supervision/policy.rs` 中实现 `SupervisionStrategy`, `RestartPolicy`, `RestartDecision` 和 `MeltdownPolicy`.
- [ ] T041 [P] [US3] 在 `src/supervision/backoff/mod.rs` 中实现 exponential backoff(指数退避), jitter(抖动), 关闭 jitter(抖动) 和 reset-after(稳定后重置) 行为.
- [ ] T042 [US3] 在 `src/supervision/policy.rs` 中实现 child-level quarantine(子任务级隔离) 和 supervisor-level meltdown(监督器级熔断) 计数器.
- [ ] T043 [P] [US3] 在 `src/supervision/child_runner.rs` 中实现从任务结果, 取消, 超时和 panic(恐慌) 得到 `TaskExit`(任务退出) 分类.
- [ ] T044 [P] [US3] 在 `src/supervision/tree.rs` 中实现 `OneForOne`(一对一), `OneForAll`(一对全部) 和 `RestForOne`(从失败处开始) restart scope(重启范围) 选择.
- [ ] T045 [US3] 在 `src/supervision/event.rs` 中发送 `ChildPanicked`, `BackoffScheduled`, `ChildRestarting`, `ChildRestarted`, `ChildQuarantined` 和 `Meltdown` 事件.
- [ ] T046 [US3] 在 `src/supervision/child_runner.rs` 中更新 child restart loop(子任务重启循环), 使它使用策略决定.

**Checkpoint(检查点)**: 用户故事三可以通过 `cargo test --test supervisor_policy` 独立运行.

---

## Phase 6(阶段六): User Story 4(用户故事四) - 治理健康状态和运行时控制 (Priority(优先级): P4)

**Goal(目标)**: `SupervisorHandle`(监督器句柄) 支持幂等运行时控制命令和基于 heartbeat(心跳) 的健康检测.

**Independent Test(独立测试)**: 对同一个 child(子任务) 重复 pause(暂停), resume(恢复), shutdown(关闭) 和 quarantine(隔离), 并停止 heartbeat(心跳) 后断言 unhealthy(不健康) 处理.

### Tests for User Story 4(用户故事四的测试)

- [ ] T047 [US4] 在 `tests/supervisor_control.rs` 中添加 idempotent control command(幂等控制命令) 测试.
- [ ] T048 [P] [US4] 在 `tests/supervisor_control.rs` 中添加 heartbeat stale detection(心跳过期检测) 和 unhealthy policy(不健康策略) 测试.

### Implementation for User Story 4(用户故事四的实现)

- [ ] T049 [P] [US4] 在 `src/supervision/health.rs` 中实现 `HealthPolicy`, `Heartbeat`, 最新 heartbeat(心跳) 和 stale detection(过期检测).
- [ ] T050 [P] [US4] 在 `src/supervision/control.rs` 中实现 `ControlCommand`, command result(命令结果) 类型和 `SupervisorHandle` 命令 API(接口).
- [ ] T051 [US4] 在 `src/supervision/runtime.rs` 中实现 `add_child`, `remove_child`, `restart_child`, `pause_child`, `resume_child` 和 `quarantine_child` 派发.
- [ ] T052 [P] [US4] 在 `src/supervision/event.rs` 中实现 command audit event(命令审计事件) 映射.
- [ ] T053 [US4] 在 `src/supervision/registry.rs` 和 `src/supervision/snapshot.rs` 中更新 paused(已暂停), resumed(已恢复), unhealthy(不健康) 和 quarantined(已隔离) 运行状态.
- [ ] T054 [US4] 在 `src/supervision/control.rs` 中把 runtime event subscription(运行时事件订阅) 接入 `SupervisorHandle::subscribe_events`.

**Checkpoint(检查点)**: 用户故事四可以通过 `cargo test --test supervisor_control` 独立运行.

---

## Phase 7(阶段七): User Story 5(用户故事五) - 关闭时不留下孤儿任务 (Priority(优先级): P5)

**Goal(目标)**: root shutdown(根关闭) 使用父到子取消, graceful timeout(优雅关闭超时), abort fallback(强制终止回退), task draining(任务排空), reverse-order shutdown(逆序关闭), four-stage reconcile(四阶段对账) 和 no-orphan verification(无孤儿任务验证).

**Independent Test(独立测试)**: 启动多个长运行 child(子任务) 和一个 blocking child(阻塞子任务), 请求 root shutdown(根关闭), 断言所有 token(令牌) 取消, 阶段按顺序执行, child(子任务) 逆序关闭, blocking boundary(阻塞边界) 被记录, 并且 supervisor(监督器) 不再拥有任何任务.

### Tests for User Story 5(用户故事五的测试)

- [ ] T055 [US5] 在 `tests/supervisor_shutdown.rs` 中添加 four-stage shutdown(四阶段关闭), reverse-order shutdown(逆序关闭), reconcile(状态对账) 和 no-orphan(无孤儿任务) 集成测试.
- [ ] T056 [P] [US5] 在 `tests/supervisor_shutdown.rs` 中添加 cancellation propagation(取消传播) 外部单元测试, 覆盖 `src/supervision/shutdown.rs`.
- [ ] T057 [P] [US5] 在 `tests/supervisor_blocking.rs` 中添加 `spawn_blocking`(阻塞任务启动) 关闭超时, 不可立即终止事件和升级策略测试.

### Implementation for User Story 5(用户故事五的实现)

- [ ] T058 [P] [US5] 在 `src/supervision/shutdown.rs` 中实现 `ShutdownPolicy`, `ShutdownPhase`, shutdown cause(关闭原因), graceful timeout(优雅关闭超时), abort wait(强制终止等待) 和四阶段状态机.
- [ ] T059 [P] [US5] 在 `src/supervision/task.rs` 中把 `CancellationToken`(取消令牌) 接入 `TaskContext`(任务上下文), child token(子令牌) 和 blocking boundary(阻塞边界) 报告.
- [ ] T060 [P] [US5] 在 `src/supervision/spec.rs` 中实现 `TaskKind`(任务类型), async worker(异步工作任务), blocking worker(阻塞工作任务) 和 blocking shutdown policy(阻塞关闭策略).
- [ ] T061 [US5] 在 `src/supervision/runtime.rs` 中实现 `JoinSet`(任务集合) 任务所有权, reverse-order shutdown(逆序关闭) 和 draining(排空).
- [ ] T062 [US5] 在 `src/supervision/control.rs` 和 `src/supervision/shutdown.rs` 中实现 `shutdown_tree` 四阶段控制流.
- [ ] T063 [US5] 在 `src/supervision/event.rs` 和 `src/supervision/snapshot.rs` 中发送 `ShutdownRequested`, `ShutdownPhaseChanged` 和 `ShutdownCompleted` 事件, 并输出 reconcile(状态对账) 后的终态快照.
- [ ] T064 [US5] 在 `src/supervision/child_runner.rs` 中实现 blocking task(阻塞任务) 关闭超时边界, 不可立即终止事件和升级策略.

**Checkpoint(检查点)**: 用户故事五可以通过 `cargo test --test supervisor_shutdown` 和 `cargo test --test supervisor_blocking` 独立运行.

---

## Phase 8(阶段八): User Story 6(用户故事六) - 观察, 审计并回放生命周期 (Priority(优先级): P6)

**Goal(目标)**: supervisor(监督器) 暴露最新快照, 完整生命周期事件, fixed-capacity event journal(固定容量事件日志缓冲区), `RunSummary`(运行摘要), tracing span/event(追踪范围和事件), metrics(指标), audit event(审计事件) 和可序列化观察数据.

**Independent Test(独立测试)**: 驱动 child(子任务) 经历启动, heartbeat(心跳), 失败, backoff(退避), 重启, quarantine(隔离), meltdown(熔断) 和 shutdown(关闭), 并断言事件, 快照, event journal(事件日志缓冲区), `RunSummary`(运行摘要), 指标, tracing(结构化追踪) 和审计一致.

### Tests for User Story 6(用户故事六的测试)

- [ ] T065 [US6] 在 `tests/supervisor_observe.rs` 中添加 `When`(何时), `Where`(何处), `What`(发生内容) 事件形状测试.
- [ ] T066 [US6] 在 `tests/supervisor_observe.rs` 中添加 tracing(结构化追踪), metrics(指标), event lag(事件滞后) 和 audit(审计) 断言测试.
- [ ] T067 [P] [US6] 在 `tests/supervisor_api.rs` 中添加 snapshot(快照), event(事件) 和 audit command(审计命令) 的 serialization contract(序列化契约) 测试.
- [ ] T068 [P] [US6] 在 `tests/supervisor_diagnostics.rs` 中添加 event journal(事件日志缓冲区) 和 `RunSummary`(运行摘要) 测试, 覆盖 meltdown(熔断), 关闭超时和父级升级.
- [ ] T069 [P] [US6] 在 `tests/supervisor_observe.rs` 中添加 metrics label(指标标签) 低基数验证测试, 拒绝错误全文, 用户输入和无界动态值.

### Implementation for User Story 6(用户故事六的实现)

- [ ] T070 [P] [US6] 在 `src/supervision/event.rs` 中实现 `SupervisorEvent`, `EventTime`, `EventLocation`, `EventPayload` 和 policy decision payload(策略决定内容).
- [ ] T071 [P] [US6] 在 `src/supervision/observe.rs` 中实现 event bus fan-out(事件总线扇出), subscriber lag accounting(订阅者滞后计数) 和 event collection hook(事件收集钩子).
- [ ] T072 [P] [US6] 在 `src/supervision/snapshot.rs` 中实现 watch-style latest snapshot store(观察式最新快照存储).
- [ ] T073 [P] [US6] 在 `src/supervision/journal.rs` 中实现 fixed-capacity event journal(固定容量事件日志缓冲区), dropped count(丢弃计数) 和最近事件查询.
- [ ] T074 [P] [US6] 在 `src/supervision/summary.rs` 中实现 `RunSummary`(运行摘要), 并从 event journal(事件日志缓冲区), snapshot(快照) 和策略决定生成诊断摘要.
- [ ] T075 [US6] 在 `src/supervision/observe.rs` 和 `src/supervision/child_runner.rs` 中实现每个 child attempt(子任务尝试) 的 tracing span(追踪范围), 以及每次状态迁移的 tracing event(追踪事件).
- [ ] T076 [US6] 在 `src/supervision/observe.rs` 中实现必需 metrics facade(指标门面) 输出和 low-cardinality label validator(低基数标签校验器).
- [ ] T077 [US6] 在 `src/supervision/control.rs` 和 `src/supervision/event.rs` 中实现 command audit event(命令审计事件) 序列化.
- [ ] T078 [US6] 在 `src/supervision/snapshot.rs`, `src/supervision/event.rs`, `src/supervision/control.rs`, `src/supervision/error.rs`, `src/supervision/journal.rs` 和 `src/supervision/summary.rs` 中为 snapshot(快照), event(事件), audit(审计), failure model(失败模型), event journal(事件日志缓冲区) 和 run summary(运行摘要) 添加 serde(序列化) 支持.
- [ ] T079 [US6] 在 `examples/supervisor_quickstart.rs` 中添加一个匹配 `specs/001-create-supervisor-core/quickstart.md` 的最小 quickstart(快速开始) 示例.

**Checkpoint(检查点)**: 用户故事六可以通过 `cargo test --test supervisor_observe`, `cargo test --test supervisor_diagnostics` 和 `cargo test --test supervisor_api` 独立运行.

---

## Phase 9(阶段九): Polish & Cross-Cutting Concerns(收尾和横向事项)

**Purpose(目的)**: 验证完整功能, 对齐最终文档名称, 并确保没有引入禁止的依赖, 内联测试或兼容表面.

- [ ] T080 [P] 在 `specs/001-create-supervisor-core/quickstart.md` 中更新最终 quickstart(快速开始) API(接口) 名称.
- [ ] T081 [P] 在 `specs/001-create-supervisor-core/contracts/public-api.md` 中更新最终公开 API(接口) 契约名称.
- [ ] T082 在 `src/supervision/mod.rs` 中为公开 supervision(监督) 类型添加 rustdoc(文档注释).
- [ ] T083 通过 `Cargo.toml` 运行 `cargo fmt --check` 格式检查.
- [ ] T084 通过 `Cargo.toml` 运行 `cargo check` 编译检查.
- [ ] T085 通过 `Cargo.toml` 运行 `cargo test` 完整测试.
- [ ] T086 运行 `specs/001-create-supervisor-core/quickstart.md` 中的 quickstart(快速开始) 验收命令.
- [ ] T087 验证 `Cargo.toml` 中没有 forbidden actor/supervisor compatibility dependency(禁止的参与者或监督器兼容依赖).
- [ ] T088 验证 `src/supervision/` 和 `examples/supervisor_quickstart.rs` 中没有把 business hot path(业务热路径) 或 data plane(数据面) 消息处理放入 supervisor(监督器).
- [ ] T089 验证 `src/` 中没有内联单元测试代码, 并确认所有测试都位于 `tests/` 目录.

---

## Dependencies & Execution Order(依赖和执行顺序)

### Phase Dependencies(阶段依赖)

- **Phase 1 Setup(阶段一初始化)**: 没有依赖.
- **Phase 2 Foundational(阶段二基础)**: 依赖 Phase 1(阶段一), 并阻塞所有用户故事.
- **US1(用户故事一) 声明并运行子任务**: 依赖 Phase 2(阶段二). 这是 MVP(最小可用产品).
- **US2(用户故事二) 构建监督树**: 依赖 Phase 2(阶段二), 并且可以在 US1(用户故事一) 的公开 spec/task(规格和任务) 原语存在后开始.
- **US3(用户故事三) 应用重启, 退避和熔断策略**: 依赖 Phase 2(阶段二), 并使用 US1(用户故事一) 的 child runner(子任务运行器) 原语和 US2(用户故事二) 的 restart scope(重启范围) 帮助函数.
- **US4(用户故事四) 治理健康状态和运行时控制**: 依赖 Phase 2(阶段二) 和 US1(用户故事一) 的 runtime handle(运行时句柄) 形状.
- **US5(用户故事五) 关闭时不留下孤儿任务**: 依赖 Phase 2(阶段二), US1(用户故事一) 的 runtime ownership(运行时所有权) 形状, 以及 US2(用户故事二) 的定义顺序.
- **US6(用户故事六) 观察, 审计并回放生命周期**: 依赖 Phase 2(阶段二), 并整合 US1 到 US5 的事件点.
- **Phase 9 Polish(阶段九收尾)**: 依赖所有选定故事.

### User Story Dependencies(用户故事依赖)

- **US1(用户故事一)**: Phase 2(阶段二) 后可以独立实现 MVP(最小可用产品).
- **US2(用户故事二)**: Phase 2(阶段二) 后可以实现, 但最终树启动会使用 US1(用户故事一) 的 runtime launch(运行时启动).
- **US3(用户故事三)**: Phase 2(阶段二) 后可以实现, 但最终 restart loop(重启循环) 会集成 US1(用户故事一) 的 child runner(子任务运行器) 和 US2(用户故事二) 的 strategy scope(策略范围).
- **US4(用户故事四)**: Phase 2(阶段二) 后可以实现, 但最终 handle command(句柄命令) 会集成 US1(用户故事一) 的 runtime(运行时).
- **US5(用户故事五)**: Phase 2(阶段二) 后可以实现, 但最终 shutdown(关闭) 会集成 US1(用户故事一) 的 runtime ownership(运行时所有权) 和 US2(用户故事二) 的定义顺序.
- **US6(用户故事六)**: 它观察所有较早生命周期行为, 最好在 US1 到 US5 的事件点存在后完成.

### Within Each User Story(每个用户故事内部)

- 先写外部失败测试.
- 先实现模型类型, 再实现运行时行为.
- 先实现运行时行为, 再做集成和公开 API(接口).
- 每个检查点都运行该故事对应的测试命令.
- `[P]` 任务必须修改不同文件, 同文件任务不得并行.

---

## Parallel Execution Examples(并行执行示例)

### Foundational(阶段二基础)

```bash
Task(任务): "T010 在 src/supervision/id.rs 中实现身份和路径类型"
Task(任务): "T011 在 src/supervision/error.rs 中实现错误类型"
Task(任务): "T012 在 src/supervision/event.rs 中实现基础事件序号"
Task(任务): "T014 在 src/supervision/test_support.rs 中实现测试支持"
```

### User Story 1(用户故事一)

```bash
Task(任务): "T021 在 src/supervision/spec.rs 中实现 ChildSpec 和 readiness policy 字段"
Task(任务): "T022 在 src/supervision/task.rs 中实现 TaskContext 和 readiness reporter"
Task(任务): "T023 在 src/supervision/readiness.rs 中实现 ReadinessPolicy"
```

### User Story 5(用户故事五)

```bash
Task(任务): "T058 在 src/supervision/shutdown.rs 中实现四阶段关闭状态机"
Task(任务): "T059 在 src/supervision/task.rs 中接入取消令牌和阻塞边界报告"
Task(任务): "T060 在 src/supervision/spec.rs 中实现 TaskKind 和阻塞关闭策略"
```

### User Story 6(用户故事六)

```bash
Task(任务): "T070 在 src/supervision/event.rs 中实现事件模型"
Task(任务): "T071 在 src/supervision/observe.rs 中实现事件扇出和滞后计数"
Task(任务): "T073 在 src/supervision/journal.rs 中实现事件日志缓冲区"
Task(任务): "T074 在 src/supervision/summary.rs 中实现运行摘要"
```

---

## Implementation Strategy(实现策略)

### MVP First(先做最小可用产品)

1. 完成 Phase 1(阶段一) 和 Phase 2(阶段二).
2. 只完成 US1(用户故事一).
3. 使用 `cargo test --test supervisor_lifecycle` 和 `cargo test --test supervisor_readiness` 验证.
4. 验证 quickstart(快速开始) 形状可以声明并运行一个受监督 worker(工作任务), 并能处理 readiness(就绪).

### Incremental Delivery(增量交付)

1. US1(用户故事一) 交付 child declaration(子任务声明), fresh task factory(新任务工厂), startup event(启动事件), readiness(就绪) 和 snapshot(快照).
2. US2(用户故事二) 增加 tree boundary(树边界) 和 stable path(稳定路径).
3. US3(用户故事三) 增加 restart(重启), backoff(退避), quarantine(隔离), meltdown(熔断) 和 strategy semantic(策略语义).
4. US4(用户故事四) 增加 runtime control(运行时控制) 和 heartbeat health(心跳健康).
5. US5(用户故事五) 增加 four-stage shutdown(四阶段关闭), blocking task(阻塞任务) 边界和 no-orphan guarantee(无孤儿任务保证).
6. US6(用户故事六) 增加 full observability(完整可观察性), audit(审计), metrics(指标), event journal(事件日志缓冲区), RunSummary(运行摘要) 和 replayable event(可回放事件).

### Final Validation(最终验证)

1. `cargo fmt --check`
2. `cargo check`
3. `cargo test`
4. 运行 `specs/001-create-supervisor-core/quickstart.md` 中记录的命令.
5. 确认 `src/` 中没有内联单元测试代码.

## Notes(说明)

- 总任务数: 89.
- 用户故事任务数: US1 12 个, US2 8 个, US3 10 个, US4 8 个, US5 10 个, US6 15 个.
- MVP(最小可用产品) 范围: Phase 1(阶段一), Phase 2(阶段二) 和 US1(用户故事一).
- 所有实现任务都使用项目自有 `src/supervision/` 路径.
- 所有测试任务都使用外部 `tests/` 路径.
- 没有任务引入 actor framework(参与者框架), 把 `supertrees` 作为核心依赖, 或为参考 crate(库) 添加 compatibility exposure(兼容暴露).
