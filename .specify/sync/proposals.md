# Drift Resolution Proposals(漂移处置提案)

Generated(生成时间): 2026-05-12T02:06:13+08:00
Based on(基于): drift-report from 2026-05-12T02:03:03+08:00
Mode(模式): INTERACTIVE(交互式)

## Summary(摘要)

| Resolution Type(处置类型) | Count(数量) |
|---|---:|
| BACKFILL(回填, Code to Spec(代码到规格)) | 0 |
| ALIGN(对齐, Spec to Code(规格到代码)) | 2 |
| HUMAN_DECISION(人工决策) | 0 |
| NEW_SPEC(新规格) | 1 |
| REMOVE_FROM_SPEC(从规格移除) | 0 |

## Proposals(提案)

### Proposal 1: 003-supervisor-dashboard/SC-003

**Direction(方向)**: ALIGN(对齐, Spec to Code(规格到代码))

**Current State(当前状态)**:
- Spec says(规格说明): `/Users/0x00/Documents/rust-supervisor-ui/tests/dashboard-performance.spec.ts` 必须使用包含 5 个目标进程, 200 个 child task(子任务), failed(失败), quarantined(隔离) 和 restarting(重启中) 节点的固定测试数据集重复执行 20 次定位流程, 其中至少 19 次必须在 30 秒内定位到指定异常 child task(子任务) 及其最近事件.
- Code does(代码行为): 当前 `dashboard-performance.spec.ts` 只执行 1 次定位流程, `src/mock/dashboardData.ts` 只有 3 个 target process(目标进程) 和少量 child task(子任务), `T066` 仍是未完成状态.
- Evidence(证据): `/Users/0x00/Documents/rust-supervisor-ui/tests/dashboard-performance.spec.ts:3`, `/Users/0x00/Documents/rust-supervisor-ui/src/mock/dashboardData.ts:149`, `specs/003-supervisor-dashboard/tasks.md:143`.

**Proposed Resolution(拟议处置)**:

保持 `SC-003` 不变, 并补齐 UI(用户界面) 性能定位测试和固定数据集.

```markdown
- 在 `/Users/0x00/Documents/rust-supervisor-ui/src/mock/dashboardData.ts` 或测试专用 fixture(测试夹具) 中生成固定数据集: 5 个 target process(目标进程), 总计 200 个 child task(子任务), 且至少包含 failed(失败), quarantined(隔离) 和 restarting(重启中) 节点及对应 recent event(最近事件).
- 在 `/Users/0x00/Documents/rust-supervisor-ui/tests/dashboard-performance.spec.ts` 中把单次定位流程改为 20 次固定用例定位流程.
- 每次定位都必须从 dashboard(看板) 找到指定异常 child task(子任务), 再验证节点详情或事件区域包含最近事件.
- 测试必须记录成功次数, 并要求 20 次中至少 19 次在 30 秒内完成.
- `T066` 只有在该测试实现并通过后才能重新标记为完成.
```

**Rationale(理由)**: `SC-003` 是刚收紧的可执行验收口径, 其目的是替代主观的用户成功率表达. 现有代码只覆盖一次页面可见性检查, 不能证明 5 个目标进程和 200 个子任务规模下的 20 次定位成功率. 因此应该让实现和测试追上规格, 而不是降低规格.

**Confidence(置信度)**: HIGH(高)

**Review Status(审查状态)**: APPROVED(已批准), 2026-05-12T02:16:52+08:00.

**Action(操作)**:
- [x] Approve(批准)
- [ ] Reject(拒绝)
- [ ] Modify(修改)

---

### Proposal 2: 003-supervisor-dashboard/SC-012

**Direction(方向)**: ALIGN(对齐, Spec to Code(规格到代码))

**Current State(当前状态)**:
- Spec says(规格说明): dashboard client(看板客户端) 交付物中 Vue(网页界面框架), shadcn-vue(组件库) 和 Tailwind(样式框架) 基线必须可验证, React(网页界面库) runtime dependency(运行时依赖) 和 React(网页界面库) component file(组件文件) 数量必须为 0.
- Code does(代码行为): `package.json` 实际没有 React(网页界面库) 依赖, `src/` 下也没有 `.tsx` 或 `.jsx` 文件, 但 `dashboard-performance.spec.ts` 只断言 `window.__RUST_SUPERVISOR_UI_BASELINE__`, 没有把 React(网页界面库) 依赖和组件文件数量写成自动验证.
- Evidence(证据): `/Users/0x00/Documents/rust-supervisor-ui/tests/dashboard-performance.spec.ts:11`, `/Users/0x00/Documents/rust-supervisor-ui/package.json:14`, `specs/003-supervisor-dashboard/tasks.md:143`.

**Proposed Resolution(拟议处置)**:

保持 `SC-012` 不变, 并把当前真实状态固化为自动测试.

```markdown
- 在 `/Users/0x00/Documents/rust-supervisor-ui/tests/dashboard-performance.spec.ts` 中保留 Vue(网页界面框架), shadcn-vue(组件库) 和 Tailwind(样式框架) baseline(基线) 断言.
- 在同一个测试或一个新的 Vitest(前端测试工具) 测试中读取 `package.json`, 验证 `dependencies` 中 `react`, `react-dom` 和其它 React(网页界面库) runtime dependency(运行时依赖) 数量为 0.
- 扫描 `/Users/0x00/Documents/rust-supervisor-ui/src/`, 验证 `.tsx` 和 `.jsx` component file(组件文件) 数量为 0.
- 如果要检查 dev dependency(开发依赖), 可以把 `@vitejs/plugin-react` 也列为拒绝项, 但不得把非运行时开发工具误判为 runtime dependency(运行时依赖).
- `T066` 只有在这些断言进入自动测试并通过后才能重新标记为完成.
```

