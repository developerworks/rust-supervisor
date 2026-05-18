# Tasks(任务): 平台边界, 说明文档与看板 IPC(进程间通信) 安全强化

**Input(输入)**: 设计文档来自 `specs/006-1-platform-docs-ipc-security/`
**Prerequisites(前置文档)**: plan.md(必需), spec.md(用户故事必需), research.md, data-model.md, contracts/ipc-control-points.md, quickstart.md

**Tests(测试)**: US3(用户故事三) 为行为变化, 测试任务必须先于实现任务. US1 和 US2 为纯文档变更, 使用人工 diff(差异比对) 审阅作为静态验证手段.

**Organization(组织方式)**: 任务按用户故事分组, 每个故事可独立实现和独立验收.

## Format(格式): `[ID] [P?] [Story] Description(描述)`

- **[P]**: 可以并行执行, 因为任务修改不同文件, 并且不依赖未完成任务.
- **[Story]**: 标记任务属于哪个用户故事, 例如 US1, US2, US3.
- 任务描述必须写出准确文件路径.

## Path Conventions(路径约定)

- **Rust single crate(Rust 单包)**: 仓库根目录下的 `src/`, `tests/` 和 `Cargo.toml`.
- 所有测试代码放在外部 `tests/` 目录, 不得写入 `src/` 模块文件.

---

## Phase 1: Setup(项目初始化)

> 创建本切片需要的新目录结构.

- [x] T001 创建目录结构: `src/platform/`, `src/ipc/security/`, 按 plan.md 项目结构执行

---

## Phase 2: Foundational(基础层)

> 所有用户故事都依赖的基础设施. **必须在本阶段完成后才能开始任何用户故事.**

- [x] T002 [P] 为 dashboard(看板) 模块添加 `#[cfg(unix)]` 条件编译守卫: 在 `src/dashboard/mod.rs` 顶部加 `#[cfg(unix)]` 属性, 在 `src/lib.rs` 中为 `pub mod dashboard` 加 `#[cfg(unix)]`
- [x] T003 [P] 创建平台模块 `src/platform/mod.rs`, 写明条件编译声明与 Unix-only 构建确认注释, 按 research.md 第一节执行
- [x] T004 [P] 在 `src/config/ipc_security.rs` 中定义 `IpcSecurityConfig` 及全部 9 个子配置结构体 (`PeerIdentityConfig`, `AuthorizationConfig`, `ReplayProtectionConfig`, `RequestSizeLimitConfig`, `RateLimitConfig`, `AuditConfig`, `IdempotencyConfig`, `AllowlistConfig`), 含 serde(序列化) derive(派生宏) 与默认值函数, 按 data-model.md 执行
- [x] T005 [P] 在 `src/dashboard/error.rs` 中扩展 `DashboardError`, 新增 IPC 安全错误变体: `peer_cred_uid_mismatch`, `peer_cred_gid_not_allowed`, `peer_cred_pid_not_allowed`, `peer_cred_unavailable`, `authz_denied`, `authz_not_configured`, `replay_detected`, `request_too_large`, `rate_limit_exceeded`, `audit_write_failed`, `audit_queue_full`, `ipc_socket_owner_mismatch`, `allowlist_denied`, `allowlist_empty`, 按 contracts/ipc-control-points.md 执行

**Checkpoint(检查点)**: `cargo check` 通过. 平台模块和 IPC 安全配置类型定义完成, 错误枚举扩展完成, dashboard(看板) 模块在 Unix 平台正常编译.

---

## Phase 3: User Story 1(用户故事一) — 购买者开箱前就看清平台边界 (Priority(优先级): P1)

**Goal(目标)**: README 中新增平台支持矩阵, 写明 Unix-like(类 Unix 系统) 和非类 Unix 组合下的编译与裁剪标记.

**Independent Test(独立测试)**: 人工 diff(差异比对) 审阅 README 支持矩阵表格, 对照 spec.md US1 验收场景逐条核对. 买方角色卡片 (集成工程师, 运维负责人, 安全复核员) 各完成一次书面选型复盘.

