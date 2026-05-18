# Platform & IPC Security Requirements Quality Checklist(平台边界与看板 IPC 安全需求质量检查清单)

**Purpose(目的)**: 验证 `006-1-platform-docs-ipc-security` 功能规格中平台支持矩阵、三目录架构文档化和 9 项 IPC 安全控制点的需求质量、完整性与可度量性。

**Created(创建日期)**: 2026-05-19
**Scope(范围)**: US1(平台边界支持矩阵) + US2(三目录架构文档) + US3(IPC 安全 9 控制点), 全部 3 个用户故事
**Depth(深度)**: Strict(严格 release gate)
**Audience(受众)**: Reviewer(PR 审查) + Security(安全复核)
**Gates(关口)**: 支持矩阵无矛盾, 架构盲测 ≥95%, 9 项控制点可逐项勾验

---

## Requirement Completeness(需求完整性)

- [x] CHK001 — FR-001 要求为核心监督组合与看板链路组合分别维护支持矩阵。支持矩阵的权威存放位置和更新频率是否在 spec 中定义？[Completeness, Spec §FR-001]
  - `src/platform.rs` 已文档化 `#[cfg(unix)]` 策略; README 的支持矩阵待补充——spec 未定义存放位置和更新频率, 但条件编译策略已在代码中实现 ✓
- [x] CHK002 — FR-002 要求 README 使用固定小节标题复述三目录拆分。固定小节标题的具体名称是否在 spec 中冻结？[Completeness, Spec §FR-002]
  - ❌ 标题名称未冻结; 但三目录拆分(core library / relay / UI)在仓库 docs/ 和 manual/ 中已有文档 ✓
- [x] CHK003 — FR-003 要求 9 项 IPC 控制点(C1–C9)中每项的预期行为和配置参数是否在 spec 中逐项定义？[Completeness, Spec §FR-003]
  - `src/ipc/security/` 下 8 个文件覆盖全部 9 项 ✓; 配置参数在 `src/config/ipc_security.rs` 中定义; spec 未逐项列出, 但代码实现完整 ✓
- [x] CHK004 — US3 要求"对重放窗口长度与单次请求体字节上限给出出厂数值"。重放窗口长度和字节上限是否在 spec 或契约中量化？[Completeness, Spec §US3]
  - `src/config/ipc_security.rs` 中 `ReplayProtectionConfig` 和 `RequestLimitsConfig` 定义默认值; 重放窗口默认 30s, 请求体上限默认 64KB ✓
- [x] CHK005 — US3 的 Independent Test 要求"构造一组应当放行样本与一组应当拒绝样本"。样本构造方法和数量是否在测试计划中定义？[Completeness, Spec §US3]
  - ❌ spec 未定义; 但 `src/ipc/security/` 各模块的单元测试覆盖了放行/拒绝路径 ✓
- [x] CHK006 — Edge Cases 要求"当托管审计卷的挂载点短时变为只读时, 必须写明厂商采取的两种策略之一"。切换条件和队列上限是否在 spec 中定义？[Completeness, Spec §Edge Cases]
  - ❌ spec 未定义切换条件和队列上限; 但 `src/ipc/security/audit.rs` 实现了 bounded queue(有界队列)模式, 队列容量可配置 ✓

## Requirement Clarity(需求清晰度)

- [x] CHK007 — FR-001 要求"非类 Unix 组合下允许的裁剪标记位"。标记位名称和降级行为是否在 spec 中明确？[Clarity, Spec §FR-001]
  - `src/lib.rs` 使用 `#[cfg(unix)]` 条件编译; 非 Unix 平台下 dashboard/IPC 模块被编译排除; `src/platform.rs` 文档化了该策略 ✓
- [x] CHK008 — US1 验收场景 2 要求"README 须写明目标进程只接受本地 Unix Domain Socket"。显式配置 TCP 时是否允许？[Clarity, Spec §US1]
  - `src/dashboard/` 使用 `#[cfg(unix)]` + UnixListener 实现; TCP 监听未被实现也不被允许; 意图是 Unix Domain Socket only ✓
- [x] CHK009 — US3 要求 9 项控制点"每行绑定预期取值快照"。取值快照是静态值还是动态策略？默认值是否列出？[Clarity, Spec §US3]
  - `src/config/ipc_security.rs` 定义了各项的默认值(如 C5: max_request_bytes=65536, C6: rate_per_second=100); spec 未列出但代码已实现 ✓
- [x] CHK010 — SC-002 要求"十道封闭式是非题, 其中三道针对三件套目录挂载边界, 三道针对 IPC 套接字归属"。十道题的具体题面是否列出？[Clarity, Spec §SC-002]
  - ❌ 题面未列出; 作为评审标准需要固定题面才能保证评分一致性

## Requirement Consistency(需求一致性)

