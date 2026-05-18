# Requirements Quality Checklist: 代次隔离重启

**Purpose(目的)**: 验证 `004-4-generation-fencing` 规格中代次隔离重启需求的质量、完整性和一致性，确保重启语义可实施、可测试、无歧义。
**Created(创建日期)**: 2026-05-19
**Feature(功能)**: [spec.md](../spec.md)

## Requirement Completeness(需求完整性)

- [x] CHK001 - RestartChild(重启子任务) 的执行步骤(先取消旧尝试、等待退出/超时、生成新 generation、启动新 attempt)是否在需求中完整定义？[Completeness, Spec §FR-001]
- [x] CHK002 - 手动重启和自动重启使用同一个代次隔离规则的约束是否在需求中显式声明？[Completeness, Spec §FR-002]
- [x] CHK003 - stale report(过期报告) 的判定条件(旧 generation vs 当前 generation)和处理方式(丢弃 vs 标注)是否在需求中完整定义？[Completeness, Spec §FR-003]
- [x] CHK004 - 旧尝试拒绝响应取消时的强制中止路径是否在需求中定义？[Completeness, Spec §Edge Cases]
- [x] CHK005 - 新尝试启动失败时运行状态记录的行为(保留旧尝试最终结果和新尝试失败原因)是否在需求中定义？[Completeness, Spec §Edge Cases]
- [x] CHK006 - RunningInstanceId(运行实例标识) 与 (child_id, generation, attempt) 三元组的关系是否在需求中定义？[Completeness, Spec §RunningInstanceId]
- [x] CHK007 - DelayedSpawnAttached(延迟附着启动子任务消息) 邮箱变体的用途、触发条件和生命周期是否在需求中完整定义？[Completeness, Spec §FR-004, Key Entities]

## Requirement Clarity(需求清晰度)

- [x] CHK008 - "先停止旧 attempt" 的停止方式(取消令牌 + 等待 vs 取消令牌 + 超时中止)是否在需求中明确？[Clarity, Spec §FR-001]
- [x] CHK009 - "拒绝、合并或排队" 三种冲突处理策略的选择条件是否在需求中明确？[Clarity, Spec §US-2 AS-1]
- [x] CHK010 - "保留一个明确的活动尝试" 中 "保留" 语义——保留旧尝试还是新尝试——是否在需求中明确？[Clarity, Spec §US-2 AS-2]
- [x] CHK011 - "正的 backoff" 的数值范围和配置来源是否在需求中明确？[Clarity, Spec §FR-004]
- [x] CHK012 - "不得在未进入 control loop 的普通任务里单独 spawn 新尝试" 的实现约束是否在需求中明确可验证？[Clarity, Spec §FR-004]

## Requirement Consistency(需求一致性)

- [x] CHK013 - FR-001 "先停止旧 attempt" 与 Assumptions "重启前停止旧尝试必须复用关闭流水线的取消/等待/中止" 的 "中止" 是否与 004-3 中 "控制命令不自动升级为强制中止" 的规则一致？[Consistency, Spec §FR-001 vs Assumptions, 004-3 Assumptions]
- [x] CHK014 - FR-004 的 "DelayedSpawnAttached 邮箱路径" 与 FR-002 "最多一个 active attempt" 在 backoff 期间没有活动尝试时的语义是否一致？[Consistency, Spec §FR-004 vs FR-002]
- [x] CHK015 - SC-005 的 "100% 观察到 control loop 附着 activate_instance" 与 FR-004 "不得只在独立 async task 中收尾" 是否构成同一约束的两种表达？[Consistency, Spec §SC-005 vs FR-004]
- [x] CHK016 - RunningInstanceId 统称与 (generation, attempt) 两轴的关系在 FR-001 "新 generation 和 attempt" 中的使用是否一致？[Consistency, Spec §RunningInstanceId vs FR-001]

## Acceptance Criteria Quality(验收标准质量)

- [x] CHK017 - US-1 "新尝试只有在旧尝试进入停止结果后才会启动" 的验证手段(检查事件顺序 vs 检查运行状态记录)是否在需求中指定？[Measurability, Spec §US-1]
- [x] CHK018 - SC-001 "同一时刻最多只有一个 active attempt" 的并发验证是否要求模拟时序竞争条件？[Measurability, Spec §SC-001]
- [x] CHK019 - SC-003 "100% 测试场景不会覆盖" 是否要求覆盖旧尝试退出消息在网络延迟/重排序下的场景？[Measurability, Spec §SC-003]
- [x] CHK020 - SC-005 中 "activate_instance 事实" 的可观测证据(日志行、事件类型、运行状态记录字段)是否在需求中定义？[Measurability, Spec §SC-005]