### Implementation(实现)

- [x] T006 [US1] 在 `README.md` 和 `README.zh.md` 中添加平台支持矩阵: 表格至少含三列 "Host OS family(主机操作系统族别)", "Core supervision(核心监督能力)", "Dashboard IPC(看板进程间通信)", "Notes(裁剪标记)"; 写明 Unix-like(类 Unix 系统) 支持及 `#[cfg(unix)]` 条件编译机制, 写明非 Unix 平台 dashboard(看板) 排除说明, 按 spec.md FR-001 执行
- [x] T007 [US1] 在 `README.md` 和 `README.zh.md` 中添加平台边界说明: 写明 target process(目标进程) 仅监听本地 Unix Domain Socket(Unix 域套接字), 禁止出现 `0.0.0.0/0` 示例行, 列出非 Unix 构建下的裁剪字段清单, 按 spec.md FR-001 验收场景执行

**Checkpoint(检查点)**: 任选一名未读过源码的同事, 仅凭 README 完成一次工件勾选, 结论不得与支持矩阵矛盾.

---

## Phase 4: User Story 2(用户故事二) — 架构三目录拆分一眼可读 (Priority(优先级): P2)

**Goal(目标)**: README 中新增架构小节, 固定标题层级写明 core library(核心库), relay(中继), user interface(用户界面) 的进程边界, 套接字归属与日志字段前缀.

**Independent Test(独立测试)**: 人工 diff(差异比对) 审阅. 参与者仅凭架构小节在白板上画出三条数据流连线. 验收者核对连线是否正确.

### Implementation(实现)

- [x] T008 [US2] 在 `README.md` 和 `README.zh.md` 中添加架构小节: 写明三目录拆分 (core library(核心库), relay(中继), user interface(用户界面)), 附可复制粘贴的目录路径示例, 套接字归属, 协议翻译职责分工与日志字段前缀, 按 spec.md FR-002 和 quickstart.md 第二节执行

**Checkpoint(检查点)**: 参与者不打开源码树, 仅凭架构小节口述三件套各自的 IPC(进程间通信) 路径, 协议翻译职责与渲染进程归属.

---

## Phase 5: User Story 3(用户故事三) — 看板 IPC(进程间通信) 安全控制点可被安全官逐项勾验 (Priority(优先级): P3)

**Goal(目标)**: 实现 9 项 IPC(进程间通信) 控制点 (C1-C9), 每项可独立配置, 拒绝时返回结构化错误与审计条目. 验收测试覆盖所有控制点的放行与拒绝样本.

**Independent Test(独立测试)**: 运行 `cargo test --test ipc_security_integration`. 每项控制点各有放行样本和拒绝样本. 拒绝样本后监督状态不变, 审计记录写入匹配条目.

### Tests(测试) — 必须先于实现

> 按宪法原则 III: 行为变化必须先有测试. 所有测试代码写入外部 `tests/` 目录.

- [x] T009 [US3] 在 `tests/ipc_security_integration.rs` 中编写 IPC(进程间通信) 安全集成测试: 对每项控制点 C1-C9 各构造一组放行样本与一组拒绝样本; 验证拒绝路径返回结构化错误, 携带正确的错误码与 `denial_control_point` 字段; 验证审计记录写入 `allowed: false`; 验证拒绝后监督状态不变, 按 contracts/ipc-control-points.md 和 spec.md SC-003 执行

### Implementation(实现) — C1-C2: Peer Identity(对端身份校验)

