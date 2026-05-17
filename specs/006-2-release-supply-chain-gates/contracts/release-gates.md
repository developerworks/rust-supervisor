# Contracts(接口契约): 发布门禁

**Feature(功能)**: 006-2-release-supply-chain-gates
**Phase(阶段)**: 1 (设计)
**Date(日期)**: 2026-05-18

本文档定义每条发布门禁的接口契约: gate_id(闸门代号), 执行命令, 输出格式, 判定规则, expected(预期通过) 与 failing(预期失败) 样本.

---

## 门禁总览

| Gate ID(闸门代号)  | Tier(层级) | Tool(工具)                    | Expected Duration(预计耗时) |
| ------------------ | ---------- | ----------------------------- | --------------------------- |
| `fmt`              | shallow    | `cargo fmt`                   | <10s                        |
| `check`            | shallow    | `cargo check`                 | <30s                        |
| `clippy`           | shallow    | `cargo clippy`                | <30s                        |
| `test`             | shallow    | `cargo test`                  | <2min                       |
| `doc`              | shallow    | `cargo doc`                   | <30s                        |
| `publish_dry_run`  | shallow    | `cargo publish`               | <10s                        |
| `dependency_audit` | middle     | `cargo audit`                 | <30s                        |
| `license_check`    | middle     | `cargo deny check licenses`   | <10s                        |
| `advisory_check`   | middle     | `cargo deny check advisories` | <30s                        |
| `semver_checks`    | middle     | `cargo semver-checks`         | <1min                       |
| `msrv_verify`      | middle     | `scripts/verify-msrv.sh`      | <30s                        |
| `sbom_verify`      | middle     | `scripts/verify-sbom.sh`      | <10s                        |
| `coverage`         | deep       | `cargo tarpaulin`             | <5min                       |
| `mutation_testing` | deep       | `cargo mutants`               | <15min                      |
| `fuzzing`          | deep       | `cargo fuzz`                  | 不限                        |
| `loom`             | deep       | `cargo test --test loom_*`    | <10min                      |
| `miri`             | deep       | `cargo miri test`             | <10min                      |

---

## Shallow Gates(浅层门禁)

### fmt

- **gate_id**: `fmt`
- **Command(命令)**: `cargo fmt --check`
- **Pass condition(通过条件)**: 退出码 0, 无 diff
- **Fail output(失败输出)**: 打印 diff, 建议 `cargo fmt` 修复
- **Pass sample(通过样本)**: 代码库已格式化
- **Fail sample(失败样本)**: 任意 `.rs` 文件缩进不对齐

### check

- **gate_id**: `check`
- **Command(命令)**: `cargo check --all-targets`
- **Pass condition(通过条件)**: 退出码 0
- **Fail output(失败输出)**: 编译器错误信息

### clippy

- **gate_id**: `clippy`
- **Command(命令)**: `cargo clippy --all-targets -- -D warnings`
- **Pass condition(通过条件)**: 退出码 0, 0 warnings
- **Fail output(失败输出)**: clippy 告警信息

### test

- **gate_id**: `test`
- **Command(命令)**: `cargo test`
- **Pass condition(通过条件)**: 退出码 0, 所有测试通过
- **Output parsing(输出解析)**: 提取 `test result: ok. N passed; 0 failed`

### doc

- **gate_id**: `doc`
- **Command(命令)**: `cargo doc --no-deps --document-private-items`
- **Pass condition(通过条件)**: 退出码 0

### publish_dry_run

- **gate_id**: `publish_dry_run`
- **Command(命令)**: `cargo publish --dry-run`
- **Pass condition(通过条件)**: 退出码 0

---

## Middle Gates(中层门禁)

### dependency_audit

- **gate_id**: `dependency_audit`
- **Command(命令)**: `cargo audit --deny warnings`
- **Pass condition(通过条件)**: 退出码 0, 无已知 CVE(公开漏洞编号)
- **Fail output(失败输出)**: CVE 编号与受影响 crate 版本列表
- **Blocking(阻断)**: ✅ 阻断发布, 除非提供 ExemptionTicket(豁免工单)

### license_check

- **gate_id**: `license_check`
- **Command(命令)**: `cargo deny check licenses`
- **Config(配置)**: `deny.toml` 中 `[licenses]` 节
- **Pass condition(通过条件)**: 退出码 0, 所有依赖许可证在白名单内
- **Blocking(阻断)**: ✅

### advisory_check

- **gate_id**: `advisory_check`
- **Command(命令)**: `cargo deny check advisories`
- **Config(配置)**: `deny.toml` 中 `[advisories]` 节
- **Pass condition(通过条件)**: 退出码 0, 无命中封锁的安全公告
- **Blocking(阻断)**: ✅

### semver_checks

