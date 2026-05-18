# Requirements Quality Checklist: 子任务运行状态控制

**Purpose(目的)**: 验证 `004-3-child-runtime-state-control` 规格中子任务运行状态记录与控制命令的需求质量、完整性和一致性。
**Created(创建日期)**: 2026-05-19
**Feature(功能)**: [spec.md](../spec.md)

## Requirement Completeness(需求完整性)

- [x] CHK001 - ChildRuntimeState(子任务运行状态记录) 的所有字段(spec/generation/attempt/status/cancellation_token/runtime_handle/last_heartbeat/readiness/restart_limit)是否在需求中完整列出？[Completeness, Spec §FR-001]
- [x] CHK002 - 公开暴露的可序列化事实(attempt/status/stop_state/liveness/readiness/restart_limit/cancel_delivered)是否在需求中明确定义？[Completeness, Spec §FR-001]
- [x] CHK003 - 内部字段与公开字段的隔离规则(cancellation_token/abort_handle/completion_receiver 只属于 runtime 内部)是否在需求中显式声明？[Completeness, Spec §FR-001]
- [x] CHK004 - PauseChild/RemoveChild/QuarantineChild 三种控制命令各自的行为差异(是否阻止自动重启、是否物理删除记录)是否完整定义？[Completeness, Spec §FR-002, US-2]
- [x] CHK005 - ChildControlResult(子任务控制结果) 的所有字段(child_id/attempt/status/stop_state/cancel_delivered/idempotent)是否在需求中完整定义？[Completeness, Spec §FR-003]
- [x] CHK006 - 无活动尝试时控制命令的行为(NoActiveAttempt 语义)是否在需求中完整定义？[Completeness, Spec §Edge Cases]
- [x] CHK007 - restart_limit(重启次数限制) 的配置来源优先级链(child override/group/supervisor/PolicyConfig)是否在需求中记录？[Completeness, Spec §Assumptions]

## Requirement Clarity(需求清晰度)

- [x] CHK008 - "未收到心跳" 和 "心跳超时" 的区分阈值是否在需求中量化为具体的时间窗口？[Clarity, Spec §US-1, AS-3]
- [x] CHK009 - "幂等" 的判定边界是否明确：何种条件下 RemoveChild 的首次删除算非幂等、重复删除算幂等？[Clarity, Spec §SC-003]
- [x] CHK010 - "reconcile_stop_deadlines(调和停止截止时间)" 的触发条件和执行逻辑是否在需求中明确？[Clarity, Spec §Edge Cases]
- [x] CHK011 - "停止等待窗口" 与 ShutdownPolicy.graceful_timeout 的映射规则是否在需求中明确？[Clarity, Spec §Assumptions]
- [x] CHK012 - ManagedChildState(受管子任务状态) 的派生映射规则(从 ChildRuntimeRecord 映射到外显枚举)是否在需求中明确？[Clarity, Spec §Assumptions]

## Requirement Consistency(需求一致性)

- [x] CHK013 - FR-002 中 "只使用取消和等待" 与 Assumptions 中 "控制命令超时后标记 Failed 不升级为强制中止" 是否与 004-4 的 FR-004(强制中止路径)无冲突？[Consistency, Spec §FR-002 vs Assumptions]
- [x] CHK014 - SC-001 的 "1 毫秒" 性能目标与 Edge Cases 中 "首次 heartbeat 没送达时区分未收到和超时" 的计算开销是否可同时满足？[Consistency, Spec §SC-001 vs Edge Cases]
- [x] CHK015 - FR-001 的外部读取 "不得暴露 raw handle" 与 FR-003 "包含目标 attempt 标识" 是否意味着 attempt 标识是公开 ID 而非句柄？[Consistency, Spec §FR-001 vs FR-003]
- [x] CHK016 - Assumptions 中 "本规格不要求新增动态子任务声明格式" 与 FR-002 "控制命令作用于真实生命周期" 在 RemoveChild 需要知道子任务声明的情况下是否矛盾？[Consistency, Spec §FR-002 vs Assumptions]

## Acceptance Criteria Quality(验收标准质量)

- [x] CHK017 - US-1 中 "连续 20 次构造 CurrentState 调用结果每次低于 1 毫秒" 是否考虑了第一次构造的冷启动开销？[Measurability, Spec §SC-001]
- [x] CHK018 - SC-002 "100% 观察到 cancellation_token 送达" 的验证是否需要检查 cancellation_token 的内部计数器还是外部确认消息？[Measurability, Spec §SC-002]
- [x] CHK019 - SC-003 "重复执行同一停止类控制命令 10 次" 是否要求覆盖不同停止状态(Failed/Aborted/NoActiveAttempt)下的幂等？[Measurability, Spec §SC-003]
- [x] CHK020 - SC-004 "控制结果包含 operation_after" 的 "命令后操作" 是否与 FR-003 定义的控制结果字段列表一致？[Measurability, Spec §SC-004 vs FR-003]