**Rationale(理由)**: 代码当前看起来符合 `SC-012` 的结果要求, 但规格要求的是可验证交付物. 缺口不是产品行为错误, 而是 proof(证明) 缺失. 正确处置是补测试, 不是修改规格.

**Confidence(置信度)**: HIGH(高)

**Review Status(审查状态)**: APPROVED(已批准), 2026-05-12T02:18:57+08:00.

**Action(操作)**:
- [x] Approve(批准)
- [ ] Reject(拒绝)
- [ ] Modify(修改)

---

### Proposal 3: New Spec for Spec Kit sync extension(规格工具同步扩展)

**Direction(方向)**: NEW_SPEC(新规格)

**Feature(功能)**: Spec Kit sync extension(规格工具同步扩展) 本地命令和技能资产

**Location(位置)**: `.specify/extensions/sync/`, `.agents/skills/speckit-sync-*`

**Suggested Spec(建议规格)**: `004-spec-sync-tooling`

**Current State(当前状态)**:
- Code does(代码行为): `.specify/extensions/sync/` 和 `.agents/skills/speckit-sync-*` 已经提供 analyze(分析), propose(提案), apply(应用), backfill(回填) 和 conflicts(冲突检测) 相关本地资产, 当前统计 2554 行.
- Spec says(规格说明): 当前 `specs/` 中没有对应产品规格覆盖这些本地工具资产.

**Draft Spec(规格草案)**:

```markdown
# Feature Specification(功能规格): Spec Kit 同步扩展工具

## User Scenarios(用户场景)

### User Story 1(用户故事一) - 分析规格和实现漂移

维护者需要在本地 Spec Kit(规格工具) 工作区运行 sync analyze(同步分析), 读取 `specs/*/spec.md`, 源码, 测试和设计文档, 并生成 Markdown(标记语言) 和 JSON(数据交换格式) drift report(漂移报告).

### User Story 2(用户故事二) - 生成漂移处置提案

维护者需要在已有 drift report(漂移报告) 基础上生成 backfill(回填), align(对齐), human decision(人工决策) 和 new spec(新规格) proposal(提案), 并把提案保存为可审查文件.

### User Story 3(用户故事三) - 应用已批准处置

维护者需要只应用已经批准的 proposal(提案), 并在修改规格或代码前创建 backup(备份), 在完成后写入 apply report(应用报告).

## Requirements(需求)

- **FR-001**: 系统必须提供 `speckit-sync-analyze`, `speckit-sync-propose`, `speckit-sync-apply`, `speckit-sync-backfill` 和 `speckit-sync-conflicts` 本地技能或命令入口.
- **FR-002**: 系统必须读取 `.specify/extensions/sync/sync-config.yml`, 并支持 analysis design doc pattern(设计文档规则), ignore pattern(忽略规则), artifact directory(产物目录), history retention(历史保留) 和 default strategy(默认策略).
- **FR-003**: sync analyze(同步分析) 必须输出 `.specify/sync/drift-report.md` 和 `.specify/sync/drift-report.json`.
- **FR-004**: sync propose(同步提案) 必须输出 `.specify/sync/proposals.md` 和 `.specify/sync/proposals.json`, 并记录 proposal id(提案编号), target(目标), direction(方向), confidence(置信度), rationale(理由) 和 action(操作).
- **FR-005**: sync apply(同步应用) 只能执行已批准 proposal(提案), 必须在修改规格前创建 backup(备份), 并写入 `.specify/sync/apply-report.md` 和 `.specify/sync/apply-report.json`.
- **FR-006**: 所有命令必须遵守 ignore pattern(忽略规则), 默认排除 `node_modules`, `target`, `dist`, backup(备份) 和 generated artifact(生成产物).
- **FR-007**: 本扩展不得提供 compatibility export(兼容导出), 历史命令别名或旧规格别名.

## Success Criteria(成功标准)

- **SC-001**: 给定存在 drifted requirement(漂移需求) 的报告, 当运行 sync propose(同步提案), 则系统必须生成包含 proposal id(提案编号), target(目标), direction(方向), confidence(置信度), action(操作) 和 rationale(理由) 的 Markdown(标记语言) 与 JSON(数据交换格式) 文件.
- **SC-002**: 给定存在 unspecced feature(无规格功能) 的报告, 当运行 sync propose(同步提案), 则系统必须生成 new spec(新规格) 草案, 并保持 pending(待处理) 状态.
- **SC-003**: 给定没有 approved(已批准) action(操作), 当运行 sync apply(同步应用), 则系统不得修改规格或代码, 并必须写入 no-op(无操作) apply report(应用报告).
```

**Rationale(理由)**: 这是 drift report(漂移报告) 中唯一无规格代码项. 如果这些 sync(同步) 资产属于项目交付能力, 应创建 `004-spec-sync-tooling`; 如果它们只是本地开发工具, 应在 sync config(同步配置) 或漂移报告规则中明确排除.

**Confidence(置信度)**: MEDIUM(中)

**Review Status(审查状态)**: APPROVED_AND_CREATED_SPEC(已批准并已创建规格), 2026-05-12T02:21:08+08:00.

**Created Spec(已创建规格)**: `specs/004-spec-sync-tooling/spec.md`

**Action(操作)**:
- [x] Approve and create spec(批准并创建规格)
- [ ] Reject(拒绝)
- [ ] Modify(修改)
