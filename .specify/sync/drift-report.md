# Spec Drift Report

Generated: 2026-05-18T00:00:00Z
Project: rust-supervisor (rust-tokio-supervisor)
Scope: `specs/006-1-platform-docs-ipc-security` (active feature per `.specify/feature.json`)

## Summary

| Category                 | Count                     |
| ------------------------ | ------------------------- |
| Specs Analyzed (active)  | 1                         |
| Requirements Checked     | 3 (FR-001 through FR-003) |
| Success Criteria Checked | 4 (SC-001 through SC-004) |
| ✓ Aligned                | 7 (100%)                  |
| ⚠ Drifted                | 0 (0%)                    |
| ✗ Not Implemented        | 0 (0%)                    |
| 🆕 Unspecced Code        | 0                         |

## Validation

| Command                                      | Result                                         |
| -------------------------------------------- | ---------------------------------------------- |
| `cargo test --test ipc_security_integration` | 21 passed                                      |
| `cargo check`                                | 0 errors                                       |
| `cargo test --test coding_standard_test`     | 7/8 passed (1 minor doc issue, non-functional) |

## Detailed Findings

### Spec: 006-1-platform-docs-ipc-security — Platform Boundary, Docs & Dashboard IPC Security

#### Aligned ✓

- **FR-001 — Support Matrix**: `README.md:45-51`. Table with Host OS family, Core supervision, Dashboard IPC, Notes columns. Unix-like marked Supported, non-Unix marked Not available with crop field list (`dashboard`, `ipc_server`, `registration`). `#[cfg(unix)]` mechanism documented.

- **FR-002 — Architecture Section**: `README.md:73-102`. Three-directory split (core library, relay, user interface) with copyable path examples (`/run/rust-supervisor/payments-worker-a.sock`), socket ownership conventions, log field prefixes per component (`rust_supervisor::dashboard`, `rust_supervisor_relay`, `rust_supervisor_ui`).

- **FR-003 — Nine IPC Control Points (C1-C9)**:
  | CP | Implementation | File |
  |----|--------------|------|
  | C1 | Socket owner check via `prepare_socket_path_for_bind()` | `src/ipc/security/peer_identity.rs:190` |
  | C2 | Peer credentials via `verify_peer_identity()` + `extract_peer_identity()` | `src/ipc/security/peer_identity.rs:144` |
  | C3 | Command authorization via `verify_authorization()` + `IpcRiskAction` | `src/ipc/security/authz.rs:59` |
  | C4 | Replay protection via `ReplayWindow::check_and_record()` | `src/ipc/security/replay.rs:13` |
  | C5 | Request size limit via `check_request_size()` | `src/ipc/security/limits.rs:26` |
  | C6 | Rate limit via `TokenBucket::try_consume()` | `src/ipc/security/limits.rs:51` |
  | C7 | Audit persistence via `AuditRecord` + `AuditBackend` + `alerts` module | `src/ipc/security/audit.rs` |
  | C8 | Command idempotency via `IdempotencyCache::get()/put()` | `src/ipc/security/idempotency.rs:22` |
  | C9 | External command allowlist via `check_allowlist()` | `src/ipc/security/allowlist.rs:13` |
  - All 9 wired through `IpcSecurityPipeline` (`src/ipc/security/mod.rs`), integrated into `DashboardIpcService` (`src/dashboard/ipc_server.rs:115-130`).
  - 14 IPC security error variants in `DashboardError` (`src/dashboard/error.rs:137-263`).
  - Config model: `IpcSecurityConfig` + 9 sub-configs (`src/config/ipc_security.rs`).
  - `#[cfg(unix)]` gating: `src/dashboard/mod.rs` (10 submodules), `src/lib.rs` (`dashboard`, `ipc`).

#### Success Criteria ✓

- **SC-001**: Support matrix present in README.md, 5 OS families, Boolean columns. Usable for blind artifact selection within 30 minutes.
- **SC-002**: Architecture section present with three-component table, directory mounts, socket ownership, log prefixes. Usable for whiteboard diagramming.
- **SC-003**: 21 integration tests covering all C1-C9 with allow/deny pairs. State unchanged after denial. Test file: `tests/ipc_security_integration.rs`.
- **SC-004**: `alerts` module with atomic failure counter (`src/ipc/security/audit.rs:160-168`). Counter increments on audit write failure.

#### Drifted ⚠

None.

#### Not Implemented ✗

None.

### Unspecced Code 🆕

None. All newly created files are fully covered by 006-1 FR-001 through FR-003.

### Inter-Spec Conflicts

None. 006-1 explicitly inherits from 003-supervisor-dashboard (path constraints, symlink rejection) without redefining contracts. Orthogonal to 005-1-failure-policy-reliability (different scope: request security vs policy pipeline).

## Other Specs Spot-Check

Legacy baseline slices verified by module existence:

| Spec ID                           | Status     | Check                                             |
| --------------------------------- | ---------- | ------------------------------------------------- |
| 001-create-supervisor-core        | Historical | Core modules under `src/`                         |
| 002-config-schema-support         | Historical | `src/config/loader.rs`                            |
| 003-supervisor-dashboard          | Historical | `src/dashboard/`                                  |
| 004-1-runtime-lifecycle-guard     | Historical | Lifecycle guard in runtime                        |
| 004-2-real-shutdown-pipeline      | Historical | `src/runtime/shutdown_pipeline.rs`                |
| 004-3-child-runtime-state-control | Historical | `src/runtime/child_runtime_state.rs`              |
| 004-4-generation-fencing          | Historical | `src/tests/supervisor_generation_fencing_test.rs` |
| 005-1-failure-policy-reliability  | Historical | Previous drift report confirmed 100% aligned      |
| 005-2-work-role-defaults          | Historical | Previous drift report confirmed 100% aligned      |
| 006-2 through 006-8               | Stub       | `plan.md`/`spec.md` only, no implementation yet   |

## Recommendations

1. **Close 006-1**: All 3 FRs aligned, all 4 SCs met, 0 drift. Mark spec as complete.
2. **Fix coding standard**: 1 remaining doc comment missing in `src/ipc/security/peer_identity.rs:90` (macOS variant function). Non-blocking.
3. **Prioritize 006-2 through 006-8**: Those 7 specs exist as stubs. Assess which has the largest implementation gap and address next.
