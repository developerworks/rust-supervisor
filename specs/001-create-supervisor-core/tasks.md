# Tasks(任务): 创建监督器核心

**Input(输入)**: 设计文档来自 `specs/001-create-supervisor-core/`.
**Prerequisites(前置文档)**: `plan.md`,`spec.md`,`research.md`,`data-model.md`,`contracts/public-api.md`,`quickstart.md`,`glossary.md`.
**Tests(测试)**: 行为变化必须先有测试任务,再有实现任务.本功能明确要求测试优先.
**Organization(组织方式)**: 任务按 User Story(用户故事) 分组,每个故事都必须独立实现和独立测试.
**Source Layout(源码布局)**: 核心源码必须使用 `src/<module>/` top-level directory module(顶层目录模块),不得使用 `src/supervision/`,不得使用 `src/<module>.rs` 平铺模块文件.

## Format(格式): `[ID] [P?] [Story] Description(描述)`

- `[P]` 表示任务可以并行执行,因为它修改不同文件或不同目录,并且不依赖未完成任务.
- `[US1]` 到 `[US8]` 表示任务所属用户故事.
- 每个任务都写出明确文件路径.
- 每个任务只能归属一个 primary workstream(主工作流).跨工作流协作只能写入 dependency(依赖),review gate(审查门禁) 或 handoff note(交接说明),不得重复分配同一个任务.
- 每个源码实现任务必须在自己的 file boundary(文件边界) 内同步补齐英文 module doc(模块文档),struct doc(结构体文档),field doc(字段文档),function doc(函数文档),source comment(源码注释) 和 public doctest(公共文档测试),不得把所有源码文档集中到一个后置大任务.
- 所有测试文件必须以 `_test.rs` 结尾.
- integration test(集成测试) 放在 `src/tests/*_test.rs`.
- unit test(单元测试) 放在 `src/<module>/tests/*_test.rs`.
- `src/lib.rs` 只包含 crate doc(包文档) 和顶层 `pub mod <mod_name>;` 声明.
- 每个 `src/<module>/mod.rs` 只包含 `pub mod <mod_name>;` 声明.
- 内部导入必须使用 `crate::` absolute path(绝对路径),不得使用 `super::` relative path(相对路径).

## Phase 1(阶段一): Setup(共享基础设施)

**Purpose(目的)**: 建立顶层目录模块结构,依赖,配置样例和质量脚本入口.

- [X] T001 更新 `Cargo.toml` 和 `Cargo.lock`,加入 `rust-config-tree` v0.1.9,`tokio`,`tokio-util`,`tracing`,`tracing-subscriber`,`metrics`,`serde`,`serde_json`,`serde_yaml`,`thiserror`,`uuid` 和 `rand`,覆盖 FR-050.
- [X] T002 创建顶层模块目录 `src/id/`,`src/error/`,`src/config/`,`src/spec/`,`src/task/`,`src/tree/`,`src/policy/`,`src/readiness/`,`src/health/`,`src/control/`,`src/registry/`,`src/runtime/`,`src/child_runner/`,`src/event/`,`src/state/`,`src/journal/`,`src/summary/`,`src/observe/`,`src/shutdown/` 和 `src/test_support/`,覆盖 FR-077.
- [X] T003 创建每个顶层模块的 `src/<module>/mod.rs` 和 `src/<module>/tests/` 目录,并确保每个 `mod.rs` 只包含 `pub mod <mod_name>;` 声明,覆盖 FR-056 和 FR-064.
- [X] T004 更新 `src/lib.rs`,只保留 crate doc(包文档) 和顶层 `pub mod <mod_name>;` 声明,覆盖 FR-056 和 SC-024.
- [X] T005 [P] 创建 `examples/config/supervisor.yaml`,作为 rust-config-tree(集中配置树) v0.1.9 的 YAML(数据序列化格式) 主配置样例,覆盖 FR-050 和 FR-065.
- [X] T006 [P] 创建质量脚本入口 `scripts/check-coding-standard.sh`,`scripts/check-maintainability.sh`,`scripts/generate-sbom.sh` 和 `scripts/validate-sbom.sh`,覆盖 FR-058,FR-059,FR-060,FR-061 和 FR-062.
- [X] T007 [P] 创建双语文档目录 `manual/zh/`,`manual/en/`,`docs/zh/` 和 `docs/en/`,覆盖 FR-053.
- [X] T008 [P] 创建发布文档入口 `README.md`,`CHANGELOG.md` 和 `LICENSE`,覆盖 FR-058.

---

## Phase 2(阶段二): Foundational(阻塞前置基础)

**Purpose(目的)**: 完成所有用户故事共享的契约类型,错误边界,事件基础,配置入口,测试支持和质量门禁.

**Critical(关键要求)**: 本阶段完成前,任何用户故事实现都不能开始.

### Foundational Tests(基础测试)

- [X] T009 [P] 在 `src/tests/source_layout_test.rs` 中添加 source layout check(源码布局检查),验证核心模块位于 `src/<module>/`,不存在 `src/supervision/` 和 `src/<module>.rs`,覆盖 FR-077 和 SC-045.
- [X] T010 [P] 在 `src/tests/module_boundary_test.rs` 中添加 `src/lib.rs` 和 `src/<module>/mod.rs` 入口规则测试,验证没有 `pub use`(公开重导出),类型定义,函数定义,常量定义或逻辑,覆盖 FR-056 和 SC-024.
- [X] T011 [P] 在 `src/tests/import_rule_test.rs` 中添加 absolute import(绝对导入) 检查,拒绝 `super::`,覆盖 FR-057 和 SC-025.
- [X] T012 [P] 在 `src/tests/naming_contract_test.rs` 中添加 naming check(命名检查) 和 public model terminology check(公开模型术语检查),拒绝 `*Snapshot`,`*View`,`snapshot()`,`state_view` 和 actor-model(参与者模型) 公开术语,覆盖 FR-038,FR-063,SC-012 和 SC-031.
- [X] T013 [P] 在 `src/tests/module_dependency_test.rs` 中添加 module dependency map(模块依赖图) 无循环依赖测试,覆盖 FR-068,FR-069 和 SC-036.
- [X] T014 [P] 在 `src/tests/config_boundary_test.rs` 中添加 hard-coded constant check(硬编码常量检查),拒绝生产运行时可调常量的硬编码默认值,覆盖 FR-051,FR-067 和 SC-035.
- [X] T015 [P] 在 `src/tests/glossary_coverage_test.rs` 中添加 glossary coverage check(词汇表覆盖检查),覆盖专业词汇和反引号词汇,覆盖 FR-066 和 SC-034.

### Foundational Implementation(基础实现)

