# Spec Drift Report(规格漂移报告)

Generated(生成时间): 2026-05-17T21:03:07+08:00
Project(项目): rust-tokio-supervisor
Scope(范围): `specs/005-1-failure-policy-reliability`, `specs/005-2-work-role-defaults` (per `.specify/feature.json` active feature directories)

一句话结论: 活跃切片 13 项需求与成功标准检查全部对齐, 0 项 drift(漂移). `005-2` 规格, 契约与实现已在 unknown work_role(未知工作任务角色) 与 `success_exit_codes`(成功退出码集合) 边界上闭合.

## Summary(摘要)

| Category(类别) | Count(数量) |
| --- | ---: |
| Specs Analyzed(深度分析规格) | 2 |
| Other Specs Spot-Check(其余规格抽检) | 7 |
| Requirements Checked(已检查需求) | 13 |
| Aligned(已对齐) | 13 (100%) |
| Drifted(漂移) | 0 |
| Not Implemented(未实现) | 0 |
| Unspecced Code(未入规格代码) | 0 |

## Validation(验证)

| Command(命令) | Result(结果) |
| --- | --- |
| `cargo test --test work_role_defaults_integration` | 15 passed |
| `cargo test --test supervisor_pipeline_order` | 4 passed |
| `cargo test --test supervisor_concurrent_restart_throttle` | 7 passed |
| `cargo test --test supervisor_backoff_jitter_distribution` | 7 passed |
| `cargo test --test supervisor_meltdown_group_isolation` | 5 passed |
| `cargo clippy --all-targets --all-features -- -D warnings` | 0 warnings |
| `cargo test` | workspace + doc-tests passed |

## Detailed Findings(详细发现)

### Spec(规格): 005-1-failure-policy-reliability - 失败策略流水线与生产级退避

#### Aligned(已对齐)

- **FR-001**: 每次运行结束进入六阶段 `policy pipeline(策略流水线)`. `control_loop(控制循环)` 构造 `PipelineContext(管线上下文)`, 调用 `SupervisionPipeline(监督管线)`, 经 `ObservabilityPipeline(可观察性管道)` 发出 `PipelineStageDiagnostic(管线阶段诊断)`. 锚点: `src/runtime/control_loop.rs`, `src/runtime/pipeline.rs:249-590`, `src/observe/pipeline.rs`.
- **FR-002**: `MeltdownTracker(熔断跟踪器)` 按 `child`(子任务), `group`(分组), `supervisor`(监督器) 三层计数; 合并后事件含 `scopes_triggered`(已触发作用域列表) 与 `lead_scope`(主导归因作用域). 锚点: `src/runtime/pipeline.rs:354-545`, `src/policy/meltdown.rs`.
- **FR-003**: `BackoffPolicy(退避策略)` 支持 `full jitter`(全抖动), `decorrelated jitter`(去相关抖动), `cold start budget`(冷启动预算), `hot loop detection`(热循环检测); 并发闸门在 `concurrent_gate(并发闸门)` 与 `control_loop` 中可核对. 锚点: `src/policy/backoff.rs`, `src/runtime/concurrent_gate.rs`.
- **SC-001**: 六阶段顺序可由 `PipelineStageDiagnostic` 与 `tests/supervisor_pipeline_order.rs` 核对.
- **SC-002**: 分组隔离见 `tests/supervisor_meltdown_group_isolation.rs`.
- **SC-003**: 并发闸门超限与 `ThrottleGateOwner(闸门归属)` 见 `tests/supervisor_concurrent_restart_throttle.rs`.
- **SC-004**: 抖动分散度见 `tests/supervisor_backoff_jitter_distribution.rs`.

#### Drifted(漂移)

无.

### Spec(规格): 005-2-work-role-defaults - 监督任务角色与默认策略

#### Aligned(已对齐)

- **FR-001**: 五类 `WorkRole(工作任务角色)` 默认策略经 `RoleDefaultPolicy::for_role()` 解析; `EffectivePolicy::merge()` 在 `evaluate budget`(评估预算) 前生效并进入 `decide action`(决定动作). 锚点: `src/policy/role_defaults.rs`, `src/runtime/pipeline.rs:366`, `src/runtime/control_loop.rs`.
- **SC-001**: 五角色行为见 `tests/work_role_defaults_integration.rs` (15 cases).
- **SC-002**: `data-model.md` 与 `contracts/role-defaults.md` 已改为私有默认构造函数与 `EffectivePolicy::merge(role, Vec<String>)`, 不再承诺按角色命名的公开默认常量.
- **SC-003**: `sidecar_config(辅助任务配置)` 缺失, 未知 `primary_child_id(主任务标识)`, 链式 sidecar, `Job + Permanent(一次性作业加永久重启)` 冲突均有可读错误. 锚点: `src/spec/supervisor.rs:486`, `tests/work_role_defaults_integration.rs`.
- **EC-001**: 缺失 `work_role` 回落 `Worker(工作任务)` 并标注 `FallbackDefault(兜底默认)`; 未知 `work_role` 字符串在规格中定义为加载阶段拒绝, 与 `unknown_work_role_is_rejected_by_deserialization` 测试一致. 锚点: `specs/005-2-work-role-defaults/spec.md:35`, `src/policy/role_defaults.rs:271`.
- **CONTRACT-SUCCESS-002**: 契约 **Rule SUCCESS-002/003** 写明 `success_exit_codes` 为 `RoleDefaultPolicy` 内部字段, 非 `ChildSpec` 用户覆写; 成功路径由 `TaskResult::Succeeded(任务成功结果)` 进入 `ExitClassification::Success`. 锚点: `specs/005-2-work-role-defaults/contracts/role-defaults.md:255-257`.

#### Drifted(漂移)

无.

## Other Specs Spot-Check(其余规格抽检)

下列规格未在本轮逐条核对 FR(功能需求), 仅确认对应实现目录与测试目标仍存在, 视为历史切片基线已实现:

| Spec ID | Spot-check(抽检结论) |
| --- | --- |
| 001-create-supervisor-core | core modules under `src/` |
| 002-config-schema-support | `src/config/loader.rs` |
| 003-supervisor-dashboard | `src/dashboard/` |
| 004-1-runtime-lifecycle-guard | lifecycle guard in runtime |
| 004-2-real-shutdown-pipeline | `src/runtime/shutdown_pipeline.rs` |
| 004-3-child-runtime-state-control | `src/runtime/child_runtime_state.rs` |
| 004-4-generation-fencing | `src/tests/supervisor_generation_fencing_test.rs` |

## Unspecced Code(未入规格代码)

未发现活跃切片范围内需要单独 backfill(回填规格) 的新增能力.

## Inter-Spec Conflicts(规格间冲突)

`005-1` 与 `005-2` 在 `evaluate budget`(评估预算) 与 `decide action`(决定动作) 衔接上无互斥要求.

## Recommendations(建议)

1. 活跃切片无需 drift(漂移) 修复.
2. 若后续要支持 `ChildSpec.success_exit_codes(子任务规格成功退出码集合)` 或未知角色字符串回落, 先开新 iteration(迭代) 再改代码.
3. 若需全仓 001-071 级 FR 逐条复检, 运行 `/speckit.sync.analyze` 并指定更宽 scope(范围) 或拆分多次分析.
