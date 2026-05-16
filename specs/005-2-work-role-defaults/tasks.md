# Tasks(任务): 005-2 Work Role Defaults(工作角色默认值)

**Input(输入)**: 设计文档来自 `/specs/005-2-work-role-defaults/`
**Prerequisites(前置文档)**: plan.md, spec.md, research.md, data-model.md, contracts/role-defaults.md, quickstart.md

**Tests(测试)**: 行为变化必须先有测试任务,再有实现任务.纯文档或纯模板变更必须说明运行时测试为什么不适用.

**Organization(组织方式)**: 任务按用户故事分组.本功能只有一个用户故事 (US1 - P1),覆盖五类工作角色的默认策略实现与验收.

## Format(格式): `[ID] [P?] [Story] Description(描述)`

- **[P]**: 可以并行执行,因为任务修改不同文件,并且不依赖未完成任务.
- **[Story]**: 标记任务属于哪个用户故事,例如 US1.
- 任务描述必须写出准确文件路径.
- 任务描述必须使用中文;英文术语必须写成 `English(中文说明)`.
- Rust(编程语言) 项目中, 所有单元测试,契约测试和集成测试都必须放在外部 `tests/` 目录, 不得把测试代码写入 `src/` 模块文件.
- 并行任务必须修改不同文件;如果两个任务会修改同一个文件, 不得同时标记 `[P]`.

---

## Phase 1(阶段一): Setup(共享基础设施)

**Purpose(目的)**: 初始化项目结构和基础依赖.

- [ ] T001 确认 `cargo fmt`, `cargo clippy`, `cargo test` 验证命令可用.
- [ ] T002 [P] 在 `src/policy/` 目录下创建 `role_defaults.rs` 模块文件.
- [ ] T003 [P] 在 `tests/` 目录下创建 `work_role_defaults_integration.rs` 集成测试文件.

---

## Phase 2(阶段二): Foundational(阻塞前置基础)

**Purpose(目的)**: 完成任何用户故事开始前都必须存在的核心数据结构与配置集成.

**Critical(关键要求)**: 本阶段完成前,用户故事实现不能开始.

- [ ] T004 在 `src/policy/role_defaults.rs` 中定义 `WorkRole` 枚举 (Service, Worker, Job, Sidecar, Supervisor),实现 `Serialize`, `Deserialize`, `JsonSchema`, `Display`, `as_str()` 方法.
- [ ] T005 在 `src/policy/role_defaults.rs` 中定义 `SidecarConfig` 结构 (`primary_child_id: ChildId`, `linked_lifecycle: bool`),实现 `Serialize`, `Deserialize`, `JsonSchema`.
- [ ] T006 在 `src/policy/role_defaults.rs` 中定义五个动作枚举: `OnSuccessAction`, `OnFailureAction`, `OnManualStopAction`, `OnTimeoutAction`, `OnBudgetExhaustedAction`,每个枚举实现 `Serialize`, `Deserialize`, `JsonSchema`.
- [ ] T007 在 `src/policy/role_defaults.rs` 中定义 `RoleDefaultPolicyPack` 结构,包含九个字段 (`on_success_exit`, `on_failure_exit`, `on_manual_stop`, `on_timeout`, `on_budget_exhausted`, `default_restart_limit`, `default_escalation_policy`, `default_backoff_policy`, `success_exit_codes`),实现 `Serialize`, `Deserialize`, `JsonSchema`.
- [ ] T008 在 `src/policy/role_defaults.rs` 中为 `RoleDefaultPolicyPack` 实现五个角色常量: `SERVICE_DEFAULT`, `WORKER_DEFAULT`, `JOB_DEFAULT`, `SIDECAR_DEFAULT`, `SUPERVISOR_DEFAULT`,以及 `for_role(role: WorkRole) -> Self` 查找函数.
- [ ] T009 [P] 在 `src/policy/role_defaults.rs` 中定义 `PolicySource` 枚举 (`RoleDefault`, `UserOverride`, `FallbackDefault`) 和 `EffectivePolicy` 结构 (`work_role`, `policy_pack`, `source`, `used_fallback`, `overridden_fields`),实现 `Serialize`, `Deserialize`, `JsonSchema`.
- [ ] T010 在 `src/spec/child.rs` 的 `ChildSpec` 结构中新增 `work_role: Option<WorkRole>` 和 `sidecar_config: Option<SidecarConfig>` 字段,标注 `#[serde(default)]`.
- [ ] T011 在 `src/event/payload.rs` 的 `TypedSupervisionEvent` 结构中新增 `work_role: Option<WorkRole>`, `used_fallback_default: bool`, `effective_policy_source: Option<PolicySource>` 字段.
- [ ] T012 在 `src/config/configurable.rs` 或等价位置实现角色默认策略与用户配置的合并逻辑 (`EffectivePolicy::merge()` 函数),支持三层优先级模型 (用户覆写 > 角色默认 > 全局兜底).
- [ ] T013 在 `src/config/` 模块中实现配置加载阶段的验证规则: Sidecar 必须提供 `sidecar_config`, `primary_child_id` 必须存在且不能是 Sidecar 角色,链式边车禁止;若存在多个 service 角色子任务且 sidecar 未显式声明 `primary_child_id`,则拒绝加载并报错.
- [ ] T014 在 `src/config/` 模块中实现冲突检测与警告逻辑: 当用户显式覆写与角色语义矛盾时 (如 Job + Permanent 重启),输出 WARN 级别日志并标注冲突点.

