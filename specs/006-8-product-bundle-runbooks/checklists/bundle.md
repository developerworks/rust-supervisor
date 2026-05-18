# Product Bundle Requirements Quality Checklist(生产包与交付文档需求质量检查清单)

**Purpose(目的)**: 验证 `006-8-product-bundle-runbooks` 功能规格中 MVP 生产包内容、值守手册可执行性和放行矩阵指针的需求质量、完整性与可度量性。

**Created(创建日期)**: 2026-05-19
**Scope(范围)**: US1(MVP 包可拉起) + US2(值守手册可执行) + US3(放行矩阵并排), 全部 3 个用户故事
**Depth(深度)**: Standard(标准)
**Audience(受众)**: Reviewer(PR 审查) + Release(发布责任人)
**Gates(关口)**: 部署盲测 ≥95%, P1 演练 ≥90%, 放行矩阵空白 td = 0

---

## Requirement Completeness(需求完整性)

- [x] CHK001 — FR-001 要求 MVP tarball 至少捆绑 4 类构件。每类的选取/排除标准是否定义？[Completeness, Spec §FR-001]
  - 4 类列出 ✓; 选取标准由 Cargo.toml include 规则定义; examples/ 目录包含 9 个示例; manual/ 包含手册——选取标准已隐含实现 ✓
- [x] CHK002 — FR-002 要求 deployment guide 和 operations runbook"必须同步抬升 semver"。版本号与 crate 版本号的关系？[Completeness, Spec §FR-002]
  - manual/book.toml 可独立配置版本号; 与 crate 版本号的关系未定义 ❌
- [x] CHK003 — FR-003 要求正式发布附带 10 项测试(含"不少于 24h 浸泡")。浸泡执行时长和执行频率是否定义？[Completeness, Spec §FR-003]
  - 006-7 定义 24h 浸泡; 执行频率=每次发布(见 FR-003) ✓
- [x] CHK004 — US1 要求 deployment guide"固定步数上限内必须打印 ready"。步数上限和 ready 判定标准是否定义？[Completeness, Spec §US1]
  - ❌ 步数上限和 ready 判定标准未定义; manual/en/getting-started.md 有步骤但未承诺步数上限
- [x] CHK005 — US2 要求"每一步末尾都必须写明期望 metrics 字段取值"。取值格式是否定义？[Completeness, Spec §US2]
  - ❌ 取值格式未定义; manual/en/ 文档中 metrics 字段以自然语言描述, 未标准化
- [x] CHK006 — Edge Cases 要求"自检脚本必须把'缺证书链'与'产品缺陷'区分枚举"。枚举值是否定义？[Completeness, Spec §Edge Cases]
  - ❌ 枚举值未定义; 实现中 DashboardError 区分 peer_cred_unavailable 和其他错误——间接满足区分要求 🔶

## Requirement Clarity(需求清晰度)

- [x] CHK007 — FR-001 要求"tarball 内禁止引用未公开的私服 registry"。vendor 方式是否豁免？[Clarity, Spec §FR-001]
  - ❌ vendor 方式是否豁免未明确; 当前无 vendor 目录, 所有依赖通过 crates.io 或公开 git 仓库引用 ✓
- [x] CHK008 — FR-002 要求"密钥引用占位"。密钥占位符格式是否在跨切片中统一定义？[Clarity, Spec §FR-002]
  - 006-6 使用 `${SECRET_NAME}` 格式; 本切片未定格式——应与 006-6 对齐 🔶
- [x] CHK009 — US1 验收场景要求"打印 ready 与最小看板链路自检字段"。自检字段的 JSON schema 是否定义？[Clarity, Spec §US1]
  - ❌ 自检字段 schema 未定义; manual/en/getting-started.md 描述健康检查但无标准化 JSON 结构
- [x] CHK010 — US2 验收场景要求"不允许出现悬空引用"。悬空引用的检测方法是否定义？[Clarity, Spec §US2]
  - ❌ 悬空引用检测方法(CI 检查? 人工?)未定义
