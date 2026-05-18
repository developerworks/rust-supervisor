# Research(研究): 最小生产包缺口分析与技术方案

**Branch(分支)**: `main` | **Date(日期)**: 2026-05-19 | **Spec(规格)**: `specs/006-8-product-bundle-runbooks/spec.md`

## 1. 研究范围

本文件是 Phase 0(研究阶段) 输出, 解决 plan.md 和 checklist 中标识的缺口, 记录对已有 examples, manual, artifacts 的审查结论.

## 2. 缺口审查结论

### 2.1 部署指南步数上限(CHK004)

**结论**: 部署指南(manual/en/getting-started.md)当前以自然语言描述步骤, 未承诺步数上限. 需要补充"步数上限"声明.

**具体缺口**: manual/en/getting-started.md 和 manual/zh/getting-started.md 的步骤末尾未标注 `(Step N of M)` 计数. 补充格式: 每个步骤标题末尾追加 `(Step X of Y)`, 文档顶部注明 Y 值.

**影响范围**: 仅 manual/en/getting-started.md 和 manual/zh/getting-started.md.

### 2.2 健康自检 JSON schema(CHK009)

**结论**: 当前健康自检输出无正式 JSON schema. 需要定义稳定顶层键名并在手册引用.

**建议键名**:

- `status`: `"ready" | "degraded" | "failed"`
- `supervisor_version`: semver 字符串
- `uptime_secs`: u64
- `children`: `{ total: u64, running: u64, failed: u64 }`
- `dashboard_link`: `"connected" | "disconnected" | Option<string>`

**影响范围**: 新增 `contracts/health-selfcheck-schema.md`; manual 相关章节引用该 schema.

### 2.3 密钥占位符格式(CHK008)

**结论**: 006-6 使用 `${SECRET_NAME}` 格式. 本切片应与 006-6 对齐.

**推荐格式**: `${SECRET_NAME}` 或 `${ENV_VAR_NAME}`. 在 deployment guide 中写明占位符替换规则.

**影响范围**: manual/deployment-guide.md 补充密钥占位符段落.

### 2.4 悬空引用检测(CHK010)

**结论**: 手册中可能存在指向不存在章节的锚点引用.

**检测方法**: 使用 mdBook 的 `mdbook test`(链接检查) + 新增 shell 脚本站内引用解析. 在 CI 中 `mdbook build` 后 grep 检查 `href="#` 锚点是否存在对应 `id="` 或 `name="` 的定义.

**影响范围**: 新增 CI 检查步骤.

### 2.5 放行矩阵格式(CHK017)

**结论**: 当前放行矩阵为 CSV 格式(`artifacts/quality-gate-outcome.csv`). US3 要求 DOM 空白 td 检查.

**方案**: 维护 CSV 作为权威数据源, 发布页面渲染时由 CI 脚本将 CSV 转换为 HTML 表格. 空白 td 检查在 CI 中对生成的 HTML 执行.

**影响范围**: 新增 `contracts/release-matrix-format.md`; 新增 `scripts/validate-release-matrix.sh`.

### 2.6 tarball 内容校验(CHK024)

**结论**: FR-001 要求 MVP tarball 至少包含 4 类构件, 且禁止私服引用.

**校验内容**:

1. `src/` 目录存在非空
2. `examples/` 目录存在非空
3. `manual/` 或 docs/ 目录存在
4. `Cargo.toml` 中 `[dependencies]` 全部指向 crates.io 或公开 git 仓库
5. 无 `path =` 指向本地绝对路径的依赖

**影响范围**: 新增 `scripts/check-tarball-content.sh`.

### 2.7 升级场景覆盖(CHK018)

**结论**: US1 仅覆盖从 tarball 首次拉起, 升级场景不在范围内. 需要在 spec Edge Cases 或 manual 中标注"超出范围".

**标注位置**: deployment guide 末尾增加"升级"章节占位, 写明本版本不支持原地升级, 需要全新部署.

### 2.8 CI 验证机制(CHK021, CHK034)

**结论**: 当前无 CI 自动化验证手册与 006-1 支持矩阵的一致性, 也无 tarball 裁剪与支持矩阵同窗修订的 CI 检查.

**方案**: 在 `scripts/check-tarball-content.sh` 中增加对 006-1 支持矩阵的交叉引用检查(至少检查 examples 中使用的 IPC 能力是否在支持矩阵中声明).

## 3. 需要澄清的问题(全部已解决)

1. ~~部署指南步数上限具体值?~~ -> 5 步(从解压 tarball 到打印 ready JSON)
2. ~~密钥占位符格式?~~ -> 与 006-6 对齐: `${SECRET_NAME}`
3. ~~放行矩阵空白 td 检查工具?~~ -> `scripts/validate-release-matrix.sh` 使用 shell + grep, 不新增依赖

## 4. 备选方案与决策理由

| 备选方案                                | 拒绝理由                                 |
| --------------------------------------- | ---------------------------------------- |
| 重写 manual/ 全文                       | 代价过高, 现有 manual 内容可用, 只需补缺 |
| 使用 Python/jsonschema 做自检 JSON 校验 | shell + grep 已满足, 不引入新语言依赖    |
| 放行矩阵使用 JSON 而非 CSV              | 006-2 已定义 CSV 格式, 保持一致性        |
