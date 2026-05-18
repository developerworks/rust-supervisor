# Sync Apply Report

Applied: 2026-05-18T00:00:00Z
Based on: proposals from 2026-05-18T00:00:00Z (auto-approve)

## Summary

| Category | Count |
|----------|-------|
| Specs Updated | 1 |
| Proposals Applied | 6 |
| Align Tasks Generated | 3 |
| Not Applied | 1 (deferred) |

---

## Changes Made

### Specs Updated

| Spec | Section | Change Type | Proposal |
|------|---------|-------------|----------|
| 006-4-restart-policy-production/spec.md | Key Entities | Modified | #6 (GroupConfig/GroupDependencyEdge/SeverityDefaults) |
| 006-4-restart-policy-production/spec.md | Key Entities | Modified | #7 (ChildSpec.severity/ChildSpec.group) |
| 006-4-restart-policy-production/spec.md | Diagnostics | Modified | #5 (emit_policy_diagnostic) |

### Implementation Tasks Generated

| Task ID | Requirement | Priority | Est. Effort | File |
|---------|-------------|----------|-------------|------|
| ALIGN-001 | SC-000 (策略事件发射) | P0 | medium | `.specify/sync/align-tasks.md` |
| ALIGN-002 | FR-003 (CorrelationId) | P0 | small | `.specify/sync/align-tasks.md` |
| ALIGN-003 | FR-001 (FairnessProbe typed event) | P1 | medium | `.specify/sync/align-tasks.md` |

### Not Applied

| Proposal | Reason |
|----------|--------|
| #4 (SC-003 事件/指标一致率) | 推迟到 006-5-typed-events-observability 切片 |

## Asset Inventory

| Asset | Path |
|-------|------|
| Backup (spec.md) | `.specify/sync/backups/spec.md.2026-05-18` |
| Backfill changes | `specs/006-4-restart-policy-production/spec.md` |
| Align tasks | `.specify/sync/align-tasks.md` |

## Next Steps

1. **Implement ALIGN-001**: 修复 `stage_emit_typed_event` 发射策略事件
2. **Implement ALIGN-002**: 修复 CorrelationId 传递 (1 行改动)
3. **Implement ALIGN-003**: 新增 `What::FairnessProbeStarvation` typed event
4. **Verify**: `cargo test` + `cargo clippy -- -D warnings`
5. **Commit**: `git add specs/ .specify/sync/ && git commit`
