# Tasks(任务): 工业级发布门禁与供应链证明

**Input(输入)**: 设计文档来自 `specs/006-2-release-supply-chain-gates/`
**Prerequisites(前置文档)**: plan.md(必需), spec.md(用户故事必需), research.md, data-model.md, contracts/release-gates.md, quickstart.md

**Tests(测试)**: 本切片为 CI/CD 工程, 无 Rust 行为变化. 验收以模拟发布抽查执行门禁脚本并核对台账.

**Organization(组织方式)**: 任务按用户故事分组, 每个故事可独立实现和独立验收.

## Format(格式): `[ID] [P?] [Story] Description(描述)`

- **[P]**: 可以并行执行, 因为任务修改不同文件, 并且不依赖未完成任务.
- **[Story]**: 标记任务属于哪个用户故事, 例如 US1, US2, US3.
- 任务描述写出准确文件路径.

## Path Conventions(路径约定)

- CI workflow(工作流) 文件: `.github/workflows/`
- 校验与发布脚本: `scripts/`
- 台账与模板: `artifacts/`
- 策略配置文件: 仓库根目录 (`deny.toml` 等)

---

## Phase 1: Setup(项目初始化)

> 创建本切片需要的目录结构与空模板文件.

- [x] T001 创建目录 `.github/workflows/` (如不存在) 和 `scripts/` (如不存在)
- [x] T002 [P] 创建空台账模板 `artifacts/release-record.json` 按 data-model.md ReleaseRecord 结构, 初始字段为空占位符
- [x] T003 [P] 创建空台账模板 `artifacts/quality-gate-outcome.csv` 按 data-model.md QualityGateOutcome 列定义, 含 CSV 表头
- [x] T004 [P] 创建豁免工单模板 `artifacts/exemption-ticket.md` 按 data-model.md ExemptionTicket 模板

---

## Phase 2: Foundational(基础层)

> 所有用户故事都依赖的基础设施. **必须在本阶段完成后才能开始任何用户故事.**

- [x] T005 创建 `deny.toml` 配置文件: 写入 `[licenses]` (允许 MIT, Apache-2.0, BSD-3-Clause), `[advisories]` (vulnerability=deny, unmaintained=warn), `[bans]` (初始为空), 按 research.md 中层门禁要求
- [x] T006 [P] 创建浅层门禁 CI workflow `.github/workflows/shallow-gates.yml`: 依次执行 `cargo fmt --check`, `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo doc --no-deps --document-private-items`, `cargo publish --dry-run`, 任一步失败则整体失败, 按 contracts/release-gates.md shallow 节
- [x] T007 [P] 创建发布前置全量检查入口脚本 `scripts/release-check.sh`: 依次调用浅层门禁各命令, 打印每项 `gate_id` 与 `passed/failed`, 退出码汇总, 按 quickstart.md 步骤 2

---

## Phase 3: User Story 1(用户故事一) — 版本可追溯 (Priority(优先级): P1)

**Goal(目标)**: 每次对外候选版本留下 signed tag(签名标签) 指针, changelog(变更日志) 条目, semver(语义化版本) 等级, MSRV(最低 Rust 版本) 自检脚本. 采购方离线可核对.

**Independent Test(独立测试)**: 从发布台账任取相邻两版本号, 只用四类指针做差异口述, 不检出源码树即能说出用户可见风险摘要.

### Implementation(实现)

- [x] T008 [US1] 创建 MSRV(最低 Rust 版本) 自检脚本 `scripts/verify-msrv.sh`: 读取 `Cargo.toml` 中 `rust-version` 字段, 用该版本 rustc 编译 `cargo check`, 失败时在固定 5 步以内退出非零状态并打印文档章节号, 按 spec.md US1 验收场景 2 与 contracts/release-gates.md msrv_verify 节
- [x] T009 [US1] 创建发布台账填充指南: 在 `quickstart.md` 步骤 4 中补充 `release-record.json` 逐字段填写说明 (version, semver_level, commit, signed_tag, msrv, changelog), 按 spec.md FR-001
- [x] T010 [P] [US1] 在 `CHANGELOG.md` 中添加 "Unreleased" 小节模板: 含 `## [Unreleased]`, `### Added`, `### Changed`, `### Fixed`, `### Security Notes` 子标题以及 semver(语义化版本) 等级标注位置; PATCH(补丁级别) 版本如改动高风险示例命令行须在 `### Security Notes` 单独列出, 按 spec.md FR-001 与 Edge Case 1
- [x] T011 [P] [US1] 创建 signed tag(签名标签) 操作说明: 在 `quickstart.md` 步骤 3 中补充 `git tag -s` 命令, 验签命令 `git tag -v`, 以及台账 `signed_tag` 字段填写方法, 按 research.md 第二节