**Checkpoint(检查点)**: 基础数据结构、配置集成、验证逻辑已完成,用户故事实现可以开始.

---

## Phase 3(阶段三): User Story 1(用户故事一) - 按角色套用安全默认 (Priority(优先级): P1) MVP(最小可用产品)

**Goal(目标)**: 为五种工作角色 (service, worker, job, sidecar, supervisor) 实现默认监督行为,在成功退出、失败退出、人工停止、超时和预算耗尽场景下提供不同的默认动作.

**Independent Test(独立测试)**: 为每个角色准备一份最小的示例拓扑,从外部检查在只用默认策略且不额外覆写时,成功退出与失败退出触发的自动动作是否与 contracts/role-defaults.md 中的行为对照表一致.

### Tests for User Story 1(用户故事一的测试)

> **NOTE(说明): 必须先写这些测试,并确认它们在实现前失败.**

- [ ] T015 [P] [US1] 在 `tests/work_role_defaults_integration.rs` 中添加 Job 角色成功退出后不得自动再起的集成测试 (验收场景 1).
- [ ] T016 [P] [US1] 在 `tests/work_role_defaults_integration.rs` 中添加 Service 角色成功退出后允许自动重启的集成测试 (验收场景 2).
- [ ] T017 [P] [US1] 在 `tests/work_role_defaults_integration.rs` 中添加 Worker 角色失败后限次数重试并在预算耗尽后停止的集成测试 (验收场景 3).
- [ ] T018 [P] [US1] 在 `tests/work_role_defaults_integration.rs` 中添加 Sidecar 角色失败时单独重启且不连带主服务的集成测试 (验收场景 4, `linked_lifecycle: false`).
- [ ] T019 [P] [US1] 在 `tests/work_role_defaults_integration.rs` 中添加 Supervisor 角色外层核算内层树预算的集成测试 (验收场景 5).
- [ ] T020 [US1] 在 `tests/work_role_defaults_integration.rs` 中添加角色缺失时回落到 Worker 默认并输出诊断日志的测试 (边界情况 1).
- [ ] T021 [US1] 在 `tests/work_role_defaults_integration.rs` 中添加 Job + Permanent 重启策略冲突警告的测试 (边界情况 3).
- [ ] T022 [US1] 在 `tests/work_role_defaults_integration.rs` 中添加 Sidecar 缺失 `sidecar_config` 时配置加载拒绝的测试 (边界情况 2).
- [ ] T023 [US1] 在 `tests/work_role_defaults_integration.rs` 中添加 Sidecar 引用不存在 `primary_child_id` 时配置加载拒绝的测试.
- [ ] T024 [US1] 在 `tests/work_role_defaults_integration.rs` 中添加链式边车 (Sidecar primary 也是 Sidecar) 配置加载拒绝的测试.
- [ ] T025 [US1] 在 `tests/work_role_defaults_integration.rs` 中添加 `TypedSupervisionEvent` 事件载荷包含 `work_role`, `used_fallback_default`, `effective_policy_source` 字段的验证测试.

### Implementation for User Story 1(用户故事一的实现)

- [ ] T026 [US1] 在 `src/runtime/control_loop.rs` 中实现 `prepare_effective_policy(child_spec: &ChildSpec) -> EffectivePolicy` 函数,在 `evaluate budget` 阶段之前计算生效策略.
- [ ] T027 [US1] 在 `src/runtime/control_loop.rs` 的 `decide action` 阶段集成 `EffectivePolicy`,使用合并后的策略决定重启、停止或升级动作.
- [ ] T028 [US1] 在 `src/runtime/control_loop.rs` 的 `execute action` 阶段写入带角色信息的 `TypedSupervisionEvent` 事件载荷.
- [ ] T029 [US1] 在 `src/policy/decision.rs` 的 `PolicyEngine` 中集成角色默认策略读取逻辑,确保 `decide action` 能够访问合并后的生效策略.
- [ ] T030 [US1] 在 `src/observe/` 管道中确保角色信息 (`work_role`, `used_fallback_default`, `effective_policy_source`) 正确转发至日志、指标与 dashboard(仪表板).
- [ ] T031 [US1] 在 `src/config/` 模块中实现角色缺失时的兜底默认逻辑 (内部使用 `WorkRole::Worker`),并在 WARN 级别日志中标注已启用安全回退.
- [ ] T032 [US1] 为 `src/policy/role_defaults.rs` 中的所有公共 API 添加英文文档注释 (`///`),包括枚举变体、结构字段、函数的参数与返回值说明 (此为宪章原则 VI 第 5 条强制要求,`src/` 中 Rust 注释必须英文).
- [ ] T033 [US1] 运行 `cargo test --test work_role_defaults_integration` 确认所有集成测试通过.
- [ ] T034 [US1] 运行 `cargo test` 确认全部单元测试与集成测试通过,无回归.
- [ ] T041 [US1] 更新 `specs/005-2-work-role-defaults/quickstart.md` 确保包含五类角色的最小示例拓扑与行为对照表引用,满足 SC-002 文档交付要求.
- [ ] T042 [US1] 在配置加载阶段明确标注本版本不支持角色热更新,若检测到运行中修改 `work_role` 字段则输出错误并提示重建监督单元 (边界情况 2).

