# Requirements Quality Checklist: 真实关闭流水线

**Purpose(目的)**: 验证 `004-2-real-shutdown-pipeline` 规格中关闭流水线需求的质量、完整性和一致性，确保关闭语义可实施、可测试、无歧义。
**Created(创建日期)**: 2026-05-19
**Feature(功能)**: [spec.md](../spec.md)

## Requirement Completeness(需求完整性)

- [x] CHK001 - 四个关闭阶段(Idle/RequestStop/GracefulDrain/AbortStragglers/Reconcile)的进入条件、执行内容、完成条件和转换规则是否完整？[Completeness, Spec §Key Entities]
- [x] CHK002 - CancellationToken(取消令牌) 的创建时机、分发范围(每个运行中任务)和送达确认方式是否在需求中定义？[Completeness, Spec §FR-001]
- [x] CHK003 - shutdown_order(关闭顺序) 的计算依据(依赖关系 vs 声明顺序)和排序算法是否在需求中指定？[Completeness, Spec §FR-002]
- [x] CHK004 - abort stragglers(强制中止滞留任务) 的超时来源(关闭预算)和强制中止的执行方式是否完整定义？[Completeness, Spec §FR-003]
- [x] CHK005 - reconcile(状态对账) 阶段中注册表清理、journal(日志)记录、metrics(指标)输出和 socket(套接字)对账的具体操作是否完整罗列？[Completeness, Spec §FR-003]
- [x] CHK006 - ChildShutdownOutcome(子任务关闭结果) 的四种变体(Graceful/Aborted/AbortFailed/AlreadyExited)的判定条件是否在需求中定义？[Completeness, Spec §Key Entities]
- [x] CHK007 - ShutdownReconcileReport(关闭对账报告) 中 "NotOwned(非运行时拥有)" 的判定规则是否在需求中明确？[Completeness, Spec §FR-003]

## Requirement Clarity(需求清晰度)

- [x] CHK008 - "关闭预算" 的具体数值来源和配置方式是否在需求中明确？[Clarity, Spec §Assumptions]
- [x] CHK009 - "迟到报告" 的窗口期和判定标准是否在需求中定义？[Clarity, Spec §Edge Cases]
- [x] CHK010 - "同一个关闭结果" 的定义：结构相等还是引用相等？[Clarity, Spec §Edge Cases]
- [x] CHK011 - "每个任务的退出结果" 是否包含退出码、退出原因和退出时间？[Clarity, Spec §FR-002]

## Requirement Consistency(需求一致性)

- [x] CHK012 - FR-001 的 "取消送达的任务集合" 与 FR-002 的 "按 shutdown_order 等待" 在任务尚未启动时的行为是否一致？[Consistency, Spec §FR-001 vs FR-002]
- [x] CHK013 - Edge Cases 中 "没有运行中任务时" 的记录为 AlreadyExited，与 US-1 的 "不得重复取消已经结束的任务" 是否自洽？[Consistency, Spec §Edge Cases vs US-1]
- [x] CHK014 - Key Entities 中 ShutdownPipeline 的执行阶段定义(6 个状态)与 FR-045 的四阶段关闭协议口径是否对齐？[Consistency, Spec §Key Entities vs 001 FR-045]

## Acceptance Criteria Quality(验收标准质量)

- [x] CHK015 - US-1 验收场景中 "观察取消信号" 的验证手段(测试断言、事件输出、hook)是否在需求中指定？[Measurability, Spec §US-1]
- [x] CHK016 - US-2 中 "退出分类" 的验证标准是否在需求中定义？[Measurability, Spec §US-2]
- [x] CHK017 - SC-001 "100% 的运行中任务" 是否要求测试用例覆盖不同关闭顺序和依赖拓扑？[Measurability, Spec §SC-001]
- [x] CHK018 - SC-003 中 "非优雅结束" 的验证是否要求同时检查 ChildShutdownOutcome 和 ReconcileReport 的一致性？[Measurability, Spec §SC-003]

## Scenario Coverage(场景覆盖)

