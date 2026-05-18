# Drift Resolution Proposals

Generated: 2026-05-18T00:00:00Z
Based on: drift-report from 2026-05-18T00:00:00Z

## Summary

| Resolution Type | Count |
|-----------------|-------|
| Align (Spec → Code) | 4 |
| Backfill (Code → Spec) | 3 |
| Human Decision | 0 |
| New Specs | 0 |
| Remove from Spec | 0 |
| **Total** | **7** |

**Resolution Mode**: 自动批准 (auto-approve)

---

## Proposals

### Proposal 1: 006-4-restart-policy-production / SC-000

**Direction**: ALIGN (Spec → Code)

**Current State**:
- Spec says: "所有策略决策路径(预算通过, 预算耗尽, 熔断触发, 升级分叉)均可被 typed event(类型化事件) 日志完整复现"
- Code does: `stage_emit_typed_event` 仅发射泛型 `What::ChildFailed`, 不发射 `What::BudgetExhausted`, `What::GroupFuseTriggered`, `What::EscalationBifurcated` 等具体策略事件

**Proposed Resolution**:

改造 `stage_emit_typed_event` 方法，根据 `ctx.budget_evaluation.meltdown_outcome` 和 `ctx.effective_policy.severity` 选择正确的 `What` 变体:

```rust
// 新增辅助函数
fn build_policy_aware_what(ctx: &PipelineContext, exit: &TaskExit) -> What {
    let budget_exhausted = ctx.budget_evaluation.as_ref().map(|be|
        matches!(be.meltdown_outcome, MeltdownOutcome::GroupFuse)
    );
    // 根据 ctx 中的评估结果选择具体的 What 变体
    match exit {
        TaskExit::Succeeded => What::ChildRunning { transition: None },
        TaskExit::Failed { .. } => {
            // TODO: Check if budget was exhausted → emit BudgetExhausted
            // TODO: Check if group fuse triggered → emit GroupFuseTriggered
            // TODO: Check severity → emit EscalationBifurcated
            What::ChildFailed {
                failure: TaskFailure::new(
                    crate::error::types::TaskFailureKind::Error,
                    "pipeline_exit",
                    "processed through six-stage pipeline",
                ),
            }
        }
    }
}
```

**Rationale**: 规格是经过审核的架构决策。pipeline 应该使用 `ctx` 中已有的评估结果来选择 typed event 变体，而不是每类失败都走 `ChildFailed`。

**Confidence**: HIGH

**Action**:
- [x] **自动批准** — Align: 修复代码

---

### Proposal 2: 006-4-restart-policy-production / FR-003 (CorrelationId)

**Direction**: ALIGN (Spec → Code)

**Current State**:
- Spec says: "CorrelationId(关联标识) 在评估管线入口生成, 贯穿整个故障链路(budget → meltdown → backoff → escalation), 即使中间某阶段被跳过也继续传递"
- Code does: `stage_emit_typed_event` 在构造 `SupervisorEvent` 时硬编码 `CorrelationId::from_uuid(uuid::Uuid::nil())`, 忽略了 `ctx.correlation_id`

**Proposed Resolution**:

将 `stage_emit_typed_event` 中构造 `SupervisorEvent` 的 CorrelationId 改为使用 `ctx.correlation_id`:

```rust
let event_correlation_id = uuid::Uuid::parse_str(&ctx.correlation_id)
    .map(CorrelationId::from_uuid)
    .unwrap_or_else(|_| CorrelationId::from_uuid(uuid::Uuid::nil()));

let mut event = SupervisorEvent::new(
    // ...timing...,
    location,
    what,
    EventSequence::new(ctx.sequence),
    event_correlation_id,
    1,
);
```

**Rationale**: 规格明确要求 CorrelationId 贯穿全链路。控制回路 (T037) 已生成真实 UUID 并通过 `PipelineContext.correlation_id` 传入，但 emit 阶段未使用。

**Confidence**: HIGH

**Action**:
- [x] **自动批准** — Align: 修复代码

---

### Proposal 3: 006-4-restart-policy-production / FR-001 (FairnessProbe typed event)

**Direction**: ALIGN (Spec → Code)

**Current State**:
- Spec says: fairness(公平性) 探针产物应能被 typed event 通道消费(隐含: `StarvationAlert` 应通过 typed event 发射)
- Code does: `check_fairness_probe` 仅通过 `event_sender.send()` 发纯文本消息 `"fairness_starvation:..."`, 未发射 typed event

**Proposed Resolution**:

1. 在 `src/event/payload.rs` 的 `What` 枚举中新增变体:
   ```rust
   FairnessProbeStarvation {
       starved_child_id: ChildId,
       skip_count: u64,
       probe_start_unix_nanos: u128,
       probe_end_unix_nanos: u128,
   },
   ```
2. 在 `check_fairness_probe` 中构造 `PendingRuntimeEvent` 而非纯文本 emit
3. 在 `What::name()` 中添加对应分支

