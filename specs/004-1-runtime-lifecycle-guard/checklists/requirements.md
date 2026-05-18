# Requirements Quality Checklist: 运行时生命周期守卫

**Purpose(目的)**: 验证 `004-1-runtime-lifecycle-guard` 规格中需求的质量、完整性和清晰度，确保需求可实施、可测试、无歧义。
**Created(创建日期)**: 2026-05-19
**Feature(功能)**: [spec.md](../spec.md)

## Requirement Completeness(需求完整性)

- [x] CHK001 - 是否明确定义了 `RuntimeControlPlane`(运行时控制面) 的生命周期阶段及其转换条件？[Completeness, Spec §Key Entities]
- [x] CHK002 - `RuntimeWatchdog`(运行时看门狗) 的启动时机、观测范围和失败处理策略是否完整？[Completeness, Spec §FR-002]
- [x] CHK003 - `RuntimeHealthReport`(运行时健康报告) 的结构化字段(阶段、退出原因、可恢复性)是否完整覆盖所有退出场景？[Completeness, Spec §FR-002, Edge Cases]
- [x] CHK004 - 是否定义了 `is_alive`、`health`、`join`、`shutdown` 这四个语义各自的返回值类型和约束？[Completeness, Spec §FR-003]
- [x] CHK005 - 控制循环启动后立即退出的场景中，故障事件和健康状态的产生路径是否在需求中明确？[Completeness, Spec §Edge Cases]
- [x] CHK006 - watchdog(看门狗) 自身无法发布事件时的降级行为是否已在需求中定义？[Completeness, Spec §Edge Cases]

## Requirement Clarity(需求清晰度)

- [x] CHK007 - "alive"(存活) 和 "not alive"(非存活) 的判定标准是否量化为可测量的条件？[Clarity, Spec §FR-001]
- [x] CHK008 - "结构化故障信号"的具体载荷字段和格式是否在需求中明确？[Clarity, Spec §FR-002]
- [x] CHK009 - "幂等"的边界是否明确：何种条件下的重复调用算幂等？[Clarity, Spec §FR-003, US-3]
- [x] CHK010 - "可恢复"和"不可恢复"的退出原因分类标准是否在需求中定义？[Clarity, Spec §FR-002]

## Requirement Consistency(需求一致性)

- [x] CHK011 - US-1 中健康状态返回的字段列表与 FR-001/FR-003 中定义的语义是否一致？[Consistency]
- [x] CHK012 - Edge Cases 中 "watchdog 自身无法发布事件" 与 FR-002 "必须发出事件" 是否存在策略冲突？[Consistency, Spec §FR-002 vs Edge Cases]
- [x] CHK013 - SC-001 的 "下一次控制命令发送前" 与 SC-002 的 "没有新控制命令时也发出事件" 是否需统一时间线表述？[Consistency, Spec §SC-001 vs SC-002]

## Acceptance Criteria Quality(验收标准质量)

- [x] CHK014 - US-1 的验收场景是否指定了可验证的断言条件(如响应字段的类型和范围)？[Measurability, Spec §US-1]
- [x] CHK015 - US-2 中 "异常退出" 是否定义了测试可构造的触发方式(如注入 panic 或通道关闭)？[Measurability, Spec §US-2]
- [x] CHK016 - SC-003 的 "1 秒内" 是否有测试环境时钟精度要求？[Measurability, Spec §SC-003]
- [x] CHK017 - SC-001 "100% 的测试场景" 是否定义了最小测试用例数量和覆盖维度？[Measurability, Spec §SC-001]

## Scenario Coverage(场景覆盖)

- [x] CHK018 - 主流程(正常启动、健康查询)的验收场景已在需求中完整覆盖。[Coverage, Spec §US-1]
- [x] CHK019 - 异常流程(控制循环异常退出、watchdog 处理失败)的验收场景是否覆盖所有退出模式？[Coverage, Spec §US-2]
- [x] CHK020 - 恢复流程(控制循环重复启动)的验收场景是否定义？[Gap, Spec §Assumptions]
- [x] CHK021 - 非功能场景(健康查询的延迟上限、事件发布的可靠性)是否已在需求中定义？[Coverage, Gap]

## Edge Case Coverage(边界情况覆盖)

- [x] CHK022 - 控制循环启动后立即退出的边界条件是否已在需求中处理？[Edge Case, Spec §Edge Cases]
- [x] CHK023 - watchdog(看门狗) 自身崩溃或事件通道关闭的边界条件是否已在需求中处理？[Edge Case, Spec §Edge Cases]
- [x] CHK024 - 操作者在控制循环结束后发送命令时，已知退出原因的结构化错误格式是否已在需求中定义？[Edge Case, Spec §Edge Cases]

## Non-Functional Requirements(非功能需求)

- [x] CHK025 - 健康查询的响应时间要求是否在需求中定义？[Gap]
- [x] CHK026 - 事件发布的可靠性语义(至少一次、至多一次)是否在需求中指定？[Gap]
- [x] CHK027 - 控制面关闭的等待超时阈值是否在需求中定义？[Gap, Spec §FR-003]

## Dependencies & Assumptions(依赖与假设)

- [x] CHK028 - 本规格不覆盖 relay(中继) 和 dashboard client(看板客户端) 的范围边界是否在需求中显式声明？[Assumption, Spec §Assumptions]
- [x] CHK029 - "不默认自动重启控制循环" 的前提是否在需求中作为不变式明确记录？[Assumption, Spec §Assumptions]
- [x] CHK030 - 与后续规格(004-2 关闭流水线、004-3 状态控制)的接口边界是否在需求中定义？[Dependency, Gap]

## Ambiguities & Conflicts(歧义与冲突)

- [x] CHK031 - "异常退出" 的判定标准是否涵盖 panic、通道关闭、任务取消和超时四种模式？[Ambiguity, Spec §US-2]
- [x] CHK032 - Health(健康) 与 Alive(存活) 的关系是否明确——健康是否隐含存活？[Ambiguity, Spec §FR-001 vs FR-003]
- [x] CHK033 - "最近观测时间" 的更新时机(每次 watchdog 观测还是 health 查询时)是否明确？[Ambiguity, Spec §US-1]

## Constitution Compliance(宪章合规)

- [x] CHK034 - 模块所有权已明确：runtime(运行时) 拥有控制面生命周期，control(控制) 只暴露句柄能力。[Constitution, Spec §Module Ownership]
- [x] CHK035 - 诊断覆盖已定义：控制循环启动/正常退出/异常退出/等待完成/关闭请求的结构化诊断。[Constitution, Spec §Diagnostics]
- [x] CHK036 - 中文写作格式合规，英文术语使用 `English(中文说明)` 格式。[Constitution, Spec §Chinese Writing]

## Notes(说明)

- 本 checklist 基于 `004-1-runtime-lifecycle-guard/spec.md` 生成，覆盖需求完整性、清晰度、一致性、可测性、场景覆盖和边界情况。
- 旧版通用格式 checklist 已替换为本 requirements quality checklist。
- 完成检查项后使用 `[x]` 标记。