- [x] T010 [P] [US3] 在 `src/ipc/security/peer_identity.rs` 和 `src/dashboard/ipc_server.rs` 中实现 C1 socket owner(套接字所有者) 检查: 在 bind(绑定) 前验证套接字文件所有者与当前进程 uid(用户标识) 一致, 拒绝 symlink(符号链接), 集成进现有 `prepare_socket_path()`, 按 contracts C1 执行
- [x] T011 [P] [US3] 在 `src/ipc/security/peer_identity.rs` 中实现 C2 peer credentials(对端身份) 检查: 通过 `std::os::unix::net::UnixStream::peer_cred()` 提取 `PeerIdentity { pid, uid, gid }`, 与 `PeerIdentityConfig` 比对 (uid(用户标识) 匹配, gid(组标识) 白名单, pid(进程标识) 白名单), 不匹配时返回结构化错误, 按 contracts C2 执行

### Implementation(实现) — C3: Command Authorization(命令授权)

- [x] T012 [P] [US3] 在 `src/ipc/security/authz.rs` 中实现 C3 命令授权: 定义 `IpcRiskAction` 枚举与 `classify()` 方法, 将 `IpcMethod` 映射为 `Read`/`WriteChild`/`Destructive`; 实现授权检查: Read(读) 类对任何已认证对端放行, WriteChild 与 Destructive 类要求对端 uid(用户标识) 在 `allowed_uids` 中, 按 contracts C3 执行

### Implementation(实现) — C4: Replay Protection(重放保护)

- [x] T013 [P] [US3] 在 `src/ipc/security/replay.rs` 中实现 C4 重放保护: 滑动窗口 `ReplayWindow` 使用 `HashMap<String, Instant>`, 可配置 `window_size` (默认 1024) 与 `ttl_seconds` (默认 60), 先 `is_replay()` 检查后 `record()` 记录, 定期 `purge_expired()` 清理过期条目, 按 contracts C4 执行

### Implementation(实现) — C5-C6: Limits(流量与尺寸限制)

- [x] T014 [P] [US3] 在 `src/ipc/security/limits.rs` 中实现 C5 请求体大小限制与 C6 速率限制: C5 在 JSON 反序列化前检查原始字节长度是否超过 `max_bytes`; C6 实现 `TokenBucket`(令牌桶) 算法, 每连接独立, `refill_rate`(补充速率) 默认 100.0/秒, `burst_capacity`(突发容量) 默认 20; 超限均返回结构化错误, 按 contracts C5-C6 与 research.md 第三节执行

### Implementation(实现) — C7: Audit Persistence(审计持久化)

- [x] T015 [P] [US3] 在 `src/ipc/security/audit.rs` 中实现 C7 审计持久化: 定义 `AuditRecord` 结构体, 含 timestamp(时间戳), method(方法), initiator_hash(发起人哈希), correlation_id(关联标识), allowed(是否放行), denial_code(拒绝码), denial_control_point(拒绝控制点); memory(内存) 后端 (ring buffer(环形缓冲) 4096 条) 与 file(文件) 后端 (追加 JSON Lines(JSON 行)); `fail_closed`(失败即闭) 策略 (审计写失败时拒绝写命令) 与 `defer_bounded`(有界延迟) 策略 (有界队列最大 1000 条); 后端离线时在 tracing(结构化追踪) 上暴露告警计数, 按 contracts C7 与 spec.md SC-004 执行

### Implementation(实现) — C8: Command Idempotency(命令幂等)

- [x] T016 [P] [US3] 在 `src/ipc/security/idempotency.rs` 中实现 C8 命令幂等: `IdempotencyCache` 映射 `request_id` 到已缓存 `IpcResponse`, 带 TTL(存活时间); 缓存命中时直接返回缓存结果, 不重新执行; 缓存未命中时执行命令并缓存结果; `result_cache_ttl_seconds`(结果缓存存活秒数) 默认 60, `max_cached_results`(最大缓存条目) 默认 1024, 按 contracts C8 执行

### Implementation(实现) — C9: External Command Allowlist(外部命令白名单)

- [x] T017 [P] [US3] 在 `src/ipc/security/allowlist.rs` 中实现 C9 外部命令白名单: 检查可执行文件绝对路径是否在 `allowed_paths` 中; `allowed_paths` 为空则拒绝所有; 拒绝时返回结构化错误 `allowlist_denied` 或 `allowlist_empty`, 按 contracts C9 执行