- [X] T016 [P] 在 `src/id/types.rs` 中实现 `ChildId`,`SupervisorId`,`SupervisorPath`,`Generation` 和 `Attempt`,覆盖 FR-004 和 FR-006.
- [X] T017 [P] 在 `src/error/types.rs` 中实现 `SupervisorError`,`TaskFailure` 和 `TaskFailureKind`,覆盖 FR-010 和 FR-011.
- [X] T018 [P] 在 `src/event/time.rs` 中实现 event sequence(事件序号),correlation id(关联标识) 和 `When`(何时) 基础类型,覆盖 FR-028 和 FR-029.
- [X] T019 [P] 在 `src/event/payload.rs` 中实现 `Where`(何处),`What`(发生内容) 和基础 `SupervisorEvent`(监督器事件) 类型,覆盖 FR-028,FR-030 和 FR-031.
- [X] T020 [P] 在 `src/state/child.rs` 和 `src/state/supervisor.rs` 中实现 `ChildState`(子任务状态) 和 `SupervisorState`(监督器状态) 基础模型,覆盖 FR-025,FR-026 和 SC-009.
- [X] T021 [P] 在 `src/spec/supervisor.rs` 中实现唯一 `SupervisionStrategy`(监督策略),并在 `src/policy/decision.rs` 中实现 `RestartPolicy`,`RestartDecision` 和 `TaskFailureKind` 到策略输入的基础类型,覆盖 FR-007,FR-008,FR-009 和 FR-012.
- [X] T022 [P] 在 `src/config/state.rs` 和 `src/config/loader.rs` 中实现 `SupervisorConfig`(监督器配置),`ConfigState`(配置状态) 和配置版本基础结构,覆盖 FR-050.
- [X] T023 [P] 在 `src/test_support/assertions.rs` 和 `src/test_support/factory.rs` 中实现 paused time(暂停时间),event collection(事件收集) 和 deterministic jitter(确定性抖动) 支持,覆盖 FR-017,FR-036 和 SC-010.
- [X] T024 在 `src/tests/foundational_gate_test.rs` 中串联基础质量门禁,确认 source layout(源码布局),module boundary(模块边界),import rule(导入规则),naming(命名),configuration boundary(配置边界) 和 glossary(词汇表) 全部可独立运行,覆盖 FR-056,FR-057,FR-063,FR-066,FR-067,FR-077,SC-024,SC-025,SC-031,SC-034,SC-035 和 SC-045.

**Checkpoint(检查点)**: 基础契约,源码布局和质量门禁已经可用,用户故事实现可以开始.

---

## Phase 3(阶段三): User Story 1(用户故事一) - 声明并运行子任务 (Priority(优先级): P1) MVP(最小可用产品)

**Goal(目标)**: 维护者可以声明 `ChildSpec`(子任务规格),通过 `TaskFactory`(任务工厂) 启动 child(子任务),并看到 running(运行中) 和 ready(已就绪) 状态.

**Independent Test(独立测试)**: 创建一个 worker(工作任务),启动 supervisor(监督器),验证 `ChildStarting`,`ChildRunning`,`ChildReady` 事件和 `current_state`(当前状态).

### Tests for User Story 1(用户故事一的测试)

- [X] T025 [P] [US1] 在 `src/spec/tests/spec_test.rs` 中添加 `ChildSpec` 和 `SupervisorSpec` 字段校验 unit test(单元测试),覆盖 FR-001.
- [X] T026 [P] [US1] 在 `src/task/tests/task_test.rs` 中添加 `TaskFactory`,`TaskContext`,`TaskResult`,`Service trait` 和 `service_fn` 测试,覆盖 FR-003,FR-004 和 FR-048.
- [X] T027 [P] [US1] 在 `src/readiness/tests/readiness_test.rs` 中添加 immediate readiness(立即就绪) 和 explicit readiness(显式就绪) 测试,覆盖 FR-043 和 SC-014.
- [X] T028 [P] [US1] 在 `src/tests/supervisor_start_test.rs` 中添加声明并运行子任务 integration test(集成测试),覆盖 FR-001,FR-002,FR-025 和 SC-001.

### Implementation for User Story 1(用户故事一的实现)

- [X] T029 [P] [US1] 在 `src/spec/child.rs` 中实现 `ChildSpec`,`TaskKind`,dependencies(依赖),tags(标签),criticality(关键程度) 和校验规则,覆盖 FR-001.
- [X] T030 [P] [US1] 在 `src/spec/supervisor.rs` 中实现 `SupervisorSpec` 和 root supervisor(根监督器) 基础声明模型,覆盖 FR-005.
- [X] T031 [P] [US1] 在 `src/task/context.rs` 中实现 `TaskContext`,cancellation token(取消令牌),heartbeat(心跳) 和 readiness(就绪) 接口,覆盖 FR-004.
- [X] T032 [P] [US1] 在 `src/task/factory.rs` 中实现 `TaskFactory`,boxed task future(装箱任务异步值),`Service trait` 和 `service_fn`,覆盖 FR-003 和 FR-048.
- [X] T033 [P] [US1] 在 `src/readiness/signal.rs` 中实现 `ReadinessPolicy` 和 ready(已就绪) 信号发送,覆盖 FR-043.
- [X] T034 [US1] 在 `src/registry/entry.rs` 和 `src/registry/store.rs` 中实现 single-child registry(单子任务注册表) 和 `ChildRuntime`(子任务运行态) 最小状态,覆盖 FR-025.
- [X] T035 [US1] 在 `src/child_runner/runner.rs` 中实现一次 worker(工作任务) 启动,结果接收和 readiness(就绪) 状态推进,覆盖 FR-002 和 FR-003.
- [X] T036 [US1] 在 `src/runtime/supervisor.rs` 中实现 `Supervisor::start` 最小启动路径和 `SupervisorHandle`(监督器句柄) 返回,覆盖 FR-023.
- [X] T037 [US1] 在 `src/event/payload.rs` 中发送 `ChildStarting`,`ChildRunning` 和 `ChildReady` 事件,覆盖 FR-028.
- [X] T038 [US1] 在 `src/state/supervisor.rs` 中实现 `current_state` 查询的 running(运行中) 和 ready(已就绪) 输出,覆盖 FR-025 和 SC-009.

**Checkpoint(检查点)**: 用户故事一已经完整可用,并且可以独立测试.

---

## Phase 4(阶段四): User Story 2(用户故事二) - 构建监督树 (Priority(优先级): P2)

**Goal(目标)**: 维护者可以构建包含 root supervisor(根监督器),子 supervisor(监督器) 和 worker(工作任务) 的 `SupervisorTree`(监督树).

**Independent Test(独立测试)**: 创建嵌套监督树,验证父子关系,稳定路径,定义顺序和逆序关闭顺序.

### Tests for User Story 2(用户故事二的测试)