**Rationale**: 规格要求所有策略决策通过 typed event 通道。文本消息无法被结构化订阅者消费。

**Confidence**: HIGH

**Action**:
- [x] **自动批准** — Align: 修复代码

---

### Proposal 4: 006-4-restart-policy-production / SC-003

**Direction**: ALIGN (Spec → Code) — 推迟实现

**Current State**:
- Spec says: "typed event(类型化事件) 与 metrics(指标) 针对同一 SupervisorDecision 键 (child_id, decision_type, correlation_id) 的一致率抽检样本不低于 98%"
- Code does: 现有测试 `test_correlation_id_present_in_pipeline_context` 仅验证硬编码 `"corr-001"`, 无事件/指标逐条比对测试

**Proposed Resolution**:

标记为 **known gap** (已知缺口), 推迟到 `006-5-typed-events-observability` 切片中实现联合验证。理由是:
- SC-003 依赖完整的 event + metrics 双通道基础设施
- 当前切片优先保证 typed event 发射正确
- metrics 一致率需 metrics 采样子系统就绪后验证

**Rationale**: 避免在当前迭代中增加过多 scope。

**Confidence**: MEDIUM

**Action**:
- [x] **自动批准** — 标记 known gap, 推迟到 006-5

---

### Proposal 5: emit_policy_diagnostic (T042) — 回填 Spec

**Direction**: BACKFILL (Code → Spec)

**Current State**:
- Code does: 在 `src/observe/pipeline.rs` 中实现了 `emit_policy_diagnostic()`, 为 `BudgetExhausted/GroupFuseTriggered/EscalationBifurcated` 事件生成 PipelineStageDiagnostic
- Spec does: 未在 spec.md 中显式提及

**Proposed Resolution**:

在 `spec.md` 的 Diagnostics 节补充:

> **Diagnostic Pipeline (诊断流水线)**: 当系统发射 BudgetExhausted, GroupFuseTriggered, EscalationBifurcated 三种 typed event 时, observability pipeline 自动为每个事件生成 PipelineStageDiagnostic, 包含事件序列号、CorrelationId、budget_evaluation 字段(携带预算耗尽退避时长/熔断传播来源/分叉严重程度). 诊断记录通过 TestRecorder.pipeline_stage_diagnostics 通道可消费.

**Rationale**: 代码已实现且通过测试 (T042), 属于有意的设计演进。

**Confidence**: HIGH

**Action**:
- [x] **自动批准** — Backfill: 更新 spec.md

---

### Proposal 6: GroupConfig (T039) — 回填 Spec

**Direction**: BACKFILL (Code → Spec)

**Current State**:
- Code does: 在 `src/spec/supervisor.rs` 中新增了 `GroupConfig` 结构体 (`name: String`, `children: Vec<ChildId>`, `budget: RestartBudgetConfig`), 同时在 `SupervisorSpec` 中新增了 `group_dependencies: Vec<GroupDependencyEdge>` 和 `severity_defaults: HashMap<WorkRole, SeverityClass>` 字段
- Spec does: Key Entities 节未列出 GroupConfig

**Proposed Resolution**:

在 `spec.md` 的 Key Entities 节补充:

> GroupConfig(分组配置): 定义分组名称、成员子任务列表、独立重启预算配置的结构体. 由 SupervisorSpec.group_configs 持有.
> GroupDependencyEdge(分组依赖边): 声明跨组故障传播关系的配置切片, 由 SupervisorSpec.group_dependencies 持有.
> SeverityDefaults(严重程度默认值): 按 WorkRole 映射默认 SeverityClass 的配置表, 由 SupervisorSpec.severity_defaults 持有.

**Rationale**: 代码已实现且通过编译 (T039), 属于有意的架构扩展。

**Confidence**: HIGH

**Action**:
- [x] **自动批准** — Backfill: 更新 spec.md Key Entities

---

### Proposal 7: ChildSpec.severity/group (T040) — 回填 Spec

**Direction**: BACKFILL (Code → Spec)

**Current State**:
- Code does: 在 `src/spec/child.rs` 的 `ChildSpec` 中新增了 `severity: Option<SeverityClass>` 和 `group: Option<String>` 可选字段
- Spec does: 未在 spec.md 中显式声明这两个字段

**Proposed Resolution**:

在 `spec.md` 的 Key Entities 节补充:

> ChildSpec.severity(子任务显式严重程度): 可选字段, 覆盖 WorkRole 默认的 SeverityClass 映射. 当同时存在 group 级默认值时, child 级显式值优先 (见 tie-break 规则表第 4 行).
> ChildSpec.group(子任务所属分组): 可选字段, 将 child 分配到命名分组, 用于 group-level budget 和熔断隔离.

**Rationale**: 代码已实现且通过编译 (T040), 属于有意的设计演进。

**Confidence**: HIGH

**Action**:
- [x] **自动批准** — Backfill: 更新 spec.md Key Entities