### Implementation(实现) — Module Assembly(模块装配)

- [x] T018 [US3] 创建 IPC(进程间通信) 安全模块入口 `src/ipc/security/mod.rs`: 定义 `IpcSecurityPipeline` 结构体, 持有由 `IpcSecurityConfig` 加载的 9 个控制点实例; 实现 `process_request()` 方法, 按 C6 → C5 → C2 → C4 → C3 → C9 → C8 → dispatch(分发) → C7 顺序执行控制点, 按 contracts 执行顺序图; 按控制点暴露 tracing(结构化追踪) target(目标)
- [x] T019 [US3] 将 `IpcSecurityPipeline` 接入 `DashboardIpcService`: 在 `src/dashboard/ipc_server.rs` 的 `handle_request()` 与 `bind_dashboard_listener()` 中集成管线; 确保 C1 (socket owner(套接字所有者)) 在 bind(绑定) 时执行, C2-C9 在每次请求时执行; 保持与现有协议的向下兼容

**Checkpoint(检查点)**: `cargo test --test ipc_security_integration` 全部通过. 9 项控制点各有放行和拒绝样本. 拒绝路径返回结构化错误. 审计记录正确.

---

## Phase 6: Polish(收尾)

> 跨切片的收尾与验证.

- [x] T020 更新 `AGENTS.md` 中的 SPECKIT 上下文: 确认功能路径指向 `specs/006-1-platform-docs-ipc-security/plan.md`, 在文档列表中补充 `tasks.md`
- [x] T021 运行完整测试套件: `cargo test` (全部已有测试 + 新增 `ipc_security_integration`), `cargo clippy -- -D warnings`, `cargo fmt --check`

---

## Dependencies(依赖关系)

```
Phase 1 (Setup 项目初始化)
  │
  ▼
Phase 2 (Foundational 基础层)  ←── 所有用户故事的前置
  │
  ├──▶ Phase 3 (US1: P1 平台边界文档)  ← 可并行
  ├──▶ Phase 4 (US2: P2 架构文档)      ← 可并行
  │
  └──▶ Phase 5 (US3: P3 IPC(进程间通信) 控制点)
           │
           T009 (Tests 测试)
           │
           ├──▶ T010 [P]  C1-C2 peer_identity(对端身份)
           ├──▶ T011 [P]  C1-C2 peer_identity(对端身份)
           ├──▶ T012 [P]  C3 authz(授权)
           ├──▶ T013 [P]  C4 replay(重放)
           ├──▶ T014 [P]  C5-C6 limits(限制)
           ├──▶ T015 [P]  C7 audit(审计)
           ├──▶ T016 [P]  C8 idempotency(幂等)
           ├──▶ T017 [P]  C9 allowlist(白名单)
           │
           └──▶ T018 mod.rs(模块入口, 依赖 T010-T017)
                    │
                    └──▶ T019 接入 ipc_server.rs
  │
  ▼
Phase 6 (Polish 收尾)  ← 所有用户故事完成后
```

## Parallel Execution(并行执行示例)

### Phase 2 内并行

```bash
# 四个基础任务可同时执行:
Task T002: "为 dashboard(看板) 模块添加 #[cfg(unix)] 守卫"
Task T003: "创建平台模块 src/platform/mod.rs"
Task T004: "定义 IPC(进程间通信) 安全配置结构体 src/config/ipc_security.rs"
Task T005: "扩展 DashboardError src/dashboard/error.rs"
```

### US1, US2, US3-Tests 并行

```bash
# Phase 2 完成后, 三个方向可同时推进:
Task T006-T007: "US1 README 平台文档"
Task T008:       "US2 README 架构文档"
Task T009:       "US3 IPC(进程间通信) 安全测试"
```

### US3 实现阶段并行

