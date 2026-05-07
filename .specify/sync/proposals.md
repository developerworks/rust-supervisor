# Drift Resolution Proposals(漂移处置提案)

Generated(生成时间): 2026-05-08T00:36:41+08:00
Based on(基于): drift-report from 2026-05-08T00:34:25+08:00

## Summary(摘要)

| Resolution Type(处置类型) | Count(数量) |
|---|---:|
| BACKFILL(回填, Code to Spec(代码到规格)) | 2 |
| ALIGN(对齐, Spec to Code(规格到代码)) | 0 |
| HUMAN_DECISION(人工决策) | 0 |
| NEW_SPEC(新规格) | 1 |
| REMOVE_FROM_SPEC(从规格移除) | 0 |

## Proposals(提案)

### Proposal 1: 001-create-supervisor-core/FR-063

**Direction(方向)**: BACKFILL(回填, Code to Spec(代码到规格))

**Current State(当前状态)**:
- Spec says(规格说明): "系统代码命名不得使用任何 `*Snapshot` 或 `*View` 后缀, 也不得提供 `snapshot()` 查询方法."
- Code did before apply(应用前代码行为): `src/dashboard/model.rs` 定义旧 dashboard payload(看板载荷) 类型, `src/dashboard/ipc_server.rs` 暴露旧 dashboard query(看板查询) 方法, 这些行为来自 003 dashboard(看板) 规格.

**Proposed Resolution(拟议处置)**:

把 001 的 FR-063 改为:

```markdown
- **FR-063**: 系统核心状态和配置命名不得使用任何 `*Snapshot` 或 `*View` 后缀, 也不得在 `SupervisorHandle`(监督器句柄), runtime(运行时), state(状态) 或 config(配置) 查询接口提供 `snapshot()` 方法. 配置加载结果必须命名为 `ConfigState`(配置状态), 监督器当前状态必须命名为 `SupervisorState`(监督器状态), 子任务当前状态必须命名为 `ChildState`(子任务状态), 运行时查询命令必须命名为 `current_state`(当前状态), 源码模块必须命名为 `state`(状态), 不得命名为 `state_view`(状态视图). 后续 dashboard(看板) 规格明确拥有的 IPC(进程间通信) 或 UI(用户界面) 协议对象可以使用 `snapshot(快照)` 术语, 但该术语不得替代核心状态查询契约.
```

**Rationale(理由)**: 003 是后续规格, 它明确要求 snapshot(快照) 协议对象和首包语义. 当前代码有对应实现和测试, `cargo test --test dashboard_config_test --test dashboard_protocol_shape_test --test dashboard_snapshot_test --test dashboard_stream_test --test dashboard_performance_test` 已通过. 因此代码和 003 规格代表有意演化, 001 应该缩窄命名禁令的适用范围.

**Confidence(置信度)**: HIGH(高)

**Apply Status(应用状态)**: APPLIED_THEN_SUPERSEDED(已应用后被覆盖), 2026-05-08T00:46:58+08:00. P002 在 2026-05-08T00:52:04+08:00 覆盖了 dashboard(看板)例外.

**Action(操作)**:
- [x] Approve(批准)
- [ ] Reject(拒绝)
- [ ] Modify(修改)

---

### Proposal 2: 001-create-supervisor-core/SC-031

**Direction(方向)**: BACKFILL(回填, Code to Spec(代码到规格))

**Current State(当前状态)**:
- Spec says(规格说明): "naming check(命名检查) 必须证明源码, 示例, 公开契约和文档中不存在任何 `*Snapshot`, `*View`, `snapshot()` 查询方法或 `state_view` 模块名."
- Code does(代码行为): `src/tests/naming_contract_test.rs` 对 `src/dashboard/` 做了例外跳过, 所以当前测试已经表达 dashboard(看板) 协议例外, 但 001 的 SC-031 没有记录这个例外.

**Proposed Resolution(拟议处置)**:

把 001 的 SC-031 改为:

```markdown
- **SC-031**: naming check(命名检查) 必须证明全仓源码, 示例, 公开契约和文档中不存在任何 `*Snapshot`, `*View`, `snapshot()` 查询方法或 `state_view` 模块名, 并且统一使用 `ConfigState`(配置状态), `SupervisorState`(监督器状态), `ChildState`(子任务状态), `current_state`(当前状态) 和 `state`(状态). 检查不得跳过 dashboard(看板), IPC(进程间通信) 或 UI(用户界面) 协议边界.
```

