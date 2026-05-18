# Implementation Plan(实现计划): 最小生产包, 交付文档与放行矩阵占位

**Branch(分支)**: `main` | **Date(日期)**: 2026-05-19 | **Spec(规格)**: `specs/006-8-product-bundle-runbooks/spec.md`
**Input(输入)**: 功能规格来自 `specs/006-8-product-bundle-runbooks/spec.md`

**Note(说明)**: 本文件由 `/speckit-plan` 命令生成, 基于 `.specify/templates/plan-template.md` 模板.

## Summary(摘要)

本切片在已有监督器核心 crate, examples, manual 目录基础上, 完成 MVP 生产包对外交付的三个用户故事: (1) 最小可用生产包可被照抄拉起, (2) 值守手册可执行, (3) 放行矩阵随版本并排发布. 核心设计决策如下:

1. **不修改 src/ 生产代码**: 本切片约束交付物(tarball, 手册, 放行矩阵), 不改变监督运行时.
2. **存量复用**: examples/ 目录已有 9 个示例, manual/ 目录已有部署指南和值守手册, artifacts/ 已有 release-record.json 和 quality-gate-outcome.csv. 本切片对这些已有文件做补缺审查和格式固化, 不重写.
3. **外链集成**: 放行矩阵指针指向 006-2 ReleaseRecord 和 006-7 SoakReport, 本切片不重复归档.
4. **Checklist 缺口修复**: spec 的 checklist(CHK001-CHK034) 标识了若干缺口(如步数上限未定义, 自检 JSON schema 未定义, 密钥占位符格式未对齐等), 本 plan 通过 research.md 和 tasks.md 逐一填补.

## Technical Context(技术背景)

**Language/Version(语言和版本)**: Rust 2024, rust-version 1.88. 手册使用 mdBook 构建.
**Primary Dependencies(主要依赖)**: 不新增外部 crate. 复用项目已有的 `serde_json`(自检 JSON 输出), `mdBook`(手册构建, 仅 CI 依赖). 不修改 `Cargo.toml` 的 `[dependencies]`.
**Storage(存储)**: MVP tarball 通过 `cargo package` 生成, 发布记录在 `artifacts/release-record.json`, 放行矩阵在 `artifacts/quality-gate-outcome.csv`. 手册通过 `mdbook build` 生成静态 HTML.
**Testing(测试)**: `cargo test`(核心库), `cargo package --list`(tarball 内容校验), mdbook 构建验证. 本切片新增的测试主要是存在性检查和格式校验脚本.
**Target Platform(目标平台)**: macOS 开发者工作站 + Linux CI runner. 部署指南针对 Linux amd64 容器.
**Project Type(项目类型)**: Rust library(库) + 发布工程 + 文档.
**Performance Goals(性能目标)**: N/A. 本切片不涉及运行时性能.
**Constraints(约束)**: 禁止修改 `src/` 生产代码. tarball 内禁止引用未公开私服 registry. 手册与 crate 版本号同步.
**Scale/Scope(规模和范围)**: 单 crate 发布包. 参考拓扑为 docker-compose 单机部署. 值守手册覆盖 P1 场景, 不覆盖 P2/P3.

## Constitution Check(宪章检查)

_GATE(关口): Phase 0(研究阶段) 前必须通过. Phase 1(设计阶段) 后必须重新检查._

- **Module Ownership(模块所有权)**: examples/ 和 manual/ 路径已存在, 不修改 `src/` 生产代码. 本切片约束目录用途而非新建目录. ✅
- **Supervision Contract(监督契约)**: N/A(不适用). 本切片约束交付物和文档, 不改变监督行为. ✅
- **Test Gate(测试关口)**: 本切片的行为变化是文档和发布工程层面的. 新增的存在性检查和格式校验脚本在 tasks.md 中先列后实现. `cargo test` 覆盖率不变. ✅
- **Observable Failures(可观察失败)**: 健康自检 JSON 必须具备稳定顶层键名(FR-003). 放行矩阵空白 td 计数必须为 0(SC-003). ✅
- **Small Increment(小增量)**: 不新增外部 crate 依赖. 不修改生产代码. 对已有 examples 和 manual 做补缺审查而非重写. ✅
- **Chinese Writing(中文写作)**: 本文件及派生物使用中文叙述, 英文术语括注. manual/ 目录已有中英文双语文档. ✅
- **Compat Exports(兼容导出)**: 本切片不新增任何 `pub use` 或模块重导出. ✅

