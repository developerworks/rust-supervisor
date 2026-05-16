# Drift Resolution Proposals

Generated: 2026-05-15T21:16:00Z
Based on: `.specify/sync/drift-report.json` (generated `2026-05-15T21:10:00Z`)

## Summary

| Resolution Type | Count |
|-----------------|-------|
| Backfill (open) | 0 |
| Backfill (applied) | 8 (**P1**, **P3** option **3a**, **P4**, **P5**, **P6**, **P7**, **P8**, **P9**) |
| Align (applied) | 1 (**P2**, closed) |
| Human Decision (open) | 0 |
| Closed (applied, cum.) | 9 (**P1**–**P9**) |

## Interactive cues

对每个 **Proposal**, 在 MR 评审或本条对话里用一个字母批复: **`A`** 采纳, **`R`** 否决, **`M`** 采纳但需改写 (跟一句说明), **`S`** 跳过本轮, **`Q`** 停止逐项批复 (你可以改在文件里勾选).

**当前**: 本条队列已全部 **APPLIED** (**P1**–**P9**); 有重大规格或实现变更时请先跑 **`speckit.sync.analyze`**.

---

### Proposal P1 (closed): `001-create-supervisor-core` / **FR-063**

**Direction**: **BACKFILL** (Code → Spec)
**resolution_status**: **APPLIED**
**applied_path**: `specs/001-create-supervisor-core/spec.md`

**Summary**: **FR-063** 已改写为以 **`ChildRuntimeRecord`** 与 **`ChildControlResult`** 为主, **`current_state`** 为查询命令, **`ChildState`** 仅作历史或受管展示指称, 与 **004-3** 一致. 当前漂移表里 **FR-063** 不再列为 drift.

---

### Proposal P2 (closed): `001-create-supervisor-core` / **SC-010**

**Direction**: **ALIGN** (Spec → Code), option **2b**
**resolution_status**: **APPLIED**
**applied_note**: `src/tests/*_test.rs` 使用 **`#[tokio::test(start_paused = true)]`** 与 **`rust_supervisor::test_support::test_time`**; **`specs/001-create-supervisor-core/spec.md`** 中 **SC-010** 已写明暂停 Tokio 与 **virtual time** 推进方式.

**Residual (非阻塞)**: 部分用例仍在测试代码里直接 **`tokio::time::sleep`**; 在 **paused** 运行时上其等待的是 **mock timer**, 不是主机墙钟. **`tests/dashboard_runtime_startup_test.rs`** 仍建议按 **SC-010** 范围做一次对照审查.

---

### Proposal P3 (closed): `001-create-supervisor-core` / **FR-072, FR-073, FR-074**

**Direction**: **BACKFILL** (Code → Spec), option **3a**
**resolution_status**: **APPLIED**
**applied_path**: `specs/001-create-supervisor-core/spec.md`

**Resolution**: 三条 **FR** 各增加 **Governance scope(治理范围)** 段: 标明 **process-level(过程级)** 验收绑定 **`specs/*/tasks.md`**, **`.specify/`**, **speckit** 与 **Pull Request(合并请求)** 流程, **不**要求 **`rust-tokio-supervisor`** crate 用单一 **`cargo test`** 证明台账或卡点记录落库源码.

**Confidence**: **MEDIUM**

**Action**:
- [x] Approve 3a
- [ ] Approve 3b
- [ ] Reject
- [ ] Modify

---

### Proposal P4 (closed): `002-config-schema-support` / **metadata Status**

**Direction**: **BACKFILL**
**resolution_status**: **APPLIED**
**applied_path**: `specs/002-config-schema-support/spec.md`

**applied_note**: 已将 **`specs/002-config-schema-support/spec.md`** 头部 **`Status`** 更新为 **Accepted**; 交互批复 **A** (2026-05-17).

**Confidence**: **HIGH**

**Action**:
- [x] Approve
- [ ] Reject
- [ ] Modify

---

### Proposal P5 (closed): `003-supervisor-dashboard` / **sibling scope**

**Direction**: **BACKFILL**
**resolution_status**: **APPLIED**
**applied_path**: `specs/003-supervisor-dashboard/spec.md`

