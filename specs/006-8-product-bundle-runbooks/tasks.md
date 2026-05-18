---
description: "Task list for product bundle and runbooks"
---

# Tasks(任务): 最小生产包, 交付文档与放行矩阵占位

**Input(输入)**: 设计文档来自 `specs/006-8-product-bundle-runbooks/`
**Prerequisites(前置文档)**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests(测试)**: 本切片不修改 `src/` 生产代码. 新增的校验脚本通过 shell 测试验证. 手册审查通过人工 review 验证.

**Organization(组织方式)**: 任务按 spec 的三个用户故事分组.

---

## Phase 1(阶段一): Setup(基础设施)

**Purpose(目的)**: 创建校验脚本和格式契约.

- [x] T001 在 `scripts/check-tarball-content.sh` 中实现 tarball 内容校验脚本: 检查 `src/`, `examples/`, `manual/` 目录存在; 检查 `Cargo.toml` 依赖无本地绝对路径; 检查无私服 registry 引用. 对所有检查项逐个断言, 任一失败退出码非 0.
- [x] T002 在 `scripts/validate-release-matrix.sh` 中实现放行矩阵格式校验脚本: 读取 `artifacts/quality-gate-outcome.csv`, 验证列数, 验证 `outcome` 字段只允许五类枚举, 验证空白 `exemption_ticket` + `outcome != "passed"` 时标记为 blocking. 退出码非 0 表示格式不通过.
- [x] T003 确认 `cargo package --list` 输出包含 src/, examples/, Cargo.toml, 且无意外包含的文件.

**Checkpoint(检查点)**: 校验脚本就绪, tarball 内容可自动化验证.

---

## Phase 2(阶段二): User Story 1(用户故事一) - MVP 包可被照抄拉起 (Priority: P1)

**Goal(目标)**: 部署指南承诺步数上限, 健康自检输出稳定 JSON schema.

- [x] T004 [US1] 审查 `manual/en/getting-started.md` 和 `manual/zh/getting-started.md`, 在每个步骤标题末尾追加 `(Step X of Y)` 计数. 文档顶部注明总步数 Y=5.
- [x] T005 [US1] 在 `manual/en/getting-started.md` 和 `manual/zh/getting-started.md` 末尾新增"健康自检"章节, 引用 `contracts/health-selfcheck-schema.md` 并给出 JSON 输出示例.
- [x] T006 [US1] 在 `manual/en/deployment-guide.md` 和 `manual/zh/deployment-guide.md` 中补充密钥占位符段落: 写明 `${SECRET_NAME}` 格式及其替换规则. 格式与 006-6 一致.
- [x] T007 [US1] 在 `manual/en/deployment-guide.md` 末尾增加"升级"章节占位, 写明"本版本不支持原地升级, 需要全新部署".
- [x] T008 [US1] 确认 `cargo run --example supervisor_quickstart` 输出的 JSON 符合 `contracts/health-selfcheck-schema.md`.

**Checkpoint(检查点)**: 部署指南步数上限明确, 健康自检 JSON schema 正式定义并引用.

---

## Phase 3(阶段三): User Story 2(用户故事二) - 值守手册可执行 (Priority: P1)

**Goal(目标)**: 值守手册每一步都写明期望 metrics 字段取值, 无悬空引用.

- [x] T009 [US2] 审查 `manual/en/operations-runbook.md` 和 `manual/zh/operations-runbook.md`, 为每个 P1 步骤补充 `Expected metrics:` 段落, 写明期望的字段名和取值(如 `status == "ready"`).
- [x] T010 [US2] 在 `manual/en/operations-runbook.md` 和 `manual/zh/operations-runbook.md` 中修复悬空引用: 使用 mdBook build 后 grep 检查 `href="#` 锚点是否存在对应 `id` 定义. 所有悬空引用替换为有效锚点或移除.
- [x] T011 [US2] 在 `manual/en/operations-runbook.md` 和 `manual/zh/operations-runbook.md` 每步末尾补充预计耗时(如 `Estimated: 5min`).
- [x] T012 [US2] 在 CI 配置(或 `scripts/` 新增脚本)中增加 `mdbook build` 后的锚点有效性检查.

**Checkpoint(检查点)**: 值守手册每一步可执行, 无悬空引用, 耗时标注完整.

---

## Phase 4(阶段四): User Story 3(用户故事三) - 放行矩阵随版本并排发布 (Priority: P2)

**Goal(目标)**: 放行矩阵空白 td 计数为 0, 006-2 和 006-7 归档哈希并排.

- [x] T013 [US3] 审查 `artifacts/quality-gate-outcome.csv`, 确认列数和枚举值合法. 补充 chaos-test, soak-24h 对应的 archive_ref 列为 006-7 路径.
- [x] T014 [US3] 在 `scripts/validate-release-matrix.sh` 中增加 CSV 到 HTML 的转换逻辑(使用 shell + awk 或简单替换). 生成的 HTML `<table>` 中空白 `<td></td>` 计数为 0.
- [x] T015 [US3] 确认放行矩阵中 006-2 ReleaseRecord 和 006-7 SoakReport 的 archive_ref 列同时存在且可解析.

**Checkpoint(检查点)**: 放行矩阵格式契约满足, 空白 td 检查通过.

---

## Phase 5(阶段五): Polish(收尾与交叉关注点)

**Purpose(目的)**: 补齐覆盖率, CI 集成, 最终验证.

- [x] T016 确认 `cargo test` 无回归.
- [x] T017 确认 `scripts/check-tarball-content.sh` 在 CI 环境中通过.
- [x] T018 确认 `scripts/validate-release-matrix.sh` 在 CI 环境中通过.
- [x] T019 确认 `mdbook build` 在 CI 环境中无错误.
- [x] T020 最终审查: 所有手册补充段落无中英文不一致, 术语格式统一.

**Checkpoint(检查点)**: 全部 20 个任务完成, 生产包构建与发布流程可重复验证.