- **gate_id**: `semver_checks`
- **Command(命令)**: `cargo semver-checks`
- **Pass condition(通过条件)**: 退出码 0, 无接口破坏
- **Fail handling(失败处理)**: 若本轮为 MAJOR(主版本), 破坏允许; 否则阻断并需升级 semver(语义化版本) 等级
- **Blocking(阻断)**: ✅ (非 MAJOR 时)

### msrv_verify

- **gate_id**: `msrv_verify`
- **Command(命令)**: `bash scripts/verify-msrv.sh`
- **Input(输入)**: 仓库 `Cargo.toml` 中 `rust-version` 字段
- **Pass condition(通过条件)**: 用指定 MSRV(最低 Rust 版本) 版本的 rustc 编译通过
- **Fail output(失败输出)**: 非零退出码 + 升级提示文案含文档章节号
- **SC-004 要求**: 失败样本必须在固定 5 步以内退出并打印章节号. 五步定义为:
  1. 从 `Cargo.toml` 提取 `rust-version` 字段值作为 MSRV(最低 Rust 版本)
  2. 检查 `rustup toolchain list` 中对应版本是否已安装
  3. 若缺失则执行 `rustup toolchain install <msrv>`
  4. 执行 `cargo +<msrv> check --all-targets`
  5. 根据退出码打印 `MSRV OK` 或 `MSRV FAIL: see manual section X.Y`

### sbom_verify

- **gate_id**: `sbom_verify`
- **Command(命令)**: `bash scripts/verify-sbom.sh`
- **Pass condition(通过条件)**: SBOM 文件哈希与 attestation(证明) 中记录一致
- **SC-002 要求**: 外部复验哈希一致率 100%

---

## Deep Gates(深层门禁, 仅夜间队列)

### coverage

- **gate_id**: `coverage`
- **Command(命令)**: `cargo tarpaulin --out json --output-dir artifacts/coverage`
- **Threshold(阈值)**: 80% (可配置)
- **Pass condition(通过条件)**: 覆盖率 ≥ 阈值

### mutation_testing

- **gate_id**: `mutation_testing`
- **Command(命令)**: `cargo mutants`
- **Output(输出)**: 变异存活/杀死统计
- **Nightly only(仅夜间)**: ✅

### fuzzing

- **gate_id**: `fuzzing`
- **Command(命令)**: `cargo fuzz run <target> -- -max_total_time=3600`
- **Pass condition(通过条件)**: 退出码 0, 0 crashes(崩溃)
- **Configurable(可配置)**: 种子目录, 超时, 最大输入长度

### loom

- **gate_id**: `loom`
- **Command(命令)**: `cargo test --test loom_*` (loom 测试以独立 test target 存在)
- **Pass condition(通过条件)**: 退出码 0
- **Nightly only(仅夜间)**: ✅

### miri

- **gate_id**: `miri`
- **Command(命令)**: `cargo miri test`
- **Pass condition(通过条件)**: 退出码 0, 0 undefined behavior(未定义行为)
- **Nightly only(仅夜间)**: ✅

---

## Blocking Gate Audit(阻断台账写入规则)

中层门禁失败时, CI workflow 的阻断行为与台账标记遵循以下规则:

| Scenario(场景)   | CI 行为                        | QualityGateOutcome.outcome | release-record.json gates.middle.\* | ExemptionTicket(豁免工单) |
| ---------------- | ------------------------------ | -------------------------- | ----------------------------------- | ------------------------- |
| 门禁通过         | 继续                           | `passed`                   | `"passed"`                          | 不需要                    |
| 门禁失败, 无豁免 | **阻断, workflow 标记 failed** | `failed`                   | `"failed"`                          | 无                        |
| 门禁失败, 有豁免 | 不阻断, 打印告警               | `waived`                   | `"waived (EX-YYYY-NNN)"`            | 必须提供有效编号          |

阻断时, CI 退出码非零, 发布台账不得写 `released_at` 时间戳. 人工审批后若放行, 必须在 `released_at` 旁追加 `waived_by` 字段记录审批人标识.

---

## OutCome Enum(结论枚举)

所有闸门共用 `QualityGateOutcome` 的五类取值:

| Value(值) | Meaning(含义) | When(何时出现)                                         |
| --------- | ------------- | ------------------------------------------------------ |
| `passed`  | 通过          | 闸门执行成功且满足阈值                                 |
| `failed`  | 失败          | 闸门执行失败或未达阈值                                 |
| `waived`  | 豁免          | 闸门失败但有有效 ExemptionTicket(豁免工单)             |
| `skipped` | 跳过          | 闸门未执行但已计划 (如非 MAJOR 版本的 semver_checks)   |
| `missing` | 缺失          | 闸门未执行且未计划 (台账空白, 视为 incomplete(不完整)) |
