# Drift Resolution Proposals(漂移处置提案)

Generated(生成时间): 2026-05-08T00:36:41+08:00
Based on(基于): drift-report from 2026-05-08T00:34:25+08:00

## Summary(摘要)

| Resolution Type(处置类型) | Count(数量) |
|---|---:|
| BACKFILL(回填, Code to Spec(代码到规格)) | 2 |
| ALIGN(对齐, Spec to Code(规格到代码)) | 6 |
| HUMAN_DECISION(人工决策) | 0 |
| NEW_SPEC(新规格) | 1 |
| REMOVE_FROM_SPEC(从规格移除) | 0 |

## Proposals(提案)

### Proposal 1: 001-create-supervisor-core/FR-063

**Direction(方向)**: BACKFILL(回填, Code to Spec(代码到规格))

**Current State(当前状态)**:
- Spec says(规格说明): "系统代码命名不得使用任何 `*Snapshot` 或 `*View` 后缀, 也不得提供 `snapshot()` 查询方法."
- Code does(代码行为): `src/dashboard/model.rs` 定义 `DashboardSnapshot`, `src/dashboard/ipc_server.rs` 定义 `DashboardIpcService::snapshot`, 这些行为来自 003 dashboard(看板) 规格.

**Proposed Resolution(拟议处置)**:

把 001 的 FR-063 改为:

```markdown
- **FR-063**: 系统核心状态和配置命名不得使用任何 `*Snapshot` 或 `*View` 后缀, 也不得在 `SupervisorHandle`(监督器句柄), runtime(运行时), state(状态) 或 config(配置) 查询接口提供 `snapshot()` 方法. 配置加载结果必须命名为 `ConfigState`(配置状态), 监督器当前状态必须命名为 `SupervisorState`(监督器状态), 子任务当前状态必须命名为 `ChildState`(子任务状态), 运行时查询命令必须命名为 `current_state`(当前状态), 源码模块必须命名为 `state`(状态), 不得命名为 `state_view`(状态视图). 后续 dashboard(看板) 规格明确拥有的 IPC(进程间通信) 或 UI(用户界面) 协议对象可以使用 `snapshot(快照)` 术语, 但该术语不得替代核心状态查询契约.
```

**Rationale(理由)**: 003 是后续规格, 它明确要求 snapshot(快照) 协议对象和首包语义. 当前代码有对应实现和测试, `cargo test --test dashboard_config_test --test dashboard_protocol_shape_test --test dashboard_snapshot_test --test dashboard_stream_test --test dashboard_performance_test` 已通过. 因此代码和 003 规格代表有意演化, 001 应该缩窄命名禁令的适用范围.

**Confidence(置信度)**: HIGH(高)