- [x] CHK011 — SC-002 要求"至少 90% 条目在手册写明的时间上限内抵达终态分叉"。P1 条目总数是否定义？[Clarity, Spec §SC-002]
  - ❌ P1 条目总数未定义; 影响 90% 的实际严格程度

## Requirement Consistency(需求一致性)

- [x] CHK012 — FR-003 的 10 项测试与 006-7 的 11 个 ChaosScenario 之间的映射关系是否定义？[Consistency, Spec §FR-003 vs specs/006-7]
  - ❌ 映射关系未定义; FR-003 的"chaos test"对应 006-7 的 11 个 scenario——粒度不一致
- [x] CHK013 — FR-003 要求深度测试证据哈希与 006-7 一致。非 006-7 测试的证据哈希来源？[Consistency, Spec §FR-003 vs specs/006-7 vs specs/006-2]
  - `release-record.json` 的 sbom.sha256 和 attestation SHA-256 作为证据; 非 006-7 测试(如单元测试)的证据哈希由 006-2 的 ReleaseRecord 覆盖 ✓
- [x] CHK014 — FR-001 要求 tarball 包含 docker-compose, Assumptions 提到 FINAL_REPORT 外链。两个交付物的交叉引用是否定义？[Consistency, Spec §FR-001 vs Assumptions]
  - ❌ 交叉引用未定义; FINAL_REPORT.md 存在于仓库根目录但 deployment guide 未引用

## Acceptance Criteria Quality(验收标准可度量性)

- [x] CHK015 — SC-001 要求"部署盲测通过率不低于 95%, 样本不少于 10 套容器镜像"。镜像的操作系统分布和网络条件是否定义？[Measurability, Spec §SC-001]
  - ❌ 镜像分布和网络条件未定义
- [x] CHK016 — SC-002 要求"至少 90% 条目在手册写明的时间上限内抵达终态分叉"。时间上限和计时起点是否定义？[Measurability, Spec §SC-002]
  - US2 提到"15 分钟滑动窗口" ✓; SC-002 未显式引用该数值; 实现中无自动计时, 桌面演练人工记录 🔶
- [x] CHK017 — SC-003 要求"对外放行矩阵 DOM 裸露空白 td 计数恒为 0"。DOM 解析工具和空白定义是否定义？[Measurability, Spec §SC-003]
  - ❌ 解析工具和空白 td 定义未定义; 当前放行矩阵为 CSV 格式, 非 DOM

## Scenario Coverage(场景覆盖)

- [x] CHK018 — US1 覆盖了"从 tarball 拉起"场景。升级场景是否在 deployment guide 范围内？[Coverage, Spec §US1]
  - ❌ 升级场景不在 US1 范围内——缺少显式的"超出范围"标注
- [x] CHK019 — US2 覆盖了 P1 事故值守。演练频率和失败后的改进行动是否定义？[Coverage, Spec §US2]
  - ❌ 演练频率和改进行动未定义
- [x] CHK020 — US3 覆盖了放行矩阵的呈现。跳过项的呈现规则是否定义？[Coverage, Spec §US3]
  - ❌ 跳过项的呈现规则未定义; quality-gate-outcome.csv 用空字符串 + exemption_ticket 表示跳过

## Edge Case Coverage(边界条件覆盖)

- [x] CHK021 — Edge Cases 要求"参考命令所需内核能力必须与 006-1 支持矩阵一致"。CI 检测机制是否定义？[Edge Case, Spec §Edge Cases]
  - ❌ CI 检测机制未定义; 人工评审确保一致性
- [x] CHK022 — Edge Cases 要求"mTLS 证书链责任分割线必须在 operations runbook 写清"。自签名 CA 的自检步骤是否定义？[Edge Case, Spec §Edge Cases]
  - ❌ 自签名 CA 的自检步骤未定义; manual/en/ 文档未覆盖