**applied_note**: 已在 **`specs/003-supervisor-dashboard/spec.md`** 增补 **Repository scope(仓库责任范围)** 表格: 能力与三仓 **ownership(主责)** 用 ✔ / — 勾选; **单仓检出** 时 **relay/UI** 不在 **`src/`** 属规格明示的拆分交付边界.

**Confidence**: **HIGH**

**Action**:
- [x] Approve
- [ ] Reject
- [ ] Modify

---

### Proposal P6 (closed): `004-1-runtime-lifecycle-guard` & `004-3-child-runtime-state-control` / **metadata Status**

**Direction**: **BACKFILL**
**resolution_status**: **APPLIED**
**applied_path**: `specs/004-1-runtime-lifecycle-guard/spec.md`, **`specs/004-3-child-runtime-state-control/spec.md`**

**applied_note**: 两份 **`spec.md`** 已将 **`Status(状态)`** 从 **`Draft(草稿)`** 升为 **`Accepted(已接受)`**, 并补齐 **`Updated(更新日期)`** **2026-05-15** (**004-1** 新增 **`Updated`** 字段). **`tasks.md`** 增加已勾选 **T037** / **T051** 记录本决议与用户 **批准** (**2026-05-15**).

**Confidence**: **HIGH**

**Action**:
- [x] Approve
- [ ] Reject
- [ ] Modify

---

### Proposal P7 (closed): `004-4-generation-fencing` / **backoff + `DelayedSpawnAttached`**

**Direction**: **BACKFILL**
**resolution_status**: **APPLIED**
**applied_path**: `specs/004-4-generation-fencing/spec.md`, `specs/004-4-generation-fencing/contracts/generation-fencing.md`, `specs/004-4-generation-fencing/plan.md`

**applied_note**: 已增加 **`FR-004`**, **`SC-005`**, **`Key Entities`** **`DelayedSpawnAttached`** 说明与 **Edge Cases** 一条; **`contracts/generation-fencing.md`** **Runtime Semantics** 专节新增 **DelayedSpawnAttached 与正 backoff**; **`plan.md`** **Technical Context** 增补 **Delayed spawn mailbox** 段. **tasks.md** **T033** 记录本决议.

**Confidence**: **HIGH**

**Action**:
- [x] Approve
- [ ] Reject
- [ ] Modify

---

### Proposal P8 (closed): `004-4-generation-fencing` / **`generation_fencing_replay_child_exit_for_test`**

**Direction**: **BACKFILL**
**resolution_status**: **APPLIED**
**applied_path**: `specs/004-4-generation-fencing/plan.md`

**applied_note**: **`plan.md`** **`Testing`(测试)** 下增补 **`Stale report test replay`(过期报告测试回放)** 段落, 写明 **`#[doc(hidden)]`**, **`ReplayChildExitForTest`**, **`mailbox`** 语义与 **`supervisor_generation_fencing_test`** 覆盖面; **`Complexity Tracking`** 指回该段; **`tasks.md`** **`T034`**. 交互批复 **A**, **日期** **2026-05-15**.

**Confidence**: **HIGH**

**Action**:
- [x] Approve
- [ ] Reject
- [ ] Modify

---

### Proposal P9 (closed): `001-create-supervisor-core` / glossary **ChildState** vs **004-3** record types

**Direction**: **BACKFILL**

**resolution_status**: **APPLIED**

**applied_path**: `specs/001-create-supervisor-core/glossary.md`, **`tasks.md`** **`T144`**

**applied_note**: **Policy And State(策略与状态)** 与 **Rust Types** 表补齐 **`ChildRuntimeRecord`**, **`ChildRuntimeState`**, **`ManagedChildState`**, **`ChildControlResult`**, 并收紧 **`ChildState`** 为历史叙述轴, 指针 **`specs/004-3-child-runtime-state-control`**; **`drift-report`** 中 **Inter-spec** 冲突条目已清空 (**2026-05-15**, **下一步** 批复链路).

**Confidence**: **MEDIUM**

**Action**:
- [x] Approve
- [ ] Reject
- [ ] Modify

---

## Interactive session

**队列已空**: **Proposal P9** **`APPLIED`** (**glossary.md** **`ChildState`** ↔ **`004-3`** 轴对齐). **`speckit.sync.apply`** 历史中 **P3** 正文已并入规格,其余为规格与台账更新.

**(历史)** **`Cursor`** 曾提示下一条交互为 **P9**; 现已结案.
