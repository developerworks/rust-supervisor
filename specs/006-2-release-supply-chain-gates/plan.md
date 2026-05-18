# Implementation Plan(实现计划): 工业级发布门禁与供应链证明

**Branch(分支)**: `006-2-release-supply-chain-gates` | **Date(日期)**: 2026-05-18 | **Spec(规格)**: `specs/006-2-release-supply-chain-gates/spec.md`
**Input(输入)**: 功能规格来自 `specs/006-2-release-supply-chain-gates/spec.md`

## Summary(摘要)

本切片处理发布流程与供应链安全横切线. 仓库页面目前缺少正式 release(版本发布). 工业级发布需补齐: signed tag(签名标签), changelog(变更日志), semver(语义化版本), MSRV(最低 Rust 版本) 验证, dependency audit(依赖审计), cargo-deny(依赖策略检查), cargo-semver-checks(接口兼容检查), cargo-mutants(变异测试), code coverage(覆盖率), fuzzing(模糊测试), loom(并发模型测试), miri(未定义行为检查), supply chain attestation(供应链证明). 交付件是 CI 门禁脚本集, 发布台账模板, 与外部可复验的证明链路.

## Technical Context(技术背景)

**Language/Version(语言和版本)**: Rust(编程语言) 2024, rust-version 1.88. 门禁脚本以 shell(外壳脚本) 为主, 发布台账以 Markdown / JSON 承载.
**Primary Dependencies(主要依赖)**: cargo-deny, cargo-semver-checks, cargo-mutants, cargo-tarpaulin(覆盖率), cargo-fuzz(libfuzzer 绑定), loom, miri. 均为开发期工具, 不进入运行时依赖树.
**Storage(存储)**: 台账文件 (ReleaseRecord 为 JSON, QualityGateOutcome 为 CSV, changelog 为 Markdown). SBOM(软件物料清单) 由 cargo-sbom 或 cyclonedx 生成.
**Testing(测试)**: 门禁脚本的验收方式为对样板仓库执行模拟发布, 校验产出台账与外部复验脚本结果一致.
**Target Platform(目标平台)**: CI runner(持续集成执行器) (Linux, GitHub Actions). 部分深度门禁 (loom, miri) 仅限 nightly(每夜构建) 工具链.
**Project Type(项目类型)**: CI/CD pipeline(持续集成/持续交付流水线) + release runbook(发布运行手册).
**Performance Goals(性能目标)**: 浅层门禁 (fmt, check, test, clippy, deny) 在 5 分钟内完成. 深层门禁 (fuzz, loom, miri, mutants) 在夜间队列执行, 不限时.
**Constraints(约束)**: 门禁脚本不得在默认 `cargo install` 路径里静默拉高运行时依赖. 台账模板必须可被外部 shell / Python 脚本复算而不依赖专有工具.
**Scale/Scope(规模和范围)**: 单仓库, 约 20 个门禁步骤. 每次 release(版本发布) 产出约 10 个台账文件.

## Constitution Check(宪章检查)

_GATE(关口): Phase 0(研究阶段) 前必须通过. Phase 1(设计阶段) 后必须重新检查._

- **Module Ownership(模块所有权)**: CI 描述文件与发布脚本存放在 `.github/workflows/` 与 `scripts/` 路径树下. 台账模板存放在 `artifacts/` 下. 不涉及 `src/` 变更.
- **Supervision Contract(监督契约)**: N/A(不适用). 本切片约束交付工程台账与闸门脚本, 不改变监督运行时状态机.
- **Test Gate(测试关口)**: 门禁脚本的验证以模拟发布抽查执行. 最终运行 `bash scripts/validate-sbom.sh` 等校验脚本.
- **Observable Failures(可观察失败)**: 任一闸口失败打印稳定 `gate_id`(闸门代号) 字符串, 并在台账附录附处置动作.
- **Small Increment(小增量)**: 仅引入开发期/CI 期工具 (cargo-deny 等), 不改变运行时依赖.
- **Chinese Writing(中文写作)**: 本文件及所有派生物使用中文, 英文术语写成 `English(中文说明)`.

## Project Structure(项目结构)

### Documentation(文档, 本功能)

```text
specs/006-2-release-supply-chain-gates/
├── spec.md              # 功能规格
├── plan.md              # 本文件
├── research.md          # Phase 0(研究阶段) 输出: 门禁工具选型研究
├── data-model.md        # Phase 1(设计阶段) 输出: 台账实体模型
├── quickstart.md        # Phase 1(设计阶段) 输出: 发布操作快速开始
├── contracts/
│   └── release-gates.md # 门禁接口契约
└── tasks.md             # Phase 2(任务阶段) 输出
```

### Source Code(源代码, 仓库根目录)

```text
# 本切片不修改 src/ 代码. 交付件集中在 CI 与 artifacts 目录.

.github/workflows/
├── release-gates.yml    # 发布门禁流水线 (浅层 + 深层)
└── nightly-gates.yml    # 夜间深度门禁 (fuzz, loom, miri, mutants)

scripts/
├── verify-msrv.sh       # MSRV(最低 Rust 版本) 自检脚本
├── verify-sbom.sh       # SBOM(软件物料清单) 外部复验脚本
├── verify-attestation.sh # 供应链证明校验脚本
└── release-check.sh     # 发布前置全量检查入口

artifacts/
├── release-record.json  # 发布记录模板
├── quality-gate-outcome.csv  # 质量闸口结果模板
├── exemption-ticket.md  # 豁免工单模板
└── sbom/                # 已有 SBOM 生成脚本 (generate-sbom.sh)
```

**Structure Decision(结构决定)**: CI 描述文件放 `.github/workflows/`, 校验脚本放 `scripts/`, 台账模板放 `artifacts/`. 不修改 `src/` 任何文件. 与宪章要求的模块所有权一致.

## Complexity Tracking(复杂度跟踪)

> **只有 Constitution Check(宪章检查) 存在违反项时, 才填写本节.**

| Violation(违反项) | Why Needed(为什么需要) | Simpler Alternative Rejected Because(为什么拒绝更简单方案) |
| ----------------- | ---------------------- | ---------------------------------------------------------- |
| N/A(不适用)       | -                      | -                                                          |