## Scenario Coverage(场景覆盖)

- [x] CHK021 - 主流程(健康子任务状态读取)的验收场景已在需求中完整覆盖。[Coverage, Spec §US-1]
- [x] CHK022 - 异常流程(子任务忽略取消后 Failed)的验收场景已在需求中覆盖。[Coverage, Spec §US-2 AS-2]
- [x] CHK023 - 幂等场景(重复发送取消)的验收场景已在需求中覆盖。[Coverage, Spec §US-3 AS-1]
- [x] CHK024 - 控制命令与自动重启并发时的裁决策略验收场景是否在需求中覆盖？[Coverage, Spec §Edge Cases]
- [x] CHK025 - restart_limit 耗尽时控制结果的剩余次数展示验收场景是否在需求中覆盖？[Coverage, Spec §US-3 AS-3]

## Edge Case Coverage(边界情况覆盖)

- [x] CHK026 - 运行状态记录刚刚启动、首次 heartbeat 尚未送达时的区分逻辑是否在需求中定义？[Edge Case, Spec §Edge Cases]
- [x] CHK027 - 任务上报 heartbeat 后立即退出时的最终结果优先级规则是否在需求中定义？[Edge Case, Spec §Edge Cases]
- [x] CHK028 - 自动重启推进到新 generation 后控制命令目标修正逻辑是否在需求中定义？[Edge Case, Spec §Edge Cases]
- [x] CHK029 - readiness 退化和从未上报的区分规则是否在需求中定义？[Edge Case, Spec §Edge Cases]
- [x] CHK030 - 占位 child 尚无活动 attempt 时停止类命令的 NoActiveAttempt 语义是否在需求中完整定义？[Edge Case, Spec §Edge Cases]

## Non-Functional Requirements(非功能需求)

- [x] CHK031 - CurrentState 构造的 1 毫秒性能目标是否在需求中作为非功能约束明确声明？[NFR, Spec §SC-001]
- [x] CHK032 - 串行策略(RUST_TEST_THREADS=1)的使用条件是否在需求中明确说明？[NFR, Spec §SC-001]
- [x] CHK033 - 控制命令处理的时间上限(从收到命令到发出取消信号)是否在需求中定义？[Gap]

## Dependencies & Assumptions(依赖与假设)

- [x] CHK034 - 本规格对 `004-2-real-shutdown-pipeline` 取消和等待语义的依赖是否在需求中显式声明？[Dependency, Spec §Assumptions]
- [x] CHK035 - RuntimeControlState 不再保留独立 children 映射的前提是否在需求中作为不变式记录？[Assumption, Spec §Assumptions]
- [x] CHK036 - PolicyEngine 是无状态结构的假设是否在需求中记录？[Assumption, Spec §Assumptions]

## Ambiguities & Conflicts(歧义与冲突)

- [x] CHK037 - "等待结果" 在 FR-002 和 FR-003 中的含义是否一致(异步可观察停止进度 vs 控制路径上的阻塞等待)？[Ambiguity, Spec §FR-002 vs FR-003]
- [x] CHK038 - "stop_state = NoActiveAttempt" 与 FR-001 的 "generation/attempt 显式为 None" 的上层语义是否要求一个统一的 "无活动尝试" 结构？[Ambiguity, Spec §FR-001 vs SC-004]
- [x] CHK039 - lazy-only 语义下 "没有后续入口进入 control loop 时失败事件不会自动发布" 与操作者期望的 "控制命令必须反映真实状态" 是否存在时序歧义？[Ambiguity, Spec §Edge Cases]

## Constitution Compliance(宪章合规)

- [x] CHK040 - 模块所有权已明确：runtime 拥有运行状态记录和句柄，control 拥有公开命令接口和结果类型。[Constitution, Spec §Module Ownership]
- [x] CHK041 - 诊断覆盖已定义：子任务尝试状态变化、取消送达、控制命令结果、heartbeat/readiness/restart_limit 更新。[Constitution, Spec §Diagnostics]
- [x] CHK042 - 中文写作格式合规，英文术语使用 `English(中文说明)` 格式。[Constitution, Spec §Chinese Writing]
- [x] CHK043 - 生命周期影响已记录：暂停/移除/隔离/自动重启绑定到运行状态记录的真实活动尝试。[Constitution, Spec §Lifecycle Impact]

## Notes(说明)

- 本 checklist 基于 `004-3-child-runtime-state-control/spec.md` 生成，覆盖需求完整性、清晰度、一致性、可测性、场景覆盖和边界情况。
- 旧版通用格式 checklist 已替换为本 requirements quality checklist。
- 完成检查项后使用 `[x]` 标记。