- [x] CHK019 - 主流程(正常关闭、所有任务优雅退出)的验收场景已在需求中完整覆盖。[Coverage, Spec §US-1]
- [x] CHK020 - 异常流程(任务忽略取消信号超时后中止)的验收场景已在需求中覆盖。[Coverage, Spec §US-3]
- [x] CHK021 - 空关闭(没有运行中任务)的验收场景是否在需求中覆盖？[Coverage, Spec §Edge Cases]
- [x] CHK022 - 重复关闭请求的幂等验收场景是否在需求中覆盖？[Coverage, Spec §Edge Cases]
- [x] CHK023 - 关闭期间任务迟到上报的验收场景是否在需求中覆盖？[Coverage, Spec §Edge Cases]

## Edge Case Coverage(边界情况覆盖)

- [x] CHK024 - 所有运行中任务已结束时，AbortStragglers 阶段是否可跳过或在需求中定义行为？[Edge Case, Spec §FR-003]
- [x] CHK025 - 部分任务成功、部分超时、部分已退出的混合场景是否在需求中定义结果聚合规则？[Edge Case, Gap]
- [x] CHK026 - 关闭预算为 0 时的行为(立即中止)是否在需求中定义？[Edge Case, Gap]
- [x] CHK027 - socket(套接字) 对账失败时(本应 NotOwned 但实际存在脏数据)的降级行为是否在需求中定义？[Edge Case, Spec §FR-003]

## Non-Functional Requirements(非功能需求)

- [x] CHK028 - 关闭流水线执行的总耗时上限是否在需求中定义？[Gap]
- [x] CHK029 - 关闭过程中可观测性(事件、指标、日志)的写入保证(至少一次、至多一次)是否在需求中指定？[Gap]
- [x] CHK030 - 关闭并行度(能否同时中止多个滞留任务)是否在需求中定义？[Gap]

## Dependencies & Assumptions(依赖与假设)

- [x] CHK031 - 本规格对 `004-1-runtime-lifecycle-guard` 运行时健康和等待语义的依赖是否在需求中显式声明？[Dependency, Spec §Assumptions]
- [x] CHK032 - ShutdownCoordinator(关闭协调器) "不直接拥有任务句柄" 的假设是否在需求中作为不变式记录？[Assumption, Spec §Assumptions]
- [x] CHK033 - "只改变关闭执行语义，不改变监督策略的重启决策" 的范围边界是否在需求中显式声明？[Assumption, Spec §Assumptions]

## Ambiguities & Conflicts(歧义与冲突)

- [x] CHK034 - "同一关闭结果" 与 "当前关闭进度" 的关系是否明确——后者是否包含已完成的阶段数据？[Ambiguity, Spec §Edge Cases]
- [x] CHK035 - "记录为迟到报告" 的 is- 如何区分迟到报告与正常退出消息：关闭期间的 vs 关闭完成后的？[Ambiguity, Spec §Edge Cases]
- [x] CHK036 - AbortFailed(强制中止失败) 的判定条件是否与 FR-003 的 "清理运行时拥有的资源" 在范围上不冲突？[Ambiguity, Spec §FR-003 vs Key Entities]

## Constitution Compliance(宪章合规)

- [x] CHK037 - 模块所有权已明确：shutdown(关闭) 模块保留阶段契约，runtime(运行时) 模块拥有句柄和执行。[Constitution, Spec §Module Ownership]
- [x] CHK038 - 诊断覆盖已定义：关闭阶段变化、每个 child 的取消送达/等待完成/超时/强制中止。[Constitution, Spec §Diagnostics]
- [x] CHK039 - 中文写作格式合规，英文术语使用 `English(中文说明)` 格式。[Constitution, Spec §Chinese Writing]
- [x] CHK040 - 生命周期影响已记录：关闭语义从阶段推进变为真实停止与清理。[Constitution, Spec §Lifecycle Impact]

## Notes(说明)

- 本 checklist 基于 `004-2-real-shutdown-pipeline/spec.md` 生成，覆盖需求完整性、清晰度、一致性、可测性、场景覆盖和边界情况。
- 旧版通用格式 checklist 已替换为本 requirements quality checklist。
- 完成检查项后使用 `[x]` 标记。