---

## Phase 4: User Story 2(用户故事二) — 供应链与合规闸口有据可查 (Priority(优先级): P2)

**Goal(目标)**: 发布流水线固定出现 dependency audit(依赖审计) 摘要, 许可证判定, CVE(公开漏洞编号) 封锁, SBOM(软件物料清单) 文件指针, supply chain attestation(供应链证明) 摘要哈希.

**Independent Test(独立测试)**: 用买方 SBOM(软件物料清单) 消费工具对归档文件跑校验, 输出哈希与 ReleaseRecord(发布记录) 登记值一致.

### Implementation(实现)

- [x] T012 [US2] 创建中层门禁 CI workflow `.github/workflows/middle-gates.yml`: 依次执行 `cargo audit --deny warnings`, `cargo deny check licenses`, `cargo deny check advisories`, `cargo semver-checks`, `bash scripts/verify-msrv.sh`; 任一步失败则工作流退出非零, 阻断发布 (除非该行在 `quality-gate-outcome.csv` 中已写入有效 `exemption_ticket` 编号); 阻断时发布台账 `gates.middle.*` 写 `"failed"`, `released_at` 留空; 豁免时写 `"waived (EX-YYYY-NNN)"`; 按 contracts/release-gates.md middle 节与 Blocking Gate Audit 节
- [x] T013 [P] [US2] 创建 SBOM(软件物料清单) 外部复验脚本 `scripts/verify-sbom.sh`: 从 `release-record.json` 读取 `sbom.path`, `sbom.sha256`, `sbom.format_version`; 计算实际文件哈希并比对; 若 `format_version` 与当前工具产出不一致则打印告警并提示查阅 `sbom-migration.md`; 按 spec.md US2 验收场景 2, SC-002 与 Edge Case 2
- [x] T014 [P] [US2] 创建供应链证明生成脚本 `scripts/generate-attestation.sh`: 收集 version, commit, timestamp, 各闸门结果摘要, 产物文件 sha256, 输出 JSON 到 `artifacts/attestation.json`; 若 attestation(供应链证明) 生成过程中任一产物路径不可达或哈希计算失败, 输出 `attestation_unavailable` 状态并写 `artifacts/attestation-error.log`, 发布台账标记为 `blocked`; 按 research.md 第三节 attestation 结构与 spec.md Edge Case 3
- [x] T015 [P] [US2] 创建供应链证明外部复验脚本 `scripts/verify-attestation.sh`: 读取 `release-record.json` 中 `supply_chain_attestation.path` 与 sha256, 重算各产物哈希, 逐项比对, 输出 `MATCH`/`MISMATCH` 明细, 按 spec.md SC-002
- [x] T016 [US2] 在 `scripts/release-check.sh` 中追加中层门禁调用: 依次调用 `cargo audit`, `cargo deny check licenses`, `cargo deny check advisories`, `cargo semver-checks`, `bash scripts/verify-msrv.sh`, 打印每项结果, 按 quickstart.md 步骤 2

---

## Phase 5: User Story 3(用户故事三) — 深度质量矩阵写进放行记录 (Priority(优先级): P3)

**Goal(目标)**: 发布放行表为接口兼容性, 变异测试, 覆盖率, fuzzing(模糊测试), loom(并发模型测试), miri(未定义行为检查) 各留独立槽位. 未执行时填豁免工单编号.

**Independent Test(独立测试)**: 打开 QualityGateOutcome(质量闸口结果) CSV, 断言空单元格数为 0 或每空同行有 ExemptionTicket(豁免工单) 编号.

### Implementation(实现)

- [x] T017 [US3] 创建深层门禁 CI workflow `.github/workflows/nightly-gates.yml`: 用 `rustup default nightly` 工具链, 依次执行 `cargo tarpaulin --out json`, `cargo mutants`, `cargo fuzz run`, `cargo test --test loom_*`, `cargo miri test`, 每项输出 `gate_id` 与退出码; 失败不阻断 PR, 但发布台账需引用本 workflow 归档 URL, 按 contracts/release-gates.md deep 节与 research.md 第四节
- [x] T018 [P] [US3] 创建 `quality-gate-outcome.csv` 填充脚本 `scripts/fill-quality-gate.sh`: 从各门禁日志中提取结论并填入 CSV 对应行; 对 `missing` 行检查是否同行有豁免编号, 按 spec.md FR-003 与 SC-003
- [x] T019 [P] [US3] 在 `scripts/release-check.sh` 中追加深度门禁摘要查询: 检查 `quality-gate-outcome.csv` 中 deep 层级各行, 若 `outcome` 为 `missing` 且 `exemption_ticket` 为空则打印告警, 按 spec.md US3 验收场景 2