- [X] T039 [P] [US2] 在 `src/tree/tests/tree_test.rs` 中添加 parent-child path(父子路径),定义顺序和 nested supervisor(嵌套监督器) 测试,覆盖 FR-005 和 FR-006.
- [X] T040 [P] [US2] 在 `src/registry/tests/registry_test.rs` 中添加树节点注册,路径索引和定义顺序测试,覆盖 FR-005.
- [X] T041 [P] [US2] 在 `src/tests/supervisor_tree_test.rs` 中添加监督树 integration test(集成测试),覆盖 SC-006,SC-007 和 SC-013.

### Implementation for User Story 2(用户故事二的实现)

- [X] T042 [P] [US2] 在 `src/tree/builder.rs` 中实现 `SupervisorTree` 构建,嵌套 supervisor spec(监督器规格) 和路径校验,覆盖 FR-005.
- [X] T043 [P] [US2] 在 `src/tree/order.rs` 中实现按声明顺序启动和按声明顺序逆序关闭的遍历工具,覆盖 FR-042.
- [X] T044 [P] [US2] 在 `src/id/types.rs` 中扩展 `SupervisorPath` 的 parent lookup(父级查找),child path joining(子路径拼接) 和路径校验,覆盖 FR-006.
- [X] T045 [US2] 在 `src/registry/store.rs` 中扩展 nested supervisor(嵌套监督器) 和 worker node(工作节点) 注册,覆盖 FR-005.
- [X] T046 [US2] 在 `src/runtime/supervisor.rs` 中实现按定义顺序启动监督树,覆盖 FR-042.
- [X] T047 [US2] 在 `src/event/payload.rs` 中把 parent id(父标识),child id(子任务标识) 和 supervisor path(监督器路径) 加入 `Where`(何处),覆盖 FR-030.
- [X] T048 [US2] 在 `src/state/supervisor.rs` 中扩展当前状态输出,加入树结构和父子关系,覆盖 FR-025 和 SC-009.

**Checkpoint(检查点)**: 用户故事二可以独立验证树结构和路径定位.

---

## Phase 5(阶段五): User Story 3(用户故事三) - 应用重启,退避和熔断策略 (Priority(优先级): P3)

**Goal(目标)**: 策略引擎可以根据退出原因,失败类别,重启策略,退避策略和熔断策略做出明确决定.

**Independent Test(独立测试)**: 让 child(子任务) 正常退出,失败,panic(恐慌),timeout(超时) 和 unhealthy(不健康),验证不重启,延迟重启,隔离,父级升级和关闭整棵树.

### Tests for User Story 3(用户故事三的测试)

- [X] T049 [P] [US3] 在 `src/policy/tests/policy_test.rs` 中添加 `RestartPolicy`,`RestartDecision` 和 policy engine(策略引擎) 测试,并在 `src/tests/module_boundary_test.rs` 中确认 `SupervisionStrategy`(监督策略) 只有一个源码定义,覆盖 FR-007,FR-008,FR-009 和 FR-012.
- [X] T050 [P] [US3] 在 `src/policy/tests/backoff_test.rs` 中添加 exponential backoff(指数退避),deterministic jitter(确定性抖动),disabled jitter(关闭抖动) 和 reset-after(稳定后重置) 测试,覆盖 FR-016 和 FR-017.
- [X] T051 [P] [US3] 在 `src/policy/tests/meltdown_test.rs` 中添加 child-level fuse(子任务级熔断) 和 supervisor-level fuse(监督器级熔断) 测试,覆盖 FR-013,FR-014 和 FR-015.
- [X] T052 [P] [US3] 在 `src/child_runner/tests/task_exit_test.rs` 中添加 `TaskExit`(任务退出) 分类测试,覆盖 FR-010 和 FR-011.
- [X] T053 [P] [US3] 在 `src/tests/supervisor_policy_test.rs` 中添加 panic restart(恐慌重启),quarantine(隔离),meltdown(熔断),`OneForAll` 和 `RestForOne` 集成测试,覆盖 SC-002,SC-003,SC-004,SC-006 和 SC-007.

### Implementation for User Story 3(用户故事三的实现)

- [X] T054 [P] [US3] 在 `src/spec/supervisor.rs` 中维护唯一 `SupervisionStrategy`(监督策略),并在 `src/policy/decision.rs` 中实现 `RestartPolicy`,`RestartDecision` 和 policy engine(策略引擎),覆盖 FR-007,FR-008 和 FR-012.
- [X] T055 [P] [US3] 在 `src/policy/backoff.rs` 中实现指数退避,jitter(抖动),关闭 jitter(抖动),确定性 jitter(抖动) 和 reset-after(稳定后重置),覆盖 FR-016 和 FR-017.
- [X] T056 [P] [US3] 在 `src/policy/meltdown.rs` 中实现 child-level fuse(子任务级熔断),supervisor-level fuse(监督器级熔断) 和计数器重置,覆盖 FR-013,FR-014 和 FR-015.
- [X] T057 [P] [US3] 在 `src/child_runner/attempt.rs` 中实现任务结果,取消,超时,unhealthy(不健康) 和 panic(恐慌) 到 `TaskExit` 的分类,覆盖 FR-010 和 FR-011.
- [X] T058 [P] [US3] 在 `src/tree/order.rs` 中实现 `OneForOne`,`OneForAll` 和 `RestForOne` restart scope(重启范围) 选择,覆盖 FR-007.
- [X] T058A [P] [US3] 在 `src/spec/supervisor.rs`,`src/tree/order.rs`,`src/runtime/control_loop.rs`,`src/tree/tests/tree_test.rs`,`src/control/tests/control_test.rs` 和 `src/tests/supervisor_auto_restart_test.rs` 中实现 group strategy(分组策略),dynamic supervisor(动态监督器),escalation policy(升级策略),per-child override(子任务级覆盖),restart limit(重启次数限制) 和 strategy execution plan(策略执行计划),覆盖 FR-007,FR-023 和 FR-070.
- [X] T059 [US3] 在 `src/event/payload.rs` 中发送 `ChildPanicked`,`BackoffScheduled`,`ChildRestarting`,`ChildRestarted`,`ChildQuarantined` 和 `Meltdown` 事件,覆盖 FR-028 和 SC-002.
- [X] T060 [US3] 在 `src/child_runner/runner.rs` 中更新 child restart loop(子任务重启循环),使它使用策略决定并保持 cognitive complexity(认知复杂度) 预算,覆盖 FR-060.

**Checkpoint(检查点)**: 用户故事三可以独立验证策略和重启行为.

---

## Phase 6(阶段六): User Story 4(用户故事四) - 治理健康状态和运行时控制 (Priority(优先级): P4)

**Goal(目标)**: 操作者可以通过 `SupervisorHandle` 执行幂等控制命令,并获得健康状态和审计事件.

