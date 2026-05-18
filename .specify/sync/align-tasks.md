# Align Tasks — 代码修复任务

Generated: 2026-05-18T00:00:00Z
Source: `.specify/sync/proposals.json` (Proposals 1-3, approved)

---

## Task: ALIGN-001 — 修复 `stage_emit_typed_event` 发射策略事件

**Spec Requirement**: SC-000
**Proposal ID**: 1
**Direction**: ALIGN (Spec → Code)
**Priority**: P0
**Estimated Effort**: medium

### Current Code

`src/runtime/pipeline.rs:578-625` — `stage_emit_typed_event` 方法在所有情况下仅发射泛型 `What::ChildFailed`, 不区分预算耗尽、熔断触发、升级分叉等策略决策。

### Required Change

根据 `ctx.budget_evaluation`, `ctx.effective_policy.severity`, `ctx.scopes_triggered` 选择正确的 `What` 变体:

1. 当 `budget_verdict == Some(BudgetVerdict::Exhausted {..})` 时，发射 `What::BudgetExhausted { child_id, retry_after_ns, budget_source_group }`
2. 当 `meltdown_outcome == MeltdownOutcome::GroupFuse` 时，发射 `What::GroupFuseTriggered { group_name, propagated_from_group }`
3. 当 `ctx.effective_policy.severity == SeverityClass::Critical` 且未触发熔断时，发射 `What::EscalationBifurcated { severity, budget_verdict, fuse_outcome, tie_break_reason }`
4. 当 `ctx.effective_policy.severity == SeverityClass::Optional` 时，发射 `What::EscalationBifurcated` (降噪路径)
5. 兜底: 保留现有的 `What::ChildRunning`/`What::ChildFailed` 逻辑

### Files to Modify

- `src/runtime/pipeline.rs` — 修改 `stage_emit_typed_event` 和/或新增 `build_policy_aware_what()` 辅助函数
- `src/event/payload.rs` — 如果 `EscalationBifurcated` 字段需要补充 `SeverityClass` 枚举而非字符串

### Acceptance Criteria

- [ ] 预算耗尽时 `SupervisorEvent.what` 为 `What::BudgetExhausted`
- [ ] 分组熔断时 `SupervisorEvent.what` 为 `What::GroupFuseTriggered`
- [ ] Critical child 失败时 `SupervisorEvent.what` 为 `What::EscalationBifurcated { severity: "Critical", .. }`
- [ ] Optional child 降噪时 `SupervisorEvent.what` 为 `What::EscalationBifurcated { severity: "Optional", .. }`
- [ ] Standard child 失败时仍保留 `What::ChildFailed`
- [ ] `cargo test` 全量通过
- [ ] `cargo clippy -- -D warnings` 零警告

---

## Task: ALIGN-002 — 修复 `stage_emit_typed_event` 使用真实 CorrelationId

**Spec Requirement**: FR-003
**Proposal ID**: 2
**Direction**: ALIGN (Spec → Code)
**Priority**: P0
**Estimated Effort**: small

### Current Code

`src/runtime/pipeline.rs:609` — 构造 `SupervisorEvent` 时硬编码 `CorrelationId::from_uuid(uuid::Uuid::nil())`, 忽略 `ctx.correlation_id`

### Required Change

将 CorrelationId 的构造改为从 `ctx.correlation_id` 解析:

```rust
let event_correlation_id = uuid::Uuid::parse_str(&ctx.correlation_id)
    .map(CorrelationId::from_uuid)
    .unwrap_or_else(|_| CorrelationId::from_uuid(uuid::Uuid::nil()));
```

然后传给 `SupervisorEvent::new()`。

### Files to Modify

- `src/runtime/pipeline.rs` — 第 609 行附近

### Acceptance Criteria

- [ ] `SupervisorEvent.correlation_id` 不再为 nil（除非 parse 失败）
- [ ] `cargo test` 全量通过
- [ ] `cargo clippy -- -D warnings` 零警告

---

## Task: ALIGN-003 — 新增 `What::FairnessProbeStarvation` typed event

**Spec Requirement**: FR-001 (隐含 typed event 通道)
**Proposal ID**: 3
**Direction**: ALIGN (Spec → Code)
**Priority**: P1
**Estimated Effort**: medium

### Current Code

`src/runtime/control_loop.rs:1603` — `check_fairness_probe` 仅发文本事件 `"fairness_starvation:..."`, 未发射 typed event。

### Required Change

1. 在 `src/event/payload.rs` 的 `What` 枚举中新增变体:
   ```rust
   FairnessProbeStarvation {
       starved_child_id: ChildId,
       skip_count: u64,
       probe_start_unix_nanos: u128,
       probe_end_unix_nanos: u128,
   },
   ```
2. 在 `What::name()` 中添加 `Self::FairnessProbeStarvation { .. } => "FairnessProbeStarvation"`
3. 修改 `check_fairness_probe` 方法: 构造 `PendingRuntimeEvent` 并调用 `self.emit_pending_event()` 替代 `event_sender.send()`
4. 如需要，在 `observe/pipeline.rs` 的 `emit_policy_diagnostic` 中添加对应的诊断处理

### Files to Modify

- `src/event/payload.rs` — 新增 `What::FairnessProbeStarvation` 变体 + `name()` 分支
- `src/runtime/control_loop.rs` — `check_fairness_probe` 改为发射 typed event

### Acceptance Criteria

- [ ] `What` 枚举包含 `FairnessProbeStarvation` 变体
- [ ] `What::name()` 返回 `"FairnessProbeStarvation"`
- [ ] 当 `FairnessProbe::check()` 返回 `Some(StarvationAlert)` 时，控制回路发射 `What::FairnessProbeStarvation` typed event
- [ ] `cargo test` 全量通过
- [ ] `cargo clippy -- -D warnings` 零警告
