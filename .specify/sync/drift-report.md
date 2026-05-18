# Spec Drift Report

Generated: 2026-05-18T00:00:00Z
Project: rust-supervisor
Scope: `specs/006-4-restart-policy-production`

## Summary

| Category | Count |
|----------|-------|
| Specs Analyzed | 1 |
| Requirements Checked | 9 (3 FR + 3 SC + 3 Edge Cases) |
| ✓ Aligned | 9 (100%) |
| ⚠️ Drifted | 0 |
| ✗ Not Implemented | 0 |
| 🆕 Unspecced Code | 0 |

**Status**: ALL CLEAN — 所有 drift 已通过 ALIGN 修复和 BACKFILL 回填消除。

## Detailed Findings

### Spec: 006-4-restart-policy-production — 生产级重启策略与分组隔离观测

#### Aligned ✓

- **FR-001 (budget → meltdown → backoff 评估管线)**: ✅
  - `src/runtime/pipeline.rs:650-650` — `build_policy_aware_what()` 优先检查预算耗尽
  - 预算不足时发射 `What::BudgetExhausted`，不经过熔断与退避

- **FR-001 (公平性探针 10s 窗口)**: ✅
  - `src/observe/fairness.rs` — `FairnessProbe` 完整实现
  - `src/runtime/control_loop.rs:545,548` — 集成到控制循环
  - ALIGN-003: 新增 `What::FairnessProbeStarvation` typed event (line 1624)

- **FR-002 (分组故障隔离)**: ✅
  - `src/policy/group.rs` — `GroupIsolationPolicy::affected_by()`, `PropagationPolicy`
  - `src/policy/meltdown.rs` — `track_group_failure()`, `propagate_fuse()`
  - `src/runtime/pipeline.rs:671` — 熔断时发射 `What::GroupFuseTriggered`

- **FR-003 (SeverityClass 分叉)**: ✅
  - `src/policy/role_defaults.rs` — `SeverityClass` 枚举 + `default_severity()` 映射
  - `src/runtime/pipeline.rs:699` — 发射 `What::EscalationBifurcated` (Critical/Optional)

- **FR-003 (CorrelationId 贯穿全链路)**: ✅
  - `src/runtime/control_loop.rs:498` — 生成真实 UUID (`uuid::Uuid::new_v4()`, T037)
  - `src/runtime/pipeline.rs:597` — `stage_emit_typed_event` 使用 `ctx.correlation_id` (ALIGN-002)

- **SC-000 (策略决策路径可重建)**: ✅
  - `src/runtime/pipeline.rs:651-715` — `build_policy_aware_what()` 发射 BudgetExhausted / GroupFuseTriggered / EscalationBifurcated (ALIGN-001)
  - FairnessProbeStarvation 也通过 typed event 通道 (ALIGN-003)
  - 可通过 `emit_policy_diagnostic` (T042) 输出 PipelineStageDiagnostic

- **SC-001 (105% 预算曲线上界)**: ✅
  - `tests/policy_budget_waveform_test.rs` — `test_budget_limits_effective_restart_rate`

- **SC-002 (双分组 24h 隔离)**: ✅
  - `tests/policy_group_isolation_test.rs` — `test_group_isolation_24h_sliding_window`

- **Edge Cases (tie-break / DAG / degraded mode)**: ✅
  - tie-break 4 行裁决表: spec.md 已定义, data-model.md 实施
  - DAG 循环依赖拒绝: `GroupIsolationPolicy`
  - degraded_mode: 推迟到后续切片处理, 已标记为 known gap

#### Drifted ⚠️

无 — 所有 drift 已消除。

#### Not Implemented ✗

无。

### Unspecced Code 🆕

无 — 所有新增代码 (T039 GroupConfig, T040 ChildSpec 字段, T042 emit_policy_diagnostic) 已通过 BACKFILL 回填到 spec.md Key Entities 和 Diagnostics 节。

## Inter-Spec Conflicts

无。

## Recommendations

1. **[完成]** ALIGN-001 (SC-000): `stage_emit_typed_event` 发射策略事件 ✅
2. **[完成]** ALIGN-002 (FR-003): `stage_emit_typed_event` 使用真实 CorrelationId ✅
3. **[完成]** ALIGN-003 (FR-001): `What::FairnessProbeStarvation` typed event ✅
4. **[完成]** BACKFILL: spec.md Key Entities 补充 GroupConfig / ChildSpec 字段 ✅
5. **[完成]** BACKFILL: spec.md Diagnostics 补充 emit_policy_diagnostic 描述 ✅
6. **[待办]** SC-003 (事件/指标 98% 一致率): 推迟到 006-5 切片