---

## Phase 6: Polish(收尾)

> 跨切片的收尾与验证.

- [x] T020 更新 `AGENTS.md` 中 SPECKIT 上下文: 确认功能路径指向 `specs/006-2-release-supply-chain-gates/plan.md`, 文档列表中补充 `tasks.md`
- [x] T021 运行全量模拟发布验证: `bash scripts/release-check.sh`, 确认所有浅层与中层门禁通过或正确报告失败; 手动执行 `bash scripts/verify-sbom.sh` 与 `bash scripts/verify-attestation.sh` 验证脚本语法正确

---

## Dependencies(依赖关系)

```
Phase 1 (Setup)
  │
  ▼
Phase 2 (Foundational)  ←── 所有用户故事的前置
  │
  ├──▶ Phase 3 (US1: P1 版本可追溯)  ← 可并行
  ├──▶ Phase 4 (US2: P2 供应链合规)  ← 可并行
  │
  └──▶ Phase 5 (US3: P3 深度质量矩阵) ← 依赖 Phase 4 中层门禁
  │
  ▼
Phase 6 (Polish 收尾)
```

## Parallel Execution(并行执行示例)

### Phase 2 内并行

```bash
Task T005: "创建 deny.toml"
Task T006: "创建 .github/workflows/shallow-gates.yml"
Task T007: "创建 scripts/release-check.sh"
```

### US1, US2, US3 并行

```bash
# Phase 2 完成后, US1 和 US2 可同时推进:
Task T008-T011: "US1 版本可追溯"
Task T012-T016: "US2 供应链合规"

# US3 在 US2 的中层门禁脚本建立后开始:
Task T017-T019: "US3 深度质量矩阵"
```

### US2 内并行

```bash
Task T013: "scripts/verify-sbom.sh"
Task T014: "scripts/generate-attestation.sh"
Task T015: "scripts/verify-attestation.sh"
```

## Implementation Strategy(实现策略)

### MVP(最小可行产品): User Story 1 + 2 Only(仅用户故事一和二)

1. 完成 Phase 1 (Setup) + Phase 2 (Foundational)
2. 完成 Phase 3 (US1): 版本可追溯 → 采购方能验签验版本
3. 完成 Phase 4 (US2): 供应链合规 → 安全专员能验 SBOM
4. 交付: 浅层 + 中层门禁完整可用, 发布台账可被外部复验

### Incremental Delivery(增量交付)

1. **Iteration 1(迭代一)**: Phase 1 + Phase 2 (基础层 + deny.toml + 浅层 CI)
2. **Iteration 2(迭代二)**: Phase 3 (US1: 版本追溯) → 能签署标签, 校验 MSRV, 写 changelog
3. **Iteration 3(迭代三)**: Phase 4 (US2: 供应链合规) → 能跑依赖审计, 生成/验 SBOM, 生成/验证明
4. **Iteration 4(迭代四)**: Phase 5 (US3: 深度质量矩阵) → 夜间 deep 门禁, 豁免工单
5. **Iteration 5(迭代五)**: Phase 6 (收尾验证)

## Summary(摘要)

| Metric(指标)                      | Value(值)                                                |
| --------------------------------- | -------------------------------------------------------- |
| Total tasks(任务总数)             | 21                                                       |
| US1 tasks(用户故事一)             | 4 (T008-T011)                                            |
| US2 tasks(用户故事二)             | 5 (T012-T016)                                            |
| US3 tasks(用户故事三)             | 3 (T017-T019)                                            |
| Setup + Foundational(初始化+基础) | 7 (T001-T007)                                            |
| Polish(收尾)                      | 2 (T020-T021)                                            |
| Parallel opportunities(可并行)    | T002-T004, T006-T007, T010-T011, T013-T015, T018-T019    |
| Independent test(独立测试)        | US1: 版本指针口述, US2: SBOM 复验, US3: CSV 空单元格断言 |
| Suggested MVP(最小可行产品)       | Phase 1-4 (US1+US2, 16 任务)                             |
