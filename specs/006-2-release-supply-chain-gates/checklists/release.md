# Release & Supply Chain Requirements Quality Checklist(发布门禁与供应链需求质量检查清单)

**Purpose(目的)**: 验证 `006-2-release-supply-chain-gates` 功能规格中发布记录追溯性、供应链闸口合规性和深度质量矩阵的需求质量、完整性与可度量性。

**Created(创建日期)**: 2026-05-19
**Scope(范围)**: US1(版本可追溯) + US2(供应链闸口) + US3(深度质量矩阵), 全部 3 个用户故事
**Depth(深度)**: Standard(标准)
**Audience(受众)**: Reviewer(PR 审查) + Release(发布责任人)
**Gates(关口)**: 四类指针可追溯, 闸口阻断有据, 深度矩阵空格率为 0

---

## Requirement Completeness(需求完整性)

- [x] CHK001 — FR-001 要求"发布流水线必须产出并长期保留"四类指针。"长期保留"的期限是否定义？[Completeness, Spec §FR-001]
  - ❌ 保留期限未量化; 当前 artifacts/ 目录随版本控制保留,Cargo.toml include 规则确保打包 ✓
- [x] CHK002 — FR-002 要求发布门禁固定包含 6 项检查。每项的通过条件是否在 spec 中逐项定义？[Completeness, Spec §FR-002]
  - `quality-gate-outcome.csv` 定义了 17 项门禁(含 6 项中层)的 tier/gate_id; 通过条件由各脚本的退出码决定 ✓
- [x] CHK003 — FR-003 要求深度检查(5 项)各留独立记录槽位。每项的阈值是否在 spec 中定义？[Completeness, Spec §FR-003]
  - `quality-gate-outcome.csv` 有 5 项深度门禁槽位; 阈值(如覆盖率)在 `scripts/check-coding-standard.sh` 等脚本中实现,但 spec 未单独列出阈值 ➖
- [x] CHK004 — US3 Independent Test 要求"断言深度矩阵相关列的空单元格数量为 0"。空单元格的判定标准是什么？[Completeness, Spec §US3]
  - `quality-gate-outcome.csv` 中空单元格=无 outcome 值; quality-gate-outcome.csv 每行在 CI 执行后填充 ✓; 但 spec 未定义"空"vs 无效枚举的区别 ❌
- [x] CHK005 — Edge Cases 要求"当 supply chain attestation 宿主服务短时故障时, 发布策略必须书面选定其一"。触发阈值和决策树是否定义？[Completeness, Spec §Edge Cases]
  - ❌ 决策树未定义; `artifacts/attestation.json` 的生成在 CI 中——如果服务故障, CI 步骤失败, 发布被阻断

## Requirement Clarity(需求清晰度)

- [x] CHK006 — FR-001 要求"与构件 tarball 同捆的 MSRV 自检脚本输出快照"。脚本文件名和位置是否明确？[Clarity, Spec §FR-001]
  - `scripts/verify-msrv.sh` 实现 MSRV 自检 ✓; `release-record.json` 的 msrv.verify_script 字段记录路径 ✓
- [x] CHK007 — FR-002 要求"策略失败时必须阻断放行入口, 或只允许附带 ExemptionTicket 编号的人工绕行节点"。绕行操作路径是否定义？[Clarity, Spec §FR-002]
  - `quality-gate-outcome.csv` 有 exemption_ticket + exemption_url 列; `artifacts/exemption-ticket.md` 作为模板 ✓; 绕行路径=填写工单编号并关联 URL
- [x] CHK008 — FR-003 要求 QualityGateOutcome 导出视图必须把空行标记为 incomplete。incomplete 是否映射到 Key Entities 的五类取值之一？[Clarity, Spec §FR-003 vs Key Entities]
  - ❌ incomplete 与 missing 的关系未明确; `quality-gate-outcome.csv` 使用空字符串表示未执行——不在 5 类枚举中
- [x] CHK009 — US1 验收场景 2 要求 MSRV 违规时"须在固定 5 步以内退出非零状态"。5 步是否与 SC-004 的 Step1–Step5 一致？[Clarity, Spec §US1 vs SC-004]
  - `scripts/verify-msrv.sh` 实现 5 步逻辑 ✓; US1 与 SC-004 一致 ✓; spec 未显式交叉引用,但实现已验证一致性 🔶
- [x] CHK010 — US2 验收场景 1 要求"流水线必须停在 blocked 状态, 并在台账附录附上坐标清单"。台账格式和坐标格式是否定义？[Clarity, Spec §US2]
  - `quality-gate-outcome.csv` 的 outcome=failed 行代表 blocked; detail 列记录坐标清单(如 dependency name) ✓

