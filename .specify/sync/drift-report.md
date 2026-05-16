# Spec Drift Report

Generated: 2026-05-15T21:10:00Z
Project: rust-tokio-supervisor

## Summary

| Category | Count |
|----------|-------|
| Specs Analyzed | 7 |
| Requirements Checked (FR and SC line items in `specs/*/spec.md`) | 215 |
| Aligned (estimated) | 213 (99%) |
| Drifted (estimated) | 2 (1%) |
| Not Implemented (estimated) | 0 (0%) |
| Unspecced Code (notable) | 0 |

**Method note**: Counts treat each `**FR-*` and `**SC-*` bullet in `specs/*/spec.md` as one line item. Alignment follows spot checks, repository grep, and module ownership, not automated proof for every bullet. Use this report as a prioritized review queue.

## Detailed Findings

### Spec: 001-create-supervisor-core - Feature Specification: create supervisor core

#### Aligned (representative)

- **FR-023**, **FR-028**–**FR-032**, **FR-036**, **FR-064** unchanged from prior review.
- **FR-063**: Spec text now centers `ChildRuntimeRecord`, `ChildControlResult`, `current_state`, and explicit `ChildState` legacy wording; matches code and 004-era naming. Treat prior FR-063 naming drift as **resolved at spec level** (see proposals **P1** `APPLIED`).
- **Glossary alignment**: **`specs/001-create-supervisor-core/glossary.md`** **Policy And State** 与 **Rust Types** 表把 **`ChildState`** **历史轴**写到与 **`ChildRuntimeRecord`**, **`ManagedChildState`**, **`ChildControlResult`**, **`ChildRuntimeState`** 同一读本,指针 **`004-3`** (**Proposal P9** **`APPLIED`**).
- **SC-031** and related naming rules: continue to align with `coding_standard_test` and glossary direction.

#### Drifted

- **SC-010**: Integration tests under `src/tests/` consistently use `#[tokio::test(start_paused = true)]` and `rust_supervisor::test_support::test_time` (`advance_test_clock`, `with_auto_clock_drive`), matching the SC-010 narrative in `specs/001-create-supervisor-core/spec.md`. **Do not** treat those files as blanket “real wall-clock sleep” violations. Remaining gap is **minor**: some tests still write `tokio::time::sleep` in the test body or in spawned tasks; on a **paused** runtime these sleeps use the **mock timer wheel**, not the host wall clock. Crate-level `tests/dashboard_runtime_startup_test.rs` uses `sleep` outside the documented paused-test pattern; flag for review against SC-010 scope (backoff, timeout, heartbeat, meltdown versus IPC startup).

#### Governance-aligned (process-level FR, Proposal P3 option 3a)

- **FR-072**, **FR-073**, **FR-074**: **`specs/001-create-supervisor-core/spec.md`** 已标注为 **process-level**, 验收绑定 **`tasks.md`**, **`.specify/`**, **speckit** 与 **Pull Request** 流程; 不再将「无 crate 内自动门卫」列为 **not_implemented** 漂移.

---

#### Not Implemented

*(本 spec 条下无条目; 跨仓能力见 **003**.)*

### Spec: 002-config-schema-support - Config struct + JsonSchema + confique

#### Aligned

- confique config, JsonSchema, template generation and module tests.
- **Metadata**: **`spec.md` Status Accepted** matches implementation maturity and sync proposal **P4** `APPLIED`.

#### Resolved (prior drift)

- **Metadata** was **Draft** versus CI coverage; elevated to **Accepted** in **`specs/002-config-schema-support/spec.md`**.

---

### Spec: 003-supervisor-dashboard - Dashboard / IPC / relay / UI

#### Aligned (this crate)

- Target-side IPC, Unix socket plumbing, protocol shapes; `src/dashboard/**`, `tests/dashboard_*`.
- **`specs/003-supervisor-dashboard/spec.md`** 已含 **Repository scope(仓库责任范围)** 表格, 勾选 **relay(中继)** 与 **UI(用户界面)** 在 **`rust-supervisor-relay`** / **`rust-supervisor-ui`** 姊妹仓主责交付; **只检出(current repository only)** 本仓不包含上述栈不构成规格与实现的「意外缺失」(见漂移决议 **Proposal P5** `APPLIED`).

#### Not Implemented