**Independent Test(独立测试)**: 重复执行 shutdown(关闭),pause(暂停),resume(恢复) 和 quarantine(隔离),验证幂等结果和审计记录.

### Tests for User Story 4(用户故事四的测试)

- [X] T061 [P] [US4] 在 `src/control/tests/control_test.rs` 中添加 `ControlCommand`,command result(命令结果),幂等性和 audit payload(审计内容) 测试,覆盖 FR-023,FR-024 和 FR-037.
- [X] T062 [P] [US4] 在 `src/health/tests/health_test.rs` 中添加 `HealthPolicy`,`Heartbeat` 和 stale detection(过期检测) 测试,覆盖 FR-018 和 FR-019.
- [X] T063 [P] [US4] 在 `src/tests/supervisor_control_test.rs` 中添加 runtime control(运行时控制) 集成测试,覆盖 FR-023,FR-024 和 SC-011.

### Implementation for User Story 4(用户故事四的实现)

- [X] T064 [P] [US4] 在 `src/health/heartbeat.rs` 中实现 `HealthPolicy`,`Heartbeat`,最新 heartbeat(心跳) 和 stale detection(过期检测),覆盖 FR-018 和 FR-019.
- [X] T065 [P] [US4] 在 `src/control/command.rs` 中实现 `ControlCommand`,command id(命令标识),请求者,原因,目标路径和结果类型,覆盖 FR-037.
- [X] T066 [P] [US4] 在 `src/control/handle.rs` 中实现 `SupervisorHandle` 的 `add_child`,`remove_child`,`restart_child`,`pause_child`,`resume_child`,`quarantine_child`,`shutdown_tree`,`current_state` 和 `subscribe_events` 方法签名,覆盖 FR-023.
- [X] T067 [US4] 在 `src/runtime/control_loop.rs` 中实现运行时控制命令派发和幂等返回,覆盖 FR-024.
- [X] T068 [US4] 在 `src/event/payload.rs` 中实现 command audit event(命令审计事件) 映射,覆盖 FR-037.
- [X] T069 [US4] 在 `src/registry/store.rs` 和 `src/state/child.rs` 中更新 paused(已暂停),resumed(已恢复),unhealthy(不健康) 和 quarantined(已隔离) 状态,覆盖 FR-023 和 FR-024.
- [X] T070 [US4] 在 `src/control/handle.rs` 中把 runtime event subscription(运行时事件订阅) 接入 `SupervisorHandle::subscribe_events`,覆盖 FR-027.

**Checkpoint(检查点)**: 用户故事四可以独立验证运行时控制和健康治理.

---

## Phase 7(阶段七): User Story 5(用户故事五) - 关闭后不留下孤儿任务 (Priority(优先级): P5)

**Goal(目标)**: root shutdown(根关闭) 执行四阶段关闭,取消所有 child token(子令牌),排空任务集合,并处理 blocking task(阻塞任务) 边界.

**Independent Test(独立测试)**: 启动多个长运行 child(子任务) 后请求 root shutdown(根关闭),验证没有 orphan task(孤儿任务),并记录关闭事件和最终状态.

### Tests for User Story 5(用户故事五的测试)

- [X] T071 [P] [US5] 在 `src/shutdown/tests/shutdown_test.rs` 中添加 `ShutdownPolicy`,`ShutdownPhase`,shutdown cause(关闭原因),四阶段状态机和取消传播测试,覆盖 FR-020,FR-021 和 FR-045.
- [X] T072 [P] [US5] 在 `src/task/tests/blocking_task_test.rs` 中添加 `BlockingWorker`(阻塞工作任务),blocking shutdown policy(阻塞关闭策略) 和不可立即终止边界测试,覆盖 FR-044 和 SC-015.
- [X] T073 [P] [US5] 在 `src/tests/supervisor_shutdown_test.rs` 中添加 root shutdown(根关闭) 集成测试,覆盖 FR-022,FR-042,FR-045 和 SC-005.

### Implementation for User Story 5(用户故事五的实现)

- [X] T074 [P] [US5] 在 `src/shutdown/stage.rs` 中实现 `ShutdownPolicy`,`ShutdownPhase`,shutdown cause(关闭原因),graceful timeout(优雅关闭超时) 和 abort wait(强制终止等待),覆盖 FR-020 和 FR-045.
- [X] T075 [P] [US5] 在 `src/shutdown/coordinator.rs` 中实现 request stop(请求停止),graceful drain(优雅排空),abort stragglers(强制终止拖尾任务) 和 reconcile(状态对账) 协调器,覆盖 FR-045.
- [X] T076 [P] [US5] 在 `src/task/context.rs` 中把 `CancellationToken` 接入 `TaskContext`,child token(子令牌) 和 parent token(父令牌) 传播,覆盖 FR-021.
- [X] T077 [P] [US5] 在 `src/spec/child.rs` 中扩展 `TaskKind`,`AsyncWorker`,`BlockingWorker` 和 blocking shutdown policy(阻塞关闭策略),覆盖 FR-044.
- [X] T078 [US5] 在 `src/runtime/supervisor.rs` 中实现 `JoinSet` 任务所有权,reverse-order shutdown(逆序关闭) 和 draining(排空),覆盖 FR-022 和 FR-042.
- [X] T079 [US5] 在 `src/control/handle.rs` 和 `src/shutdown/coordinator.rs` 中实现 `shutdown_tree` 四阶段控制流,覆盖 FR-020 和 FR-045.
- [X] T080 [US5] 在 `src/event/payload.rs` 和 `src/state/supervisor.rs` 中发送 `ShutdownRequested`,`ShutdownPhaseChanged` 和 `ShutdownCompleted`,并输出 reconcile(状态对账) 后的终态当前状态,覆盖 FR-045.
- [X] T081 [US5] 在 `src/child_runner/runner.rs` 中实现 blocking task(阻塞任务) 关闭超时边界,不可立即终止事件和升级策略,覆盖 FR-044 和 SC-015.

**Checkpoint(检查点)**: 用户故事五可以独立验证关闭协议和无孤儿任务结果.

---

## Phase 8(阶段八): User Story 6(用户故事六) - 观测,审计并回放生命周期 (Priority(优先级): P6)

**Goal(目标)**: 维护者可以读取 current state(当前状态),事件流,structured log(结构化日志),tracing(结构化追踪),metrics(指标),audit event(审计事件),event journal(事件日志缓冲区) 和 `RunSummary`(运行摘要).

**Independent Test(独立测试)**: 让 child(子任务) 经历启动,心跳,失败,退避,重启,隔离和关闭,验证每次状态迁移都有完整可观测性信号.

### Tests for User Story 6(用户故事六的测试)