## Project Structure(项目结构)

### Documentation(文档, 本功能)

```text
specs/006-8-product-bundle-runbooks/
├── plan.md              # 本文件, 由 /speckit-plan 生成
├── spec.md              # 功能规格(Accepted)
├── research.md          # Phase 0 输出: 缺口分析与技术方案
├── data-model.md        # Phase 1 输出: 交付物数据模型
├── quickstart.md        # Phase 1 输出: 构建与发布快速开始
├── contracts/           # Phase 1 输出: 接口契约
│   ├── health-selfcheck-schema.md   # 健康自检 JSON schema
│   └── release-matrix-format.md     # 放行矩阵格式契约
├── checklists/
│   └── bundle.md         # 已完成的检查清单(34 项)
└── tasks.md             # Phase 2 输出
```

### Source Code & Deliverables(源代码与交付物, 仓库根目录)

```text
# 已有, 本切片审查不重写
examples/
├── config_tree_supervisor.rs
├── diagnostic_replay.rs
├── observability_probe.rs
├── policy_failure_matrix.rs
├── restart_policy_lab.rs
├── runtime_control_story.rs
├── shutdown_tree.rs
├── supervisor_quickstart.rs
└── supervisor_tree_story.rs

manual/
├── en/
│   ├── deployment-guide.md     # 部署指南(审查补缺)
│   ├── operations-runbook.md   # 值守手册(审查补缺)
│   └── getting-started.md      # 快速开始
├── zh/
│   ├── deployment-guide.md     # 部署指南(中文)
│   ├── operations-runbook.md   # 值守手册(中文)
│   └── getting-started.md      # 快速开始
├── theme/
└── dashboard.md

artifacts/
├── release-record.json         # 发布记录(006-2)
├── quality-gate-outcome.csv    # 放行矩阵
└── validation/
    └── soak-*.md               # 浸泡报告(006-7)

# 本切片新增/更新
scripts/
├── check-tarball-content.sh    # tarball 内容校验脚本
└── validate-release-matrix.sh  # 放行矩阵格式校验脚本
```

**Structure Decision(结构决定)**: 采用现有仓库结构, 不新增目录. 新增的校验脚本放在 `scripts/` 目录(与已有脚本 `check-coding-standard.sh` 等同级). 手册审查在 `manual/en/` 和 `manual/zh/` 中进行, 不在本切片重写而是补充缺口段落(步数上限, 自检 JSON schema 引用, 密钥占位符格式等).

## Complexity Tracking(复杂度跟踪)

> **本切片不违反 Constitution Check. 以下为本切片特有的复杂度说明, 非违反项.**

| Complexity(复杂度项) | Why Needed(为什么需要) | Simpler Alternative Rejected Because(为什么拒绝更简单方案) |
|---|---|---|
| 双语文档审查补缺 | US1 要求部署指南可被照抄拉起, 中文和英文都需要承诺步数上限 | 只维护英文: 不符合 spec 中英文双语要求 |
| 放行矩阵格式契约 | 006-2 的 QualityGateOutcome 是 CSV 格式, 但 US3 要求 DOM 空白 td 检查 | 只依赖 CSV: 发布页面 HTML 需要独立格式契约 |
| 健康自检 JSON schema | US1 要求自检 JSON 具备稳定顶层键名, 当前无正式 schema | 靠自然语言描述: 调用方无法可靠解析 |
| tarball 内容校验脚本 | FR-001 要求 4 类构件存在且禁止私服引用 | 人工检查: 每次发布前手动核对, 不可靠 |