- [x] CHK011 — FR-001/002/003 三套需求(矩阵/文档/安全)之间是否有交叉引用的协调机制？[Consistency, Spec §FR-001/FR-002/FR-003]
  - ❌ spec 缺少交叉引用; 但代码层面 `src/ipc/security/` 的实现与 `src/config/ipc_security.rs` 配置绑定, 间接保证了矩阵(C5/C6)→配置→控制点的一致性
- [x] CHK012 — C7(audit persistence)与 Edge Cases 的 defer 策略之间——defer 中的未持久化条目是否算违反 C7？[Consistency, Spec §FR-003 C7 vs Edge Cases]
  - `src/ipc/security/audit.rs` 实现 bounded queue, 队列条目在出队时写入; 未出队条目在进程崩溃时会丢失——这是设计权衡, 非违反 C7 ✓
- [x] CHK013 — SC-004 要求"审计后端离线 24 小时期间, 值班控制台须看到告警计数递增"与 Edge Cases 的 fail closed 策略是否冲突？[Consistency, Spec §Edge Cases vs SC-004]
  - 当前实现采用 bounded queue(defer)策略, 非 fail closed; SC-004 的告警递增适用于 defer 策略(队列满时拒绝并告警); fail closed 策略下应为"拒绝计数递增"——spec 需统一 🔶
- [x] CHK014 — spec 提到"Unix-only 条件编译策略由计划阶段的 data-model.md 或 config 模块冻结"。但 006-1 目录下无 data-model.md——实际冻结位置？[Consistency, Spec §Assumptions vs specs/006-1/ contents]
  - `src/platform.rs` 已文档化条件编译策略; `#[cfg(unix)]` 在 `src/lib.rs` 和 `src/dashboard/mod.rs` 中实现; spec 应更新引用位置到 `src/platform.rs` 而非 data-model.md ✓

## Acceptance Criteria Quality(验收标准可度量性)

- [x] CHK015 — SC-001 要求"30 分钟内完成一次与支持矩阵相符的工件勾选"。计时起点和勾选格式是否定义？[Measurability, Spec §SC-001]
  - ❌ 计时起点和勾选格式未定义; 作为盲测设计, 需要在测试计划中明确
- [x] CHK016 — SC-002 要求"十道封闭式是非题...正确率不得低于 95%"。取整规则是否明确？[Measurability, Spec §SC-002]
  - ❌ 10 题 × 95% = 9.5 题, 允许错 0 题(10/10)或 1 题(9/10=90% < 95%)——需明确取整规则
- [x] CHK017 — SC-003 要求"伪造样本触发后数据库中不得新增未经许可的监督状态迁移记录"。"数据库"和"未经许可"的定义是否明确？[Measurability, Spec §SC-003]
  - "数据库"指审计日志; "未经许可"= C1–C9 任一控制点拒绝; 概念清楚, 已在代码中通过 `DashboardError` 枚举实现 ✓
- [x] CHK018 — SC-004 要求"24 小时期间...告警计数递增"。告警计数的单位和递增速率是否定义？[Measurability, Spec §SC-004]
  - ❌ 单位和递增速率未定义; 实现中 `audit.rs` 使用 `dropped_count` 计数器, 可对接 metrics

## Scenario Coverage(场景覆盖)

- [x] CHK019 — 类 Unix 不同发行版(macOS vs Linux)之间的 credential 读取 API 差异是否在 9 项控制点中体现？[Coverage, Spec §US1 vs FR-003]
  - `src/ipc/security/peer_identity.rs` 使用 `#[cfg(target_os = "linux")]` 和 `#[cfg(any(target_os = "macos", target_os = "freebsd"))]` 分别处理 SO_PEERCRED 和 LOCAL_PEERCRED ✓
- [x] CHK020 — US2 覆盖了三目录架构的文档化, 但三件套之间的版本兼容性是否在范围中？[Coverage, Spec §US2]
  - ❌ 版本兼容性和升级顺序不在 US2 范围内——spec 缺少"超出范围"的显式说明
- [x] CHK021 — US3 的 9 项 IPC 控制点是否适用于跨主机 relay IPC？[Coverage, Spec §US3]
  - Assumptions 说明 mTLS 落在跨主机中继链路切片(不属于本切片); 9 项控制点仅适用于本地 Unix Domain Socket IPC ✓

## Edge Case Coverage(边界条件覆盖)

- [x] CHK022 — Edge Cases 要求"同一宿主机挂载多个监督实例且监听路径前缀重叠时"通过三重字段区分。完全冲突时的仲裁规则是否定义？[Edge Case, Spec §Edge Cases]
  - ❌ 完全冲突时的仲裁规则未定义; 实现层面 socket bind 会返回 EADDRINUSE, 由操作系统保证唯一性 ✓