## Requirement Consistency(需求一致性)

- [x] CHK011 — FR-002 要求门禁包含 dependency audit 和 license policy 判定。输入源(deny.toml)是否与 FR-002 的 policy 一致？[Consistency, Spec §FR-002 vs repo contents]
  - `deny.toml` 已存在于仓库根目录 ✓; `cargo deny check` 命令使用该配置; FR-002 未显式引用 deny.toml 但 CI 实现已验证一致性 ✓
- [x] CHK012 — FR-003 要求深度检查包含 fuzzing, 但 US3 Independent Test 未引用 fuzzing。fuzzing 是否必检？[Consistency, Spec §FR-003 vs US3 Independent Test]
  - `quality-gate-outcome.csv` 有 fuzzing 槽位 ✓; FR-003 列出 5 项必检; Independent Test 只要求空单元格 = 0——fuzzing 属于必检, 未执行需豁免工单 ✓
- [x] CHK013 — SC-001 的中层门禁列表(4 项)与 FR-002(6 项)不完全一致。哪个是权威列表？[Consistency, Spec §FR-002 vs SC-001]
  - `quality-gate-outcome.csv` 有 6 项中层门禁, 与 FR-002 一致; SC-001 列表不完整——应以 FR-002 + quality-gate-outcome.csv 为准 🔶

## Acceptance Criteria Quality(验收标准可度量性)

- [x] CHK014 — SC-001 要求"发布记录表格均能在一页 A4 视图内找齐四类指针"。A4 视图的假设条件是否定义？[Measurability, Spec §SC-001]
  - `release-record.json` 格式紧凑, 一页 A4 可容纳 ✓; 字体和方向未定义,但 JSON 格式自动伸缩 ➖
- [x] CHK015 — SC-002 要求"SBOM 外部复验哈希完全一致率达到 100%"。样板数据集是否定义？[Measurability, Spec §SC-002]
  - `scripts/verify-sbom.sh` 验证 SBOM 哈希; `release-record.json` 的 sbom.sha256 字段记录参考值 ✓; 样板数据集同一次构建的 SBOM + attestation
- [x] CHK016 — SC-003 要求"深度质量矩阵相关列在无豁免样本集中空格率为 0%"。无豁免样本集的定义？[Measurability, Spec §SC-003]
  - 无豁免样本集 = 所有深度检查都在 CI 中执行且通过的发布; `quality-gate-outcome.csv` 的 outcome 列不为空即表示有值 ✓
- [x] CHK017 — SC-004 的 5 步 MSRV 自检脚本, 第 4 步使用 `cargo +<msrv> check`。toolchain 安装失败时的错误处理是否定义？[Measurability, Spec §SC-004]
  - `scripts/verify-msrv.sh` 实现了 Step3(rustup install); 如果 rustup 失败脚本退出非零并打印错误 ✓

## Scenario Coverage(场景覆盖)

- [x] CHK018 — US1 覆盖了 signed tag、changelog、semver、MSRV 四个维度。RC 与 stable release 的流程差异是否在 spec 中区分？[Coverage, Spec §US1]
  - ❌ RC 与 stable 流程差异未定义; CI 门禁相同——RC 和 stable 都经过同一套门禁
- [x] CHK019 — US2 覆盖了 dependency audit 等。私有依赖源的审计策略是否在范围中？[Coverage, Spec §US2]
  - ❌ 私有依赖源的审计策略未定义; `cargo deny` 默认检查所有依赖, 含 git dependencies ✓
- [x] CHK020 — US3 覆盖了 5 项深度检查。紧急发布跳过 loom/miri 的审批流程是否定义？[Coverage, Spec §US3]
  - ❌ 紧急跳过的审批流程未定义; `quality-gate-outcome.csv` 的 exemption_ticket 列可填写豁免工单

## Edge Case Coverage(边界条件覆盖)

- [x] CHK021 — Edge Cases 要求 PATCH 版本中误改高风险命令行的判定标准是否定义？[Edge Case, Spec §Edge Cases]
  - ❌ 判定标准未定义
- [x] CHK022 — Edge Cases 要求 SBOM schema 变更时附带迁移说明。双轨并存期限是否定义？[Edge Case, Spec §Edge Cases]
  - ❌ 双轨并存期限未定义; `release-record.json` 有 sbom.migration_note 字段,但旧版支持期限未规定
- [x] CHK023 — Edge Cases 要求"禁止无证明条目情况下直接把构件标成 released"。attestation 间隙(打包后签名前被篡改)的检测？[Edge Case, Spec §Edge Cases]
  - ❌ attestation 间隙的检测未定义; `scripts/generate-attestation.sh` 在 CI 中运行,提交哈希锁定防止篡改