- [X] T082 [P] [US6] 在 `src/tests/supervisor_event_shape_test.rs` 中添加 `When`,`Where`,`What`,sequence(序号) 和 correlation id(关联标识) 集成测试,覆盖 FR-028,FR-029,FR-030,FR-031 和 SC-008.
- [X] T083 [P] [US6] 在 `src/observe/tests/observe_test.rs` 中添加 structured log(结构化日志),tracing event(追踪事件),metrics(指标),audit event(审计事件) 和低基数 label(标签) 测试,覆盖 FR-032,FR-033,FR-034,FR-047 和 SC-016.
- [X] T084 [P] [US6] 在 `src/journal/tests/journal_test.rs` 中添加 fixed-capacity event journal(固定容量事件日志缓冲区),dropped count(丢弃计数) 和最近事件查询测试,覆盖 FR-046.
- [X] T085 [P] [US6] 在 `src/summary/tests/summary_test.rs` 中添加 `RunSummary` 构建测试,覆盖 FR-046 和 SC-017.
- [X] T086 [P] [US6] 在 `src/tests/observability_smoke_test.rs` 中添加 observability smoke test(可观测性冒烟测试),覆盖 FR-049 和 SC-018.

### Implementation for User Story 6(用户故事六的实现)

- [X] T087 [P] [US6] 在 `src/event/time.rs` 和 `src/event/payload.rs` 中完善 `SupervisorEvent`,`EventTime`,`EventLocation`,`EventPayload` 和 policy decision payload(策略决定内容),覆盖 FR-028 到 FR-031.
- [X] T088 [P] [US6] 在 `src/observe/tracing.rs` 中实现每个 child attempt(子任务尝试) 的 tracing span(追踪范围) 和每次状态迁移的 tracing event(追踪事件),覆盖 FR-033.
- [X] T089 [P] [US6] 在 `src/observe/metrics.rs` 中实现必需 metrics facade(指标门面) 输出和 low-cardinality label validator(低基数标签校验器),覆盖 FR-034 和 FR-047.
- [X] T090 [P] [US6] 在 `src/journal/ring.rs` 中实现 fixed-capacity event journal(固定容量事件日志缓冲区),dropped count(丢弃计数) 和最近事件查询,覆盖 FR-046.
- [X] T091 [P] [US6] 在 `src/summary/builder.rs` 中实现 `RunSummary`,并从 event journal(事件日志缓冲区),current state(当前状态) 和策略决定生成诊断摘要,覆盖 FR-046.
- [X] T092 [US6] 在 `src/observe/pipeline.rs` 中实现 event bus fan-out(事件总线扇出),subscriber lag accounting(订阅者滞后计数) 和 test recorder(测试记录器) 接入,覆盖 FR-027 和 FR-049.
- [X] T093 [US6] 在 `src/control/command.rs` 和 `src/event/payload.rs` 中实现 command audit event(命令审计事件) 序列化,覆盖 FR-037.
- [X] T094 [US6] 在 `src/state/supervisor.rs`,`src/event/payload.rs`,`src/error/types.rs`,`src/journal/ring.rs` 和 `src/summary/builder.rs` 中为当前状态,事件,失败模型,event journal(事件日志缓冲区) 和 run summary(运行摘要) 添加 serde(序列化) 支持,覆盖 FR-049.

**Checkpoint(检查点)**: 用户故事六可以独立验证可观测性和诊断回放.

---

## Phase 9(阶段九): User Story 7(用户故事七) - 使用集中配置,示例和双语文档接入 (Priority(优先级): P7)

**Goal(目标)**: 使用者可以通过 rust-config-tree(集中配置树) v0.1.9 加载 YAML(数据序列化格式) 配置,运行示例,并阅读中英双语文档.

**Independent Test(独立测试)**: 加载 `examples/config/supervisor.yaml`,生成 `SupervisorSpec`,运行示例,并验证文档,契约和示例公开 API(接口) 一致.

### Tests for User Story 7(用户故事七的测试)

- [X] T095 [P] [US7] 在 `src/config/tests/yaml_config_test.rs` 中添加 rust-config-tree(集中配置树) v0.1.9 YAML(数据序列化格式) 加载,校验错误和 `FatalConfig` 测试,覆盖 FR-050,FR-051,FR-065 和 SC-033.
- [X] T096 [P] [US7] 在 `src/tests/supervisor_config_test.rs` 中添加 centralized configuration(集中化配置) 派生 `SupervisorSpec`,默认策略,可观测性选项和关闭预算的集成测试,覆盖 SC-019.
- [X] T097 [P] [US7] 在 `src/tests/supervisor_examples_test.rs` 中添加 examples smoke test(示例冒烟测试),覆盖 FR-052 和 SC-020.
- [X] T098 [P] [US7] 在 `src/tests/supervisor_docs_sync_test.rs` 中添加 documentation sync check(文档同步检查) 和 terminology check(术语检查),验证使用 `Shutdown Without Orphaned Tasks`(关闭后不留下孤儿任务),覆盖 FR-054,SC-022 和 SC-027.
- [X] T099 [P] [US7] 在 `src/tests/bilingual_docs_test.rs` 中添加 bilingual documentation check(双语文档检查),覆盖 FR-053 和 SC-021.

### Implementation for User Story 7(用户故事七的实现)

- [X] T100 [P] [US7] 在 `src/config/yaml.rs` 中实现 rust-config-tree(集中配置树) v0.1.9 YAML(数据序列化格式) 主配置加载和格式限制,覆盖 FR-050 和 FR-065.
- [X] T101 [P] [US7] 在 `src/config/loader.rs` 中实现配置校验,错误映射,缺失 runtime tunable constant(运行时可调常量) 拒绝和 `FatalConfig` 结果,覆盖 FR-051 和 FR-067.
- [X] T102 [P] [US7] 在 `src/config/state.rs` 中实现 `ConfigState` 到 `SupervisorSpec`,策略默认值,可观测性选项和关闭预算的派生,覆盖 FR-050 和 SC-019.
- [X] T103 [P] [US7] 在 `examples/supervisor_quickstart.rs` 中实现 quickstart(快速开始) 示例,覆盖 FR-052.
- [X] T104 [P] [US7] 在 `examples/config_tree_supervisor.rs` 中实现 rust-config-tree(集中配置树) 配置示例,覆盖 FR-052.
- [X] T105 [P] [US7] 在 `examples/restart_policy_lab.rs` 中实现 restart policy(重启策略) 学习示例,覆盖 FR-052.
- [X] T106 [P] [US7] 在 `examples/shutdown_tree.rs` 中实现 four-stage shutdown(四阶段关闭) 示例,覆盖 FR-052.
- [X] T107 [P] [US7] 在 `examples/observability_probe.rs` 中实现 observability(可观测性) 示例,覆盖 FR-052.
- [X] T108 [P] [US7] 在 `manual/zh/index.md` 和 `manual/en/index.md` 中编写完整手册入口,覆盖 FR-053.
- [X] T109 [P] [US7] 在 `docs/zh/index.md` 和 `docs/en/index.md` 中编写双语文档入口,覆盖 FR-053.
- [X] T110 [US7] 更新 `specs/001-create-supervisor-core/glossary.md`,确保 public API(公开接口),配置键,示例命令和测试目标全部登记,覆盖 FR-066.