```bash
# T009 (测试) 完成后, 7 个模块实现可同时执行:
Task T010: "src/ipc/security/peer_identity.rs"
Task T012: "src/ipc/security/authz.rs"
Task T013: "src/ipc/security/replay.rs"
Task T014: "src/ipc/security/limits.rs"
Task T015: "src/ipc/security/audit.rs"
Task T016: "src/ipc/security/idempotency.rs"
Task T017: "src/ipc/security/allowlist.rs"
```

## Implementation Strategy(实现策略)

### MVP(最小可行产品): 仅 User Story 1(用户故事一)

1. 完成 Phase 1 (Setup(项目初始化)) + Phase 2 (Foundational(基础层))
2. 完成 Phase 3 (US1): README 平台支持矩阵
3. 交付: 集成方能根据 README 做出平台选型决策

### Incremental Delivery(增量交付)

1. **Iteration 1(迭代一)**: Phase 1 + Phase 2 (基础层, 对所有故事可见)
2. **Iteration 2(迭代二)**: Phase 3 (US1: 平台边界文档) → 买方能看懂支持矩阵
3. **Iteration 3(迭代三)**: Phase 4 (US2: 架构文档) → 运维能看懂部署拓扑
4. **Iteration 4(迭代四)**: Phase 5 (US3: IPC(进程间通信) 控制点实现 + 测试) → 安全官能逐项勾验
5. **Iteration 5(迭代五)**: Phase 6 (收尾验证)

## Summary(摘要)

| Metric(指标)                      | Value(值)                                                  |
| --------------------------------- | ---------------------------------------------------------- |
| Total tasks(任务总数)             | 21                                                         |
| US1 tasks(用户故事一)             | 2 (T006-T007)                                              |
| US2 tasks(用户故事二)             | 1 (T008)                                                   |
| US3 tasks(用户故事三)             | 11 (T009-T019)                                             |
| Setup + Foundational(初始化+基础) | 5 (T001-T005)                                              |
| Polish(收尾)                      | 2 (T020-T021)                                              |
| Parallel opportunities(可并行)    | T002-T005, T010-T017                                       |
| Independent test(独立测试)        | US1: diff(差异比对) 审阅, US2: 白板连线, US3: `cargo test` |
| Suggested MVP(最小可行产品)       | Phase 1-3 (仅 US1, 8 任务)                                 |

1. 完成 Phase 1 (Setup) + Phase 2 (Foundational)
2. 完成 Phase 3 (US1): README 平台支持矩阵
3. 交付: 集成方能根据 README 做出平台选型决策

### Incremental Delivery(增量交付)

1. **Iteration 1(迭代一)**: Phase 1 + Phase 2 (基础层, 对所有故事可见)
2. **Iteration 2(迭代二)**: Phase 3 (US1: 平台边界文档) → 买方能看懂支持矩阵
3. **Iteration 3(迭代三)**: Phase 4 (US2: 架构文档) → 运维能看懂部署拓扑
4. **Iteration 4(迭代四)**: Phase 5 (US3: IPC 控制点实现 + 测试) → 安全官能逐项勾验
5. **Iteration 5(迭代五)**: Phase 6 (收尾验证)

## Summary(摘要)

| Metric(指标)                      | Value(值)                                        |
| --------------------------------- | ------------------------------------------------ |
| Total tasks(任务总数)             | 21                                               |
| US1 tasks(用户故事一)             | 2 (T006-T007)                                    |
| US2 tasks(用户故事二)             | 1 (T008)                                         |
| US3 tasks(用户故事三)             | 11 (T009-T019)                                   |
| Setup + Foundational(初始化+基础) | 5 (T001-T005)                                    |
| Polish(收尾)                      | 2 (T020-T021)                                    |
| Parallel opportunities(可并行)    | T002-T005, T010-T017                             |
| Independent test(独立测试)        | US1: diff 审阅, US2: 白板连线, US3: `cargo test` |
| Suggested MVP(最小可行产品)       | Phase 1-3 (US1 only, 8 tasks)                    |