**Checkpoint(检查点)**: 用户故事一已经完整可用,并且可以独立测试.五类角色的默认行为与行为对照表一致.

---

## Phase 4(阶段四): Polish & Cross-Cutting Concerns(打磨与跨切面关注点)

**Purpose(目的)**: 代码质量、文档完善、性能优化.

- [ ] T035 运行 `cargo fmt` 格式化全部源码 (此为最终发布前复查,确保实现过程中无格式漂移).
- [ ] T036 运行 `cargo clippy --all-targets --all-features -- -D warnings` 修复所有 lint 警告 (此为最终发布前复查,确保实现过程中无 lint 漂移).
- [ ] T037 确认 `src/` 中所有 Rust 注释均为英文,规格文档保持中文.
- [ ] T038 更新 `specs/005-2-work-role-defaults/quickstart.md` 中的代码阅读顺序与实际实现一致.
- [ ] T039 在 AGENTS.md 的 SPECKIT START 段中确认当前功能计划指向 `specs/005-2-work-role-defaults/plan.md`.
- [ ] T040 运行 `cargo build --release` 确认发布模式编译成功.

---

## Dependencies(依赖关系)

### User Story Completion Order(用户故事完成顺序)

```
Phase 1 (Setup) → Phase 2 (Foundational) → Phase 3 (US1 - P1) → Phase 4 (Polish)
```

本功能只有一个用户故事 (US1),无多故事依赖.

### Parallel Execution Opportunities(并行执行机会)

**Phase 1 并行**:
- T002 (创建 role_defaults.rs) 和 T003 (创建集成测试文件) 可并行,修改不同文件.

**Phase 2 并行**:
- T004, T005, T006 可并行,均在 `src/policy/role_defaults.rs` 中定义不同数据结构,但需注意文件写入冲突 (建议按顺序执行).
- T009 (PolicySource 和 EffectivePolicy) 可在 T007-T008 完成后并行.
- T010 (ChildSpec 扩展) 和 T011 (TypedSupervisionEvent 扩展) 可并行,修改不同文件.

**Phase 3 测试并行**:
- T015-T019 (五个角色的验收测试) 可并行写入 `tests/work_role_defaults_integration.rs`,但需注意文件写入冲突 (建议按顺序执行).
- T020-T025 (边界情况测试) 可在 T015-T019 完成后并行.

**Phase 3 实现并行**:
- T026-T028 (控制循环集成) 需按顺序执行,因为涉及同一文件的不同阶段.
- T029 (策略引擎集成) 和 T030 (可观察性管道) 可在 T026-T028 完成后并行.

### Suggested MVP Scope(建议的 MVP 范围)

**MVP = Phase 1 + Phase 2 + Phase 3 (US1)**

MVP 交付五类工作角色的默认策略实现,包括:
- 核心数据结构 (`WorkRole`, `RoleDefaultPolicyPack`, `EffectivePolicy` 等)
- 配置加载与验证 (角色声明、sidecar 绑定、冲突检测)
- 运行时集成 (控制循环注入、事件载荷写入)
- 完整验收测试 (五个角色 + 六个边界情况)

Phase 4 (Polish) 为可选打磨阶段,不影响功能可用性.

---

## Summary(摘要)

**Total Task Count(总任务数)**: 42 个任务

**Task Count Per Phase(各阶段任务数)**:
- Phase 1 (Setup): 3 个任务
- Phase 2 (Foundational): 11 个任务
- Phase 3 (US1): 22 个任务 (11 个测试 + 9 个实现 + 2 个文档与边界任务)
- Phase 4 (Polish): 6 个任务

**Parallel Opportunities(并行机会)**: 约 6 组并行任务可加速开发 (T004-T006 移除 [P] 后减少 2 组)

**Independent Test Criteria(独立测试标准)**:
- US1: 五个角色的标准验收样例中,Job 成功后再起比例为 0%,Service 成功后保持自动恢复,Worker 限次数重试,Sidecar 单独重启不连带主服务,Supervisor 外层核算预算.抽样一致率 100%.

**Format Validation(格式验证)**: 所有任务均遵循 `- [ ] [TaskID] [P?] [Story?] Description with file path` 格式,复选框、任务 ID、故事标签、文件路径齐全.