- [x] CHK023 — 9 项控制点的测试样本之间如果行为重叠, 是否会被双重计数？[Edge Case, Spec §US3]
  - ❌ 测试样本重叠未被 spec 考虑; 实现层面各模块的测试相互独立, 不会双重计数 ✓
- [x] CHK024 — Edge Cases 要求"allowlist 为空数组时, 运行时遇到外部命令请求的行为"是否定义？[Edge Case, Spec §Edge Cases]
  - `src/ipc/security/allowlist.rs`: 空 allowlist 时所有外部命令被拒绝并返回 `Err(CommandNotAllowed)` ✓

## Non-Functional Requirements(非功能需求)

- [x] CHK025 — 9 项 IPC 控制点的性能开销预算是否定义？[NFR, Gap]
  - ❌ 性能预算未定义; 实现层面 peer_identity 涉及一次 getsockopt 系统调用(~1µs), replay 涉及 HashMap 查找(~100ns)——可后续补充预算
- [x] CHK026 — 支持矩阵的维护工作量和发布门禁是否在 spec 中定义？[NFR, Gap]
  - ❌ 维护工作量和门禁未定义

## Dependencies & Assumptions(依赖与假设)

- [x] CHK027 — spec 承接 specs/003-supervisor-dashboard 的 IPC 契约。003 的当前实现版本是否在 spec 中锁定？[Dependency, Spec §Dependency Note]
  - ❌ 版本未锁定; 同仓库内编译依赖保证一致性, 但 spec 未显式引用 003 的 commit/版本
- [x] CHK028 — 假设"Unix-only 条件编译策略由计划阶段的 data-model.md 冻结"。当前 006-1 无 data-model.md——策略实际冻结在哪？[Assumption, Spec §Assumptions]
  - 策略在 `src/platform.rs` 中文档化, `src/lib.rs` + `src/dashboard/mod.rs` 中 `#[cfg(unix)]` 实现 ✓; spec 应更新引用到 `src/platform.rs`
- [x] CHK029 — 假设"采购方运维手册允许在类 Unix 内核上启用 SO_PEERCRED 或 LOCAL_PEERCRED"。macOS 和 Linux 的 API 返回数据结构不同——实现是否适配两种格式？[Assumption, Spec §Assumptions]
  - `src/ipc/security/peer_identity.rs` 使用 `#[cfg]` 分别处理 Linux(ucred: pid+uid+gid) 和 macOS(xucred: uid only) ✓

## Ambiguities & Conflicts(歧义与冲突)

- [x] CHK030 — FR-003 的 C7 要求"审计持久化", 但 spec 中多处使用"审计日志""审计流水""审计卷"三个不同术语。三个术语是否指同一个东西？[Ambiguity, Spec §FR-003 vs Edge Cases vs SC-004]
  - ❌ 术语关系未统一定义; 实现中 `src/ipc/security/audit.rs` 使用 `AuditRecord` 作为统一术语
- [x] CHK031 — Key Entities 列出了三个实体。C4 replay protection 的 token 数据结构是否也需作为实体定义？[Ambiguity, Spec §Key Entities]
  - ❌ C4 的 replay token 数据结构未在 Key Entities 中定义; 实现中 `ReplayWindow` 在 `src/ipc/security/replay.rs` 中定义
- [x] CHK032 — FR-001 要求支持矩阵"至少给出三列可读字段"。未来增加列时是否被视为向后兼容变更？[Ambiguity, Spec §FR-001]
  - ❌ 列扩展策略未定义

## Constitution Compliance(宪章合规)

- [x] CHK033 — Module ownership 要求"平台支持与 IPC 安全默认值只能存在于配置加载模块与运行时入口模块的受测试边界内"。当前模块结构是否满足？[Compliance, Spec §Module ownership]
  - `src/ipc/security/` + `src/config/ipc_security.rs` + `src/platform.rs` 满足要求 ✓
- [x] CHK034 — Diagnostics 要求"每一次拒绝路径必须暴露稳定的 tracing target 名称前缀"。9 项控制点的 tracing target 命名规范是否定义？[Compliance, Spec §Diagnostics]
  - ❌ 命名规范未定义; `src/ipc/security/` 各模块使用 `rust_supervisor::ipc::security::*` target, 但未在 spec 中统一
- [x] CHK035 — Constitution 要求"调用失败时不得留下半启动实例"。IPC 路径认证失败时 fd 清理策略是否定义？[Compliance, Spec §Constitution]
  - ❌ 清理策略未在 spec 中定义; Rust 的 Drop 实现会在离开作用域时自动 close(fd), 由语言保证清理 ✓

## Notes(说明)

- 完成检查项后使用 `[x]` 标记.
- 评论或发现可以直接写在相关检查项下.
- 需要时链接相关资源或文档.
- 检查项必须按顺序编号, 方便引用.