*(本条目原 **「姊妹仓缺失」** 已上升为明示范围; **单仓工作区(workspace)** 若未克隆 relay/UI 仍可视为能力未在本 checkout 中出现, 但语义上属于拆分交付而非未写规格.)*

---

### Spec: 004-1-runtime-lifecycle-guard - Runtime control loop lifecycle guard

#### Aligned

- Control loop messaging, shutdown joins, typed outcomes; `src/runtime/control_loop.rs`, related tests.
- **Metadata**: **`spec.md` Status Accepted** aligns control-loop implementation with lifecycle-guard narrative (**Proposal P6** `APPLIED`).

#### Resolved (prior drift)

- **Draft** banner versus shipped behavior cleared by **`Accepted`** (**2026-05-15**).

---

### Spec: 004-2-real-shutdown-pipeline - Real shutdown pipeline

#### Aligned

- Shutdown coordinator, pipeline, events; `src/shutdown/**`, `supervisor_real_shutdown_pipeline_test`.

---

### Spec: 004-3-child-runtime-state-control - Child runtime state control

#### Aligned

- `ChildRuntimeState`, cancellation, `CurrentState` projection; `supervisor_child_runtime_state_control_test`.
- **Metadata**: **`spec.md` Status Accepted** aligns published types with spec authority (**Proposal P6** `APPLIED`).

#### Resolved (prior drift)

- **Draft** banner versus implemented types cleared by **`Accepted`** (**2026-05-15**).

---

### Spec: 004-4-generation-fencing - Generation fencing for restart

#### Aligned

- **FR-001** / **SC-002**: **`RestartChild`** ordering and fencing phases; **FR-002** / **SC-001**: single active attempt; **FR-003** / **SC-003**: stale exit path and replay hook tests; **FR-004** / **SC-005**: positive **`backoff`** requires **`DelayedSpawnAttached`** mailbox so **`activate_instance`** stays on **`control loop`**; **`plan.md` `Stale report test replay`** anchors **`generation_fencing_replay_child_exit_for_test`** (**`#[doc(hidden)]`**) for **`SC-003`** (**Proposal P8** `APPLIED`).

#### Drifted

- **Metadata**: **Draft** while implementation and tests exist.

#### Resolved (prior drift under proposals P7 and P8)

- **Backoff activation**: **`spec.md` `FR-004`**, **`SC-005`**, **`contracts/generation-fencing.md`** Runtime Semantics subsection, **`plan.md` Delayed spawn mailbox** note now anchor **`DelayedSpawnAttached`** semantics.
- **Stale exit replay hook**: **`plan.md` Testing** **`Stale report test replay`** paragraphs document **`generation_fencing_replay_child_exit_for_test`**, routed via **`ReplayChildExitForTest`**, aligning **`spec.md` `FR-003` / `SC-003`** validation in **`supervisor_generation_fencing_test`**.

---

## Unspecced Code

*(当前无 **Unspecced Code** 表行; 回放钩子已写入 **`plan.md` `Stale report test replay`**.)*

## Inter-Spec Conflicts

*(暂无: **Proposal P9** **`APPLIED`**, **`001` glossary** 已写明 **`ChildState`** 历史轴 versus **`004-3`** **`ChildRuntimeRecord`** 主轴.)*

## Recommendations

1. Keep **004-4** `spec.md`, `plan.md`, and `tasks.md` in sync after merges; promote **Status** when review completes.
2. **DelayedSpawnAttached** + **`control loop` `activate_instance`** non-zero **`backoff`** text is pinned in **`FR-004`**, **`contracts/generation-fencing.md`**, and **`plan.md`** (**Proposal P7** `APPLIED`).
3. **003** Repository scope **Repository scope(仓库责任范围)** 表格已写入规格; **P5** **APPLIED** 后, **relay(中继)** 与 **UI(用户界面)** 只在未克隆姊妹仓的 **checkout(工作区检出)** 中缺席属于明示拆分交付预期.
4. **Hidden test replay** pathway is spelled in **`plan.md`** under **Stale report test replay** (**Proposal P8** `APPLIED`); **`README`** should link there instead of duplicating ad-hoc wording.
5. **001** glossary **Policy And State** / **Rust Types** rows (**Proposal P9** **`APPLIED`**) anchor cross-reader terminology for **`ChildState`** versus **`004-3`** record types without inventing parallel public enums.