- [x] CHK024 — release-record.json 本身被篡改时, 是否有检测机制？[Edge Case, Gap]
  - ❌ release-record.json 自身未签名; `scripts/generate-attestation.sh` 生成 attestation.json 包含 release-record.json 的 SHA-256 哈希——间接保护 🔶

## Non-Functional Requirements(非功能需求)

- [x] CHK025 — 中层门禁的执行时间预算是否定义？[NFR, Gap]
  - ❌ 时间预算未定义; `.github/workflows/middle-gates.yml` 无超时配置
- [x] CHK026 — 发布台账的存储格式和增长管理是否定义？[NFR, Gap]
  - `release-record.json` 单文件覆盖单次发布; `quality-gate-outcome.csv` 每次发布会覆盖; 不累积历史版本 ✓
- [x] CHK027 — MSRV 自检脚本的 toolchain 安装时间是否计入门禁时间预算？[NFR, Gap]
  - `scripts/verify-msrv.sh` 第二步检查 toolchain 是否已安装; 安装时间通常 10-30s,未单独量化但影响可接受 ✓

## Dependencies & Assumptions(依赖与假设)

- [x] CHK028 — spec 与 006-8 分工明确。ReleaseRecord JSON schema 的共享/同步机制是否定义？[Dependency, Spec §Dependency Note]
  - ❌ ReleaseRecord JSON schema 的同步机制未定义
- [x] CHK029 — 假设"组织已经具备代码签名与时间戳服务"或"接受 Git signed-off-by + tag 签名"。两个都不具备时的降级路径？[Assumption, Spec §Assumptions]
  - ❌ 降级路径未定义; 当前实现使用 GPG tag 签名, 假设组织具备 Git 签名能力
- [x] CHK030 — 假设"深度测试工具链可以只在 CI 夜间队列执行"。夜间队列未完成时的发布规则？[Assumption, Spec §Assumptions]
  - ❌ 夜间队列未完成时的规则未定义; `quality-gate-outcome.csv` 允许引用最近一次归档指针——如果归档不存在,发布被阻断

## Ambiguities & Conflicts(歧义与冲突)

- [x] CHK031 — spec 使用"浅层门禁""中层门禁""深度门禁"等多个层级。三者的包含关系和执行顺序是否统一定义？[Ambiguity, Spec §SC-001 vs §FR-002 vs §FR-003]
  - `.github/workflows/shallow-gates.yml`(每次 PR), `middle-gates.yml`(每次发布), nightly-gates.yml(夜间深度); 执行顺序和包含关系已在 CI 配置中定义 ✓
- [x] CHK032 — Key Entities 中 QualityGateOutcome 的 5 类取值与 FR-003 的 incomplete 标记之间的关系。incomplete = missing? [Ambiguity, Spec §Key Entities vs FR-003]
  - ❌ incomplete 与 missing 的关系未定义; `quality-gate-outcome.csv` 使用空字符串表示未执行
- [x] CHK033 — semver-checks 发现破坏性变更但发布者本意是 MAJOR 抬升——预期 vs 意外的区分标准？[Ambiguity, Spec §SC-001]
  - ❌ 区分标准未定义; 人工在 changelog 和 release-record.json 的 semver_level 字段中声明

## Constitution Compliance(宪章合规)

- [x] CHK034 — Module ownership 要求"CI 描述文件与发布脚本只能存放在 tools/ 或 .github/workflows/ 路径树下"。当前 scripts/ 在使用——是否需要更新 Constitution？[Compliance, Spec §Module ownership]
  - Constitution 允许 tools/ 或 .github/workflows/; 当前使用 scripts/ + .github/——scripts/ 不在允许列表中, 需更新 Constitution 或移动脚本 🔶
- [x] CHK035 — Diagnostics 要求"任一闸口脚本返回失败时必须打印稳定的 gate_id 字符串"。gate_id 命名规范是否定义？[Compliance, Spec §Diagnostics]
  - `quality-gate-outcome.csv` 的 gate_id 列使用 snake_case(如 dependency_audit, semver_checks); 命名规范已在 CSV 中隐含定义 ✓
- [x] CHK036 — Constitution 要求"变更必须经过与普通源码同等力度的评审标签"。发布脚本的 PR 评审要求是否定义？[Compliance, Spec §Module ownership]
  - ❌ 具体评审规则未定义; GitHub 分支保护规则要求 PR 评审,但 spec 未引用

## Notes(说明)

- 完成检查项后使用 `[x]` 标记.
- 评论或发现可以直接写在相关检查项下.
- 需要时链接相关资源或文档.
- 检查项必须按顺序编号, 方便引用.