新增或调整 acceptance scenario(验收场景):

```markdown
Given(假设) 维护者在任意仓库源码, 示例, 公开契约或文档中新增 `*Snapshot`, `*View`, `snapshot()` 或 `state_view`, When(当) naming check(命名检查) 运行, Then(则) 检查必须失败.
Given(假设) dashboard(看板), IPC(进程间通信) 或 UI(用户界面) 协议需要表达 snapshot(快照) 语义, When(当) 命名和检查运行, Then(则) 它必须使用不带 `*Snapshot` 或 `*View` 的替代命名, 并且不得跳过该协议边界.
```

**Rationale(理由)**: 用户明确要求全仓禁止 `*Snapshot` 和 `*View`, 因此本次应用不采用原 proposal(提案) 中的 dashboard(看板)例外, 并同步覆盖 Proposal 1 中已经写入的例外口径.

**Confidence(置信度)**: HIGH(高)

**Apply Status(应用状态)**: APPLIED_WITH_MODIFICATION(修改后已应用), 2026-05-08T00:52:04+08:00.

**Action(操作)**:
- [x] Approve(批准)
- [ ] Reject(拒绝)
- [x] Modify(修改)

---

### Proposal 9: New Spec for Spec Kit sync extension(规格工具同步扩展)

**Direction(方向)**: NEW_SPEC(新规格)

**Feature(功能)**: Spec Kit sync extension(规格工具同步扩展) 本地命令和技能资产
**Location(位置)**: `.specify/extensions/sync/`, `.agents/skills/speckit-sync-analyze/SKILL.md`

**Draft Spec(规格草案)**:

```markdown
# Feature Specification(功能规格): Spec Kit 同步扩展工具

## User Scenarios(用户场景)

### User Story 1(用户故事一) - 分析规格和实现漂移

维护者需要在本地 Spec Kit(规格工具) 工作区运行 sync analyze(同步分析), 读取 `specs/*/spec.md`, 源码, 测试和设计文档, 并生成 Markdown(标记语言) 和 JSON(数据交换格式) 漂移报告.

### User Story 2(用户故事二) - 生成处置提案

维护者需要在已有 drift report(漂移报告) 基础上生成 backfill(回填), align(对齐), human decision(人工决策) 和 new spec(新规格) 提案, 并把提案保存为可审查文件.

### User Story 3(用户故事三) - 管理同步配置和产物

维护者需要通过 `sync-config.yml` 控制 ignore pattern(忽略规则), design doc pattern(设计文档规则), artifact directory(产物目录), history retention(历史保留) 和默认提案策略.

## Requirements(需求)

- **FR-001**: 系统必须提供 `speckit-sync-analyze`, `speckit-sync-propose`, `speckit-sync-apply`, `speckit-sync-backfill` 和 `speckit-sync-conflicts` 本地技能或命令入口.
- **FR-002**: 系统必须读取 `.specify/extensions/sync/sync-config.yml`, 并支持 `ignore_patterns`, `design_doc_patterns`, `default_strategy`, `artifacts_dir`, `keep_history` 和 `history_limit`.
- **FR-003**: sync analyze(同步分析) 必须输出 `.specify/sync/drift-report.md` 和 `.specify/sync/drift-report.json`.
- **FR-004**: sync propose(同步提案) 必须输出 `.specify/sync/proposals.md` 和 `.specify/sync/proposals.json`.
- **FR-005**: analyze(分析) 和 propose(提案) 必须保持 read-only(只读) 实现行为, 除写入同步产物外不得修改规格或源码.
- **FR-006**: apply(应用) 只能执行已批准的提案, 并且必须保留变更摘要和验证证据.
- **FR-007**: 本扩展不得提供 compatibility export(兼容导出), 历史命令别名或旧规格别名.
```

**Rationale(理由)**: 当前 sync extension(同步扩展) 文件已经存在, 但没有产品规格覆盖. 如果这些资产属于项目工具链, 新规格可以防止它们在后续同步分析中持续被判定为 unspecced code(无规格代码). 如果它们只是临时本地工具, 可以在后续 apply(应用) 阶段改为 ignore pattern(忽略规则), 不创建新规格.

**Confidence(置信度)**: MEDIUM(中)

**Action(操作)**:
- [ ] Approve and create spec(批准并创建规格)
- [ ] Reject(拒绝)
- [ ] Modify(修改)