**Apply Status(应用状态)**: APPLIED(已应用), 2026-05-08T00:46:58+08:00.

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
- **SC-031**: naming check(命名检查) 必须证明核心源码, 示例, 公开契约和文档中不存在任何 `*Snapshot`, `*View`, `snapshot()` 查询方法或 `state_view` 模块名, 并且统一使用 `ConfigState`(配置状态), `SupervisorState`(监督器状态), `ChildState`(子任务状态), `current_state`(当前状态) 和 `state`(状态). dashboard(看板) 规格明确拥有的 IPC(进程间通信) 和 UI(用户界面) 协议对象可以使用 `snapshot(快照)`, 但检查必须证明该例外只出现在 dashboard(看板) 协议边界, 不得扩散到核心状态查询接口.
```

新增或调整 acceptance scenario(验收场景):

```markdown
Given(假设) 维护者在 `src/dashboard/` 之外新增 `*Snapshot`, `*View`, `snapshot()` 或 `state_view`, When(当) naming check(命名检查) 运行, Then(则) 检查必须失败.
Given(假设) dashboard(看板) 协议需要 `snapshot(快照)` 首包, When(当) naming check(命名检查) 运行, Then(则) 检查必须允许该术语留在 dashboard(看板) 协议边界, 并证明核心状态查询仍然使用 `current_state`(当前状态).
```

**Rationale(理由)**: 当前测试已经通过例外路径表达实际边界, 但规格仍写成全局禁止. 回填 SC(成功标准) 可以让测试口径和规格一致, 同时避免 dashboard(看板) 术语继续扩散.

**Confidence(置信度)**: HIGH(高)

**Action(操作)**:
- [ ] Approve(批准)
- [ ] Reject(拒绝)
- [ ] Modify(修改)

---

### Proposal 3: 004-agent-retrieval-rules/FR-001-FR-002

**Direction(方向)**: ALIGN(对齐, Spec to Code(规格到代码))

**Current State(当前状态)**:
- Spec says(规格说明): 系统必须维护 risk pattern(风险模式) 清单, 并让每个 risk pattern(风险模式) 包含适用条件, 触发信号, 所需证据, 排除条件, 当前相关度和最近验证时间.
- Code does(代码行为): 当前 `src/` 没有 risk pattern(风险模式) 模块, 没有 risk pattern(风险模式) 数据模型, 也没有对应测试.

**Proposed Resolution(拟议处置)**:

保留 FR-001 和 FR-002, 并在 004 的 plan(计划) 和 tasks(任务) 阶段新增实现工作. 建议的实现边界是 `src/retrieval_rules/risk.rs`, `src/retrieval_rules/mod.rs` 和 `src/retrieval_rules/tests/risk_pattern_test.rs`. 第一组测试必须先覆盖当前项目已出现问题, 尚未出现但相关的经验问题, 适用理由, 证据需求和排除条件.

**Rationale(理由)**: 004 是 Draft(草稿) 新功能, 未实现不是代码 bug(缺陷). 需求仍然是该功能的 P1(第一优先级) 输入, 不应从规格删除.

**Confidence(置信度)**: MEDIUM(中)

**Action(操作)**:
- [ ] Approve(批准)
- [ ] Reject(拒绝)
- [ ] Modify(修改)

---

### Proposal 4: 004-agent-retrieval-rules/FR-003-FR-006

**Direction(方向)**: ALIGN(对齐, Spec to Code(规格到代码))

**Current State(当前状态)**:
- Spec says(规格说明): 系统必须生成 evidence plan(证据计划), 建立 causal chain(因果链), 检测证据缺失和冲突, 并把未充分验证的结论标记为 hypothesis(假设).
- Code does(代码行为): 当前 `src/` 没有 evidence plan(证据计划), evidence record(证据记录), causal chain(因果链), evidence diagnostic(证据诊断) 或 hypothesis(假设) 标记实现.

**Proposed Resolution(拟议处置)**:

保留 FR-003 到 FR-006, 并新增 `src/retrieval_rules/evidence.rs`, `src/retrieval_rules/causal.rs`, `src/retrieval_rules/diagnostics.rs` 和对应外部测试. 测试必须先覆盖单一来源依赖, 来源过期, 来源冲突, 因果链断点和 hypothesis(假设) 降级.

**Rationale(理由)**: 这组需求是 004 的核心质量门禁. 删除它们会让功能只剩任务拆分, 不能满足用户关于信息检索完整性和因果链稳健性的原始输入.

**Confidence(置信度)**: MEDIUM(中)

**Action(操作)**:
- [ ] Approve(批准)
- [ ] Reject(拒绝)
- [ ] Modify(修改)

---

### Proposal 5: 004-agent-retrieval-rules/FR-007-FR-008

**Direction(方向)**: ALIGN(对齐, Spec to Code(规格到代码))

**Current State(当前状态)**:
- Spec says(规格说明): 系统必须支持 rule evolution(规则演化), 并记录旧规则, 新规则, 变更原因, 适用范围, 生效时间, 回滚条件和审查状态.
- Code does(代码行为): 当前 `src/` 没有 behavior rule(行为规则) 模型, rule evolution record(规则演化记录) 或审查状态.

**Proposed Resolution(拟议处置)**:

保留 FR-007 和 FR-008, 并新增 `src/retrieval_rules/rule.rs`, `src/retrieval_rules/evolution.rs` 和 `src/retrieval_rules/tests/rule_evolution_test.rs`. 测试必须证明人工反馈, 遗漏, 矛盾和误判可以生成待审查规则更新, 并且每条记录都有回滚条件.

**Rationale(理由)**: 规则演化是 004 和普通一次性分析工具的边界. 当前没有实现, 但没有证据表明该能力过时.

**Confidence(置信度)**: MEDIUM(中)

**Action(操作)**:
- [ ] Approve(批准)
- [ ] Reject(拒绝)
- [ ] Modify(修改)

---

### Proposal 6: 004-agent-retrieval-rules/FR-009-FR-013

**Direction(方向)**: ALIGN(对齐, Spec to Code(规格到代码))

**Current State(当前状态)**:
- Spec says(规格说明): 系统必须把复杂问题分解为 parallel subtask(并行子任务), 支持多个 agent(智能体) 同时检索, 合并结果, 迭代细化并定义停止条件.
- Code does(代码行为): 当前 `src/` 没有 parallel subtask(并行子任务), agent result(智能体结果), result merge(结果合并), iterative refinement(迭代细化) 或 stop condition(停止条件) 实现.

**Proposed Resolution(拟议处置)**:

保留 FR-009 到 FR-013, 并新增 `src/retrieval_rules/subtask.rs`, `src/retrieval_rules/agent.rs`, `src/retrieval_rules/iteration.rs` 和 `src/retrieval_rules/tests/parallel_iteration_test.rs`. 实现必须先用纯数据模型和同步合并逻辑交付, 并把实际并行执行方式留给 plan(计划) 阶段确认.

**Rationale(理由)**: 004 的 assumption(假设) 已经说明多个 agent(智能体) 可以是并行分析角色或并行执行单元. 因此第一版可以先实现可验证的数据和合并规则, 不必立即引入新的异步执行框架.

**Confidence(置信度)**: MEDIUM(中)

**Action(操作)**:
- [ ] Approve(批准)
- [ ] Reject(拒绝)
- [ ] Modify(修改)

---

### Proposal 7: 004-agent-retrieval-rules/FR-014-FR-016

**Direction(方向)**: ALIGN(对齐, Spec to Code(规格到代码))

**Current State(当前状态)**:
- Spec says(规格说明): 系统必须生成 final synthesis(最终汇总), 提供结构化诊断, 并不得提供 compatibility export(兼容导出), 旧规则别名或历史任务别名.
- Code does(代码行为): 当前 `src/` 没有 synthesis report(汇总报告), retrieval diagnostics(检索诊断) 或 004 专属 no compatibility check(无兼容检查).

**Proposed Resolution(拟议处置)**:

保留 FR-014 到 FR-016, 并新增 `src/retrieval_rules/synthesis.rs`, `src/retrieval_rules/diagnostics.rs` 和 `src/retrieval_rules/tests/synthesis_diagnostics_test.rs`. 同时在 004 tasks(任务) 中加入 module boundary(模块边界), no compatibility export(无兼容导出) 和结构化诊断测试.

**Rationale(理由)**: final synthesis(最终汇总) 是用户可见结果. 没有它, 前面风险模式, 证据计划和并行子任务都无法形成可审查输出.

**Confidence(置信度)**: MEDIUM(中)

**Action(操作)**:
- [ ] Approve(批准)
- [ ] Reject(拒绝)
- [ ] Modify(修改)

---

### Proposal 8: 004-agent-retrieval-rules/SC-001-SC-007

**Direction(方向)**: ALIGN(对齐, Spec to Code(规格到代码))

**Current State(当前状态)**:
- Spec says(规格说明): 系统必须通过 20 个经验风险样本, 证据缺失和冲突样本, 两轮迭代, 至少 3 个并行子任务, 规则演化记录, final synthesis(最终汇总) 和停止条件说明完成验收.
- Code does(代码行为): 当前仓库没有这些样本, 测试或验收证据.

**Proposed Resolution(拟议处置)**:

保留 SC-001 到 SC-007, 并在 004 tasks(任务) 中先创建测试数据和外部测试. 建议测试文件包括 `src/retrieval_rules/tests/risk_sample_test.rs`, `src/retrieval_rules/tests/evidence_gap_test.rs`, `src/retrieval_rules/tests/parallel_subtask_test.rs`, `src/retrieval_rules/tests/rule_evolution_test.rs` 和 `src/retrieval_rules/tests/final_synthesis_test.rs`. 这些测试必须先失败, 再由实现通过.

**Rationale(理由)**: 这些 SC(成功标准) 是可量化验收门槛. 它们未实现是因为功能尚未进入实现阶段, 不是因为需求应删除.

**Confidence(置信度)**: MEDIUM(中)

**Action(操作)**:
- [ ] Approve(批准)
- [ ] Reject(拒绝)
- [ ] Modify(修改)

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