**Checkpoint(检查点)**: 用户故事七可以独立验证配置,示例和文档同步.

---

## Phase 10(阶段十): User Story 8(用户故事八) - 遵守编码和发布约定 (Priority(优先级): P8)

**Goal(目标)**: 编码阶段满足完整英文代码文档,顶层目录模块结构,模块依赖图,认知复杂度,可维护性,SBOM(软件物料清单),crates.io readiness(发布就绪) 和并行治理要求.

**Independent Test(独立测试)**: 运行 source layout(源码布局),module boundary(模块边界),coding standard(编码标准),complexity(复杂度),maintainability(可维护性),release readiness(发布就绪),SBOM(软件物料清单),parallelization(并行化),blocker elimination(卡点消除) 和 lead agent supervision(主代理监督) 检查.

### Tests for User Story 8(用户故事八的测试)

- [X] T111 [P] [US8] 在 `src/tests/coding_standard_test.rs` 中添加英文 module doc(模块文档),struct doc(结构体文档),field doc(字段文档),function doc(函数文档),doctest(文档测试),test naming(测试命名),`src/lib.rs` 和 `src/<module>/mod.rs` 规则测试,覆盖 FR-055,FR-056,SC-023 和 SC-032.
- [X] T112 [P] [US8] 在 `src/tests/complexity_test.rs` 中添加 cognitive complexity(认知复杂度) 预算测试,覆盖 FR-060 和 SC-028.
- [X] T113 [P] [US8] 在 `src/tests/maintainability_test.rs` 中添加 maintainability profile(可维护性画像),module cohesion(模块内聚),coupling boundary(耦合边界),business hot path(业务热路径) 隔离和 control plane(控制面) 与 data plane(数据面) 隔离测试,覆盖 FR-035,FR-041,FR-061 和 SC-029.
- [X] T114 [P] [US8] 在 `src/tests/release_readiness_test.rs` 中添加 crates.io readiness(发布就绪) 测试,并验证 `supertrees` 只作为 concept input(概念输入) 而不是生产依赖,覆盖 FR-040,FR-058,FR-059 和 SC-026.
- [X] T115 [P] [US8] 在 `src/tests/sbom_test.rs` 中添加 CycloneDX JSON(CycloneDX JSON 格式),SPDX JSON(SPDX JSON 格式) 和 `Cargo.lock` 一致性测试,覆盖 FR-062 和 SC-030.
- [X] T116 [P] [US8] 在 `src/tests/parallel_governance_test.rs` 中添加 parallel workstream(并行工作流),task completion ledger(任务完成台账),blocker elimination record(卡点消除记录),lead agent supervision record(主代理监督记录) 和 correction record(纠偏记录) 测试,覆盖 FR-070 到 FR-076 和 SC-037 到 SC-044.

### Implementation for User Story 8(用户故事八的实现)

- [X] T117 [P] [US8] 在 `scripts/check-coding-standard.sh` 中实现 source layout(源码布局),module boundary(模块边界),documentation(文档),import rule(导入规则) 和 compatibility method(兼容方法) 禁止检查,覆盖 FR-039,FR-055,FR-056,FR-057 和 FR-077.
- [X] T118 [P] [US8] 在 `scripts/check-maintainability.sh` 中实现 module dependency map(模块依赖图),cognitive complexity(认知复杂度),maintainability profile(可维护性画像) 和 parallelization(并行化) 检查,覆盖 FR-060,FR-061,FR-068,FR-069 和 FR-070.
- [X] T119 [P] [US8] 在 `scripts/generate-sbom.sh` 中实现 CycloneDX JSON(CycloneDX JSON 格式) 和 SPDX JSON(SPDX JSON 格式) 生成入口,覆盖 FR-062.
- [X] T120 [P] [US8] 在 `scripts/validate-sbom.sh` 中实现 SBOM(软件物料清单) 格式,依赖版本,license(许可证),checksum(校验和) 和本地路径泄漏校验,覆盖 FR-062.
- [X] T121 [P] [US8] 在 `Cargo.toml` 中补齐 crates.io(软件包发布平台) 发布元数据,package include/exclude(打包包含或排除) 和 docs.rs(文档托管平台) 元数据,覆盖 FR-058.
- [X] T122 [P] [US8] 在 `docs/zh/quality-gates.md` 和 `docs/en/quality-gates.md` 中记录源码布局,模块入口,导入规则,测试命名,认知复杂度和可维护性门禁,覆盖 FR-054 和 FR-061.
- [X] T123 [P] [US8] 在 `docs/zh/parallel-governance.md` 和 `docs/en/parallel-governance.md` 中记录 parallel workstream(并行工作流),unattended implementation(无人值守实现),task completion ledger(任务完成台账),blocker elimination record(卡点消除记录) 和 lead agent supervision(主代理监督),覆盖 FR-070 到 FR-076.
- [X] T124 [P] [US8] 在 `src/id/`,`src/error/`,`src/event/` 和 `src/state/` 源码文件中补齐英文 module doc(模块文档),struct doc(结构体文档),field doc(字段文档),function doc(函数文档),source comment(源码注释) 和 public doctest(公共文档测试),覆盖 FR-055.
- [X] T125 [P] [US8] 在 `src/config/` 源码文件中补齐英文 module doc(模块文档),struct doc(结构体文档),field doc(字段文档),function doc(函数文档),source comment(源码注释) 和 public doctest(公共文档测试),覆盖 FR-055.
- [X] T126 [P] [US8] 在 `src/spec/`,`src/task/` 和 `src/readiness/` 源码文件中补齐英文 module doc(模块文档),struct doc(结构体文档),field doc(字段文档),function doc(函数文档),source comment(源码注释) 和 public doctest(公共文档测试),覆盖 FR-055.
- [X] T127 [P] [US8] 在 `src/policy/` 和 `src/test_support/` 源码文件中补齐英文 module doc(模块文档),struct doc(结构体文档),field doc(字段文档),function doc(函数文档),source comment(源码注释) 和 public doctest(公共文档测试),覆盖 FR-055.
- [X] T128 [P] [US8] 在 `src/tree/`,`src/registry/`,`src/runtime/` 和 `src/child_runner/` 源码文件中补齐英文 module doc(模块文档),struct doc(结构体文档),field doc(字段文档),function doc(函数文档),source comment(源码注释) 和 public doctest(公共文档测试),覆盖 FR-055.
- [X] T129 [P] [US8] 在 `src/control/`,`src/health/` 和 `src/shutdown/` 源码文件中补齐英文 module doc(模块文档),struct doc(结构体文档),field doc(字段文档),function doc(函数文档),source comment(源码注释) 和 public doctest(公共文档测试),覆盖 FR-055.
- [X] T130 [P] [US8] 在 `src/observe/`,`src/journal/` 和 `src/summary/` 源码文件中补齐英文 module doc(模块文档),struct doc(结构体文档),field doc(字段文档),function doc(函数文档),source comment(源码注释) 和 public doctest(公共文档测试),覆盖 FR-055.
- [X] T131 [US8] 在 `artifacts/validation/documentation-ownership.md` 中记录每个 `src/<module>/` 源码文件的 documentation owner(文档负责人),对应实现任务,验收检查和剩余缺口,确认源码文档已经由 T124-T130 的独立 file boundary(文件边界) 完成,覆盖 FR-055.