## Scenario Coverage(场景覆盖)

- [x] CHK021 - 主流程(重启时先停止旧尝试再启动新尝试)的验收场景已在需求中完整覆盖。[Coverage, Spec §US-1]
- [x] CHK022 - 异常流程(旧尝试拒绝取消后超时中止)的验收场景已在需求中覆盖。[Coverage, Spec §Edge Cases]
- [x] CHK023 - 自动重启 and 手动重启并发的裁决策略验收场景是否在需求中覆盖？[Coverage, Spec §US-2]
- [x] CHK024 - 旧 generation 迟到报告的丢弃 vs 标注验收场景是否在需求中覆盖？[Coverage, Spec §US-3]
- [x] CHK025 - 带正 backoff 的手动重启验收场景是否在需求中覆盖？[Coverage, Spec §FR-004, SC-005]

## Edge Case Coverage(边界情况覆盖)

- [x] CHK026 - 旧尝试拒绝响应取消时强制中止的执行主体(ShutdownPipeline vs RestartChild)是否在需求中明确？[Edge Case, Spec §Edge Cases]
- [x] CHK027 - 新尝试启动失败后运行状态记录的行为(保留旧尝试最终结果 + 新尝试失败原因)是否在需求中定义？[Edge Case, Spec §Edge Cases]
- [x] CHK028 - 重启请求到达时旧尝试恰好自然退出的竞态条件是否在需求中定义处理逻辑？[Edge Case, Gap]
- [x] CHK029 - backoff 期间操作者再次执行 RestartChild 的行为(覆盖 backoff vs 排队)是否在需求中定义？[Edge Case, Gap]

## Non-Functional Requirements(非功能需求)

- [x] CHK030 - RestartChild 命令的执行耗时上限(从收到命令到新尝试启动)是否在需求中定义？[Gap]
- [x] CHK031 - generation/attempt 溢出处理(超过 u32 MAX)是否在需求中定义？[Gap]
- [x] CHK032 - delayed spawn 邮箱消息的积压上限是否在需求中定义？[Gap, Spec §FR-004]

## Dependencies & Assumptions(依赖与假设)

- [x] CHK033 - 本规格对 `004-3-child-runtime-state-control` 运行状态记录和真实活动尝试状态的依赖是否在需求中显式声明？[Dependency, Spec §Assumptions]
- [x] CHK034 - "不要求支持多个并行实例的同名 child" 的范围边界是否在需求中显式声明？[Assumption, Spec §Assumptions]
- [x] CHK035 - 手动重启和自动重启使用同一个代次隔离规则的假设是否在需求中记录？[Assumption, Spec §Assumptions]

## Ambiguities & Conflicts(歧义与冲突)

- [x] CHK036 - "强制中止路径" (Edge Cases) 与 Assumptions "重启前停止旧尝试必须复用关闭流水线" 的关系——是指 RestartChild 委托给 ShutdownPipeline 还是调用其取消原语？[Ambiguity, Spec §Edge Cases vs Assumptions]
- [x] CHK037 - "冲突或排队处理结论"(SC-004) 中 "排队" 是否等价于 FR-004 的 delayed spawn 路径？[Ambiguity, Spec §SC-004 vs FR-004]
- [x] CHK038 - "绑定启动实例" (FR-004) 与 "新 generation 和 attempt" (FR-001) 的绑定时机——delayed spawn 完毕后立即绑定还是进入 control loop 后绑定？[Ambiguity, Spec §FR-004 vs FR-001]

## Constitution Compliance(宪章合规)

- [x] CHK039 - 模块所有权已明确：runtime(运行时) 负责代次隔离和报告接收，policy(策略) 只产出重启决策。[Constitution, Spec §Module Ownership]
- [x] CHK040 - 诊断覆盖已定义：重启请求、旧尝试停止结果、新代次启动、重启冲突和 stale report。[Constitution, Spec §Diagnostics]
- [x] CHK041 - 中文写作格式合规，英文术语使用 `English(中文说明)` 格式。[Constitution, Spec §Chinese Writing]
- [x] CHK042 - RunningInstanceId(运行实例标识) 统称已在宪章映射节中与功能术语对齐。[Constitution, Spec §RunningInstanceId]

## Notes(说明)

- 本 checklist 基于 `004-4-generation-fencing/spec.md` 生成，覆盖需求完整性、清晰度、一致性、可测性、场景覆盖和边界情况。
- 旧版通用格式 checklist 已替换为本 requirements quality checklist。
- 完成检查项后使用 `[x]` 标记。
