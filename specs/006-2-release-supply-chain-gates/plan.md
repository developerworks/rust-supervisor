# Implementation Plan(实现计划): 工业级发布门禁与供应链证明

**Branch(分支)**: `[006-2-release-supply-chain-gates]` | **Date(日期)**: 2026-05-17 | **Spec(规格)**: `specs/006-2-release-supply-chain-gates/spec.md`
**Input(输入)**: 功能规格来自 `specs/006-2-release-supply-chain-gates/spec.md`

## Summary(摘要)

本切片约束发布工程能力与记录模板, 不改变运行时监督语义. 核心交付物包括: 发布流水线脚本集合, 门禁记录模板 (ReleaseRecord, QualityGateOutcome, ExemptionTicket), 以及一份可被第三方复验的发布台账. 当前 README 已列出部分门禁 (cargo fmt/check/test/doc/SBOM/publish --dry-run), 需要补齐 signed tag 策略, changelog 模板, MSRV 自检脚本, cargo-deny, cargo-semver-checks, cargo-mutants, code coverage, fuzzing(模糊测试), loom(并发模型测试), miri(未定义行为检查) 的台账槽位与归档路径.

## Technical Context(技术背景)

**Language/Version(语言和版本)**: Rust(编程语言) 2024, rust-version 1.88
**Primary Dependencies(主要依赖)**: 开发期工具: cargo-deny, cargo-semver-checks, cargo-mutants, cargo-fuzz(或等价), cargo-loom(或等价), cargo-miri. 不得在默认 cargo install 路径中静默拉高运行时依赖.
**Storage(存储)**: 发布描述文件驻留仓库 release/ 目录或 git tag 附属文件; 台账以 Markdown/JSON 格式归档.
**Testing(测试)**: 门禁脚本必须返回稳定 exit code(退出码) 与 gate_id(闸门代号) 字符串.
**Target Platform(目标平台)**: CI 运行环境 (Linux x86_64); 发布验证跨平台.
**Project Type(项目类型)**: CI 描述文件 + 脚本集合 + 记录模板.
**Performance Goals(性能目标)**: N/A(不适用). 门禁脚本不计入运行时延迟预算.
**Constraints(约束)**: 禁止兼容导出. CI 脚本变更必须经过与普通源码同等力度的评审标签.
**Scale/Scope(规模和范围)**: 单仓库发布流水线, 每条记录对应一次对外版本.

## Constitution Check(宪章检查)

*GATE(关口): Phase 0(研究阶段) 前必须通过. Phase 1(设计阶段) 后必须重新检查.*

- **Module Ownership(模块所有权)**: CI 描述文件与发布脚本存放在 .github/workflows/ 或 tools/ 目录. 模板记录文件存放在 specs/006-2/ 或 release-templates/.
- **Supervision Contract(监督契约)**: N/A(不适用). 本切片不改变运行时监督语义.
- **Test Gate(测试关口)**: 门禁脚本的通过阈值必须有断言. 模拟发布验证幂等性与哈希一致性.
- **Observable Failures(可观察失败)**: 门禁脚本返回错误时必须打印稳定 gate_id(闸门代号) 与处置段落锚点.
- **Small Increment(小增量)**: 不引入新运行时 crate, 只引入开发期工具.
- **Chinese Writing(中文写作)**: 本文件与派生物使用中文叙述, 英文术语括注.

## Project Structure(项目结构)

### Documentation(文档, 本功能)

```text
specs/006-2-release-supply-chain-gates/
├── spec.md              # 功能规格
├── plan.md              # 本文件
├── research.md          # Phase 0(研究阶段) 输出: 工具链兼容性调研
├── data-model.md        # Phase 1(设计阶段) 输出: ReleaseRecord / QualityGateOutcome / ExemptionTicket 字段定义
├── quickstart.md        # Phase 1(设计阶段) 输出: 发布一份对外版本的操作顺序
├── contracts/
│   └── release-record-schema.md  # 发布台账 JSON schema
└── tasks.md             # Phase 2(任务阶段) 输出
```

### Source Code(源代码, 仓库根目录)

```text
.github/workflows/
├── release.yml          # 发布流水线定义
├── nightly-deep-check.yml  # 夜间深度测试队列 (loom, miri, fuzzing, mutants)
└── pr-check.yml         # PR 级门禁 (fmt, check, test, deny)

tools/
├── check-msrv.sh        # MSRV 自检脚本
├── check-semver.sh      # cargo-semver-checks 封装 (退出码标准化)
├── check-deny.sh        # cargo-deny 封装
├── check-mutants.sh     # cargo-mutants 封装
├── check-fuzz.sh        # fuzzing 入口脚本
├── check-loom.sh        # loom 入口脚本
├── check-miri.sh        # miri 入口脚本
├── gen-sbom.sh          # SBOM 生成脚本
├── gen-attestation.sh   # supply chain attestation 摘要脚本
└── gen-changelog.sh     # changelog 模板生成脚本

release-templates/
├── RELEASE_RECORD.md    # 发布台账模板
├── CHANGELOG.md         # changelog 模板
└── QUALITY_GATE.csv     # 放行矩阵导出模板
```

**Structure Decision(结构决定)**: 工具脚本统一放在 tools/ 目录, CI 工作流在 .github/workflows/ 中引用这些脚本. 发布记录模板在 release-templates/ 下与规范隔离.

## Complexity Tracking(复杂度跟踪)

> **只有 Constitution Check(宪章检查) 存在违反项时, 才填写本节.**

| Violation(违反项) | Why Needed(为什么需要) | Simpler Alternative Rejected Because(为什么拒绝更简单方案) |
|---|---|---|
| N/A(不适用) | - | - |