**Checkpoint(检查点)**: 用户故事八可以独立验证编码规范,发布准备和并行治理.

---

## Phase 11(最终阶段): Polish & Cross-Cutting Concerns(收尾和横向事项)

**Purpose(目的)**: 运行全量验证,修正文档漂移,确认所有任务完成证据.

- [X] T132 运行 `cargo fmt --check`,并把结果记录到 `artifacts/validation/cargo-fmt.md` 对应验收项,覆盖 FR-061 和 SC-029.
- [X] T133 运行 `cargo check`,并把结果记录到 `artifacts/validation/cargo-check.md` 对应验收项,覆盖 FR-058,FR-061,SC-026 和 SC-029.
- [X] T134 运行 `cargo test`,并把所有 `_test.rs` 测试目标结果记录到 `artifacts/validation/cargo-test.md`,覆盖 SC-001 到 SC-045.
- [X] T135 运行 `cargo doc --no-deps`,并把英文 rustdoc(代码文档注释) 生成结果记录到 `artifacts/validation/cargo-doc.md`,覆盖 FR-055 和 SC-023.
- [X] T136 运行 `scripts/check-coding-standard.sh`,并把源码布局,模块入口,导入规则和文档门禁结果记录到 `artifacts/validation/coding-standard.md`,覆盖 FR-055,FR-056,FR-057,FR-063,FR-064,FR-077,SC-023,SC-024,SC-025,SC-031,SC-032 和 SC-045.
- [X] T137 运行 `scripts/check-maintainability.sh`,并把 module dependency map(模块依赖图),cognitive complexity(认知复杂度),maintainability profile(可维护性画像) 和 parallelization(并行化) 门禁结果记录到 `artifacts/validation/maintainability.md`,覆盖 FR-060,FR-061,FR-068,FR-069,FR-070,SC-028,SC-029,SC-036,SC-037,SC-038 和 SC-041.
- [X] T138 运行 `scripts/generate-sbom.sh` 和 `scripts/validate-sbom.sh`,并把 `artifacts/sbom/rust-supervisor.cdx.json` 和 `artifacts/sbom/rust-supervisor.spdx.json` 校验结果记录到 `artifacts/validation/sbom.md`,覆盖 FR-062 和 SC-030.
- [X] T139 运行 `cargo package --list`,并把 package contents(打包内容) 检查结果记录到 `artifacts/validation/cargo-package-list.md`,覆盖 FR-058,FR-059 和 SC-026.
- [X] T140 运行 `cargo publish --dry-run`,并把 crates.io readiness(发布就绪) 结果记录到 `artifacts/validation/cargo-publish-dry-run.md`,覆盖 FR-058,FR-059 和 SC-026.
- [X] T141 更新 `specs/001-create-supervisor-core/quickstart.md`,记录最终验证命令和通过条件,覆盖 FR-054 和 SC-022.
- [X] T142 更新 `specs/001-create-supervisor-core/contracts/public-api.md`,`specs/001-create-supervisor-core/data-model.md` 和 `specs/001-create-supervisor-core/glossary.md`,确保代码,契约,数据模型和词汇表同步,覆盖 FR-054,FR-066,SC-022 和 SC-034.
- [X] T143 在 `specs/001-create-supervisor-core/tasks.md` 中完成 task completion ledger(任务完成台账) 的最终证据记录,确认没有 pending task(待处理任务),in-progress task(进行中任务),失败检查或未记录完成证据,覆盖 FR-071 到 FR-076 和 SC-039 到 SC-044.
- [X] T144 记录 **`speckit.sync.proposals`** Proposal P9 APPLIED: `glossary.md` Policy And State(策略与状态) 表与 Rust 类型表补齐 `ChildState` 与 `ChildRuntimeRecord`、`ManagedChildState`、`ChildControlResult`、`ChildRuntimeState` 的对照说明,交叉引用 `specs/004-3-child-runtime-state-control`,落账日期 2026-05-15.

---

## Dependencies & Execution Order(依赖和执行顺序)

### Phase Dependencies(阶段依赖)

- Phase 1(阶段一) 没有依赖,可以立即开始.
- Phase 2(阶段二) 依赖 Phase 1(阶段一) 完成,并阻塞所有用户故事.
- User Story(用户故事) 阶段都依赖 Phase 2(阶段二) 完成.
- US1(用户故事一) 是 MVP(最小可用产品),应先完成并验证.
- US2(用户故事二) 到 US8(用户故事八) 可以按优先级推进,也可以在契约稳定后按 workstream(工作流) 并行推进.
- Phase 11(最终阶段) 依赖被选择的所有用户故事完成.

### User Story Dependencies(用户故事依赖)

- US1(用户故事一) 依赖 Foundational(基础) 完成,不依赖其它故事.
- US2(用户故事二) 可以独立验证树结构,但运行时集成会复用 US1(用户故事一) 的启动路径.
- US3(用户故事三) 可以独立验证策略模块,运行时重启集成会复用 US1(用户故事一) 和 US2(用户故事二).
- US4(用户故事四) 可以独立验证控制命令和健康模块,运行时派发会复用 US1(用户故事一).
- US5(用户故事五) 可以独立验证关闭模块,运行时关闭集成会复用 US2(用户故事二) 的树顺序.
- US6(用户故事六) 可以独立验证可观测性模块,完整冒烟测试会复用 US1(用户故事一),US3(用户故事三) 和 US5(用户故事五).
- US7(用户故事七) 可以并行编写示例和文档,配置派生会依赖 `ConfigState`(配置状态) 基础.
- US8(用户故事八) 可以并行实现脚本和文档,最终门禁依赖所有源码和文档产物.

### Primary Workstream Ownership(主工作流所有权)