- [x] CHK023 — FR-003 要求放行矩阵包含"不少于 24h 浸泡"。浸泡报告的可复用策略(有效期)是否定义？[Edge Case, Spec §FR-003]
  - ❌ 浸泡报告有效期未定义; 006-7 假设每次发布前执行浸泡

## Non-Functional Requirements(非功能需求)

- [x] CHK024 — MVP tarball 的压缩后大小预算是否定义？[NFR, Gap]
  - ❌ 大小预算未定义; `cargo package --list` 可估算——建议后续加入 spec
- [x] CHK025 — deployment guide 和 operations runbook 的格式和访问方式是否定义？[NFR, Gap]
  - manual/ 使用 mdBook 生成 HTML ✓; 格式已隐含为 Markdown + mdBook,但 spec 未锁定 🔶

## Dependencies & Assumptions(依赖与假设)

- [x] CHK026 — spec 强依赖 006-2 和 006-7 的归档路径。归档路径约定是否在本切片中锁定？[Dependency, Spec §Dependency Note]
  - Dependency Note 要求"发布台账必须把 006-2 与 006-7 归档哈希并排挂上" ✓; 路径约定为 artifacts/release-record.json 和 artifacts/soak-report/,已在仓库中实现 ✓
- [x] CHK027 — 假设"对内详尽 FINAL REPORT 可以通过外链摘要映射到外发精简页面"。防火墙阻断时的离线替代方案？[Assumption, Spec §Assumptions]
  - ❌ 离线替代方案未定义; FINAL_REPORT.md 可在仓库内离线访问 ✓
- [x] CHK028 — FR-003 要求供应链指针与 006-2 ReleaseRecord 一致。006-2 的 ReleaseRecord 格式是否已冻结？[Dependency, Spec §FR-003 → specs/006-2]
  - `artifacts/release-record.json` 已在仓库中存在, schema 已冻结 ✓

## Ambiguities & Conflicts(歧义与冲突)

- [x] CHK029 — US3 验收场景要求比对两列哈希。006-2 和 006-7 使用相同的哈希算法？data-archive-sha256 属性名是否需要区分来源？[Ambiguity, Spec §US3 vs specs/006-2 vs specs/006-7]
  - 两者均使用 SHA-256 ✓; data-archive-sha256 属性名未区分来源, 但 JSON schema 中各自有独立字段名(sbom.sha256 vs attestation.sha256) ✓
- [x] CHK030 — Key Entities 中 DeliveryBundle 的 sha256 是指每个构件的独立哈希还是整个 tarball 的哈希？[Ambiguity, Spec §Key Entities]
  - ❌ 哈希粒度未定义; release-record.json 使用文件级 SHA-256, 非 tarball 级
- [x] CHK031 — SC-001 要求"95% 部署盲测通过率"。基础设施失败 vs 产品缺陷的区分规则？[Ambiguity, Spec §SC-001]
  - ❌ 区分规则未定义

## Constitution Compliance(宪章合规)

- [x] CHK032 — Module ownership 要求"examples 与参考服务代码目录固定在仓库约定路径"。当前目录是否符合？[Compliance, Spec §Module ownership]
  - examples/ 和 manual/ 已存在 ✓; 各目录用途在 plan.md 中定义 ✓
- [x] CHK033 — Diagnostics 要求"健康自检脚本 stdout JSON 必须具备稳定顶层键名"。键名清单是否定义？[Compliance, Spec §Diagnostics]
  - ❌ 键名清单未定义; 健康自检输出格式待后续固化
- [x] CHK034 — Constitution 要求"tarball 裁剪与支持矩阵必须同窗修订"。CI 验证机制是否定义？[Compliance, Spec §Constitution]
  - ❌ CI 验证机制未定义

## Notes(说明)

- 完成检查项后使用 `[x]` 标记.
- 评论或发现可以直接写在相关检查项下.
- 需要时链接相关资源或文档.
- 检查项必须按顺序编号, 方便引用.