- WS0 Setup(初始化): T001-T004.
- WS1 Contract Foundation(契约基础): T009-T013,T015-T021,T024,T124.
- WS2 Configuration(集中配置): T005,T014,T022,T095,T096,T100-T102,T125.
- WS3 Declaration And Task(声明和任务): T025-T033,T126.
- WS4 Policy And Time(策略和时间): T023,T049-T060,T127.
- WS5 Runtime Tree(运行时树): T034-T048,T128.
- WS6 Control And Shutdown(控制和关闭): T061-T081,T129.
- WS7 Observability Diagnostics(可观测性和诊断): T082-T094,T130.
- WS8 Docs Examples Release(文档示例和发布): T007,T008,T097-T099,T103-T110,T121-T123,T138-T142.
- WS9 Quality Governance(质量治理): T006,T111-T120,T131-T137,T143-T144.

### Cross-Workstream Handoffs(跨工作流交接)

- T014 由 WS2 Configuration(集中配置) 拥有,WS1 Contract Foundation(契约基础) 只消费它的检查结果.
- T022 由 WS2 Configuration(集中配置) 拥有,WS1 Contract Foundation(契约基础) 只依赖 `ConfigState`(配置状态) 契约.
- T023 由 WS4 Policy And Time(策略和时间) 拥有,WS1 Contract Foundation(契约基础) 只使用 test time(测试时间) 支持.
- T067-T070 由 WS6 Control And Shutdown(控制和关闭) 拥有,WS5 Runtime Tree(运行时树) 只提供 runtime(运行时) 接入点.
- T076-T077 由 WS6 Control And Shutdown(控制和关闭) 拥有,WS3 Declaration And Task(声明和任务) 只审查 `TaskContext`(任务上下文) 和 `TaskKind`(任务类型) 契约兼容性.
- T097-T099,T103-T110 由 WS8 Docs Examples Release(文档示例和发布) 拥有,WS2 Configuration(集中配置) 只提供配置加载契约.
- T124-T130 由各自源码模块的 primary workstream(主工作流) 拥有,WS9 Quality Governance(质量治理) 只通过 T131 汇总 documentation ownership(文档所有权) 证据.
- T121-T123,T138-T142 由 WS8 Docs Examples Release(文档示例和发布) 拥有,WS9 Quality Governance(质量治理) 只消费验证结果.
- T131-T137,T143-T144 由 WS9 Quality Governance(质量治理) 拥有,其它工作流只提供各自完成证据.

### Parallel Opportunities(并行机会)

- T005-T008 可以并行,因为它们修改示例配置,脚本入口,文档目录和发布文档.
- T009-T015 可以并行,因为它们分别写入不同 `src/tests/*_test.rs` 文件.
- T016-T023 可以并行,因为它们分别写入不同顶层模块目录.
- 每个用户故事中的 `[P]` 测试任务可以并行,因为它们写入不同测试文件.
- 每个用户故事中的 `[P]` 实现任务可以并行,因为它们写入不同模块目录.
- US7(用户故事七) 的 examples(示例程序),manual(手册),docs(文档) 和 config(配置) 任务可以在契约稳定后并行.
- US8(用户故事八) 的 scripts(脚本),release metadata(发布元数据),quality docs(质量文档) 和 governance docs(治理文档) 可以并行.
- T132-T140 是 final validation(最终验证) 命令,默认按顺序运行.只有在显式设置独立 `CARGO_TARGET_DIR`(Cargo 目标目录) 并记录证据时,才允许并行运行会共享 Cargo(构建工具) 输出目录的命令.

---

## Parallel Example(并行示例): User Story 1(用户故事一)

```bash
# 同时启动用户故事一的测试任务:
Task(任务): "T025 在 src/spec/tests/spec_test.rs 中添加 ChildSpec 和 SupervisorSpec 字段校验测试"
Task(任务): "T026 在 src/task/tests/task_test.rs 中添加 TaskFactory 和 TaskContext 测试"
Task(任务): "T027 在 src/readiness/tests/readiness_test.rs 中添加 readiness 测试"

# 同时启动用户故事一的独立模块实现:
Task(任务): "T029 在 src/spec/child.rs 中实现 ChildSpec"
Task(任务): "T031 在 src/task/context.rs 中实现 TaskContext"
Task(任务): "T033 在 src/readiness/signal.rs 中实现 ReadinessPolicy"
```

## Parallel Example(并行示例): User Story 8(用户故事八)

```bash
Task(任务): "T117 在 scripts/check-coding-standard.sh 中实现编码标准检查"
Task(任务): "T118 在 scripts/check-maintainability.sh 中实现可维护性检查"
Task(任务): "T119 在 scripts/generate-sbom.sh 中实现 SBOM 生成"
Task(任务): "T123 在 docs/zh/parallel-governance.md 和 docs/en/parallel-governance.md 中记录并行治理"
```

---

## Implementation Strategy(实现策略)

### MVP First(先做最小可用产品)

1. 完成 Phase 1(阶段一) 和 Phase 2(阶段二).
2. 完成 US1(用户故事一),交付声明并运行子任务的最小可用监督器.
3. 运行 T025-T038 对应测试,确认 MVP(最小可用产品) 可独立验证.

### Incremental Delivery(增量交付)

1. 先交付 US1(用户故事一),建立子任务声明,启动和当前状态查询.
2. 再交付 US2(用户故事二),加入监督树.
3. 再交付 US3(用户故事三),加入重启,退避和熔断策略.
4. 再交付 US4(用户故事四) 和 US5(用户故事五),加入控制面,健康和关闭协议.
5. 再交付 US6(用户故事六),加入完整可观测性和诊断回放.
6. 再交付 US7(用户故事七) 和 US8(用户故事八),完成配置,示例,文档,发布和治理门禁.

### Unattended Parallel Strategy(无人值守并行策略)

1. lead agent(主代理) 先完成 T001-T024,稳定契约和质量门禁.
2. lead agent(主代理) 按 WS1-WS9 分派 subagent workstream(子代理工作流),每个 subagent(子代理) 只修改自己的 primary files(主文件).
3. lead agent(主代理) 持续读取 task completion ledger(任务完成台账),调度可执行 pending task(待处理任务),不得在单个任务完成后停止等待人工继续.
4. lead agent(主代理) 对每个 subagent output(子代理输出) 执行 clean review record(清洁审查记录) 或 correction record(纠偏记录).
5. 所有任务完成后,运行 T132-T144 的最终验证和完成证据记录.

---

## Notes(说明)

- 本任务清单禁止使用 `src/supervision/` 中间层.
- 本任务清单禁止使用 `src/<module>.rs` 平铺模块文件.
- 本任务清单禁止重导出兼容 API(接口),旧接口别名和第三方 API(接口) 形状复制.
- 所有 public API(公开接口),configuration schema(配置模式),example behavior(示例行为),observability signal(可观测性信号) 和 glossary term(词汇表词条) 变化时,必须同步文档.
- 每个任务完成时都必须保留测试结果,文件变更和文档同步证据.
