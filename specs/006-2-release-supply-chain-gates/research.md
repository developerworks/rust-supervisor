# Research(研究): 发布门禁工具选型与策略

**Feature(功能)**: 006-2-release-supply-chain-gates
**Phase(阶段)**: 0 (研究)
**Date(日期)**: 2026-05-18

## 研究问题清单

1. 门禁工具选型: 每项检查用什么工具, 版本, 集成方式.
2. Signed tag(签名标签) 策略: Git tag 签名 vs Sigstore 等方案.
3. Supply chain attestation(供应链证明): 用什么格式, 如何外部复验.
4. 深度门禁 (loom/miri/fuzz/mutants) 的 CI 策略.

## 1. 门禁工具选型

### 浅层门禁 (每次 PR 与每次 release 均执行)

| 门禁                      | 工具                    | 版本                            | 命令                                           | 预计耗时 |
| ------------------------- | ----------------------- | ------------------------------- | ---------------------------------------------- | -------- |
| Format(格式化检查)        | `cargo fmt`             | Rust 内置                       | `cargo fmt --check`                            | <10s     |
| Compile(编译检查)         | `cargo check`           | Rust 内置                       | `cargo check --all-targets`                    | <30s     |
| Lint(静态检查)            | `cargo clippy`          | Rust 内置                       | `cargo clippy --all-targets -- -D warnings`    | <30s     |
| Test(单元/集成测试)       | `cargo test`            | Rust 内置                       | `cargo test`                                   | <2min    |
| Doc(文档生成)             | `cargo doc`             | Rust 内置                       | `cargo doc --no-deps --document-private-items` | <30s     |
| SBOM(软件物料清单)        | `cargo-sbom` 或自行脚本 | 已有 `scripts/generate-sbom.sh` | 已有                                           | <30s     |
| Publish dry-run(发布预演) | `cargo publish`         | Rust 内置                       | `cargo publish --dry-run`                      | <10s     |

### 中层门禁 (每次 release 执行)

| 门禁                            | 工具                              | 理由                                             | 命令                          |
| ------------------------------- | --------------------------------- | ------------------------------------------------ | ----------------------------- |
| Dependency audit(依赖审计)      | `cargo audit` (cargo-audit 0.21)  | 标准 Rust 生态依赖漏洞扫描                       | `cargo audit --deny warnings` |
| License compliance(许可证合规)  | `cargo-deny check license` (0.16) | 统一依赖策略引擎, 可配置许可证白名单与被禁 crate | `cargo deny check licenses`   |
| Advisory check(安全公告检查)    | `cargo-deny check advisories`     | 同一工具, 统一策略文件 `deny.toml`               | `cargo deny check advisories` |
| Semver compat(语义化版本兼容)   | `cargo-semver-checks` (0.39)      | de facto 标准 Rust 接口兼容检查                  | `cargo semver-checks`         |
| MSRV verify(最低 Rust 版本校验) | 自写 shell 脚本 + `rustup`        | 无需专用工具, shell 即够                         | `scripts/verify-msrv.sh`      |

**决策**: 中层门禁使用 `cargo-deny` (而非独立的 `cargo-audit` + `cargo-license`) 以减少工具数量. `cargo-deny` 的单一 `deny.toml` 配置文件覆盖许可证, 安全公告, 依赖源三项策略.

### 深层门禁 (夜间队列执行, 不限时)

| 门禁                               | 工具                          | 理由                   | 最低 Rust 版本 |
| ---------------------------------- | ----------------------------- | ---------------------- | -------------- |
| Code coverage(代码覆盖率)          | `cargo-tarpaulin` (0.31)      | 比 `grcov` 配置更简单  | stable         |
| Mutation testing(变异测试)         | `cargo-mutants` (24.7)        | 比 mutagen 维护活跃    | nightly        |
| Fuzzing(模糊测试)                  | `cargo-fuzz` (libfuzzer 绑定) | LLVM libfuzzer 集成    | nightly        |
| Concurrency model(并发模型测试)    | `loom` (0.7)                  | 标准 Rust 并发模型检查 | nightly        |
| Undefined behavior(未定义行为检查) | `miri` (Rust 内置)            | 不需要额外安装         | nightly        |

**决策**: 深层门禁仅在 nightly 工具链的夜间队列执行. 门禁失败时不阻断 PR, 但 release 台账必须引用 night 队列归档指针.

## 2. Signed Tag(签名标签) 策略

### 方案对比

| 方案                     | 复杂度           | 采购方验签难度           | 建议                      |
| ------------------------ | ---------------- | ------------------------ | ------------------------- |
| Git signed tag (GPG/SSH) | 低, 仓库原生支持 | `git tag -v` 即可        | ✅ 采用                   |
| Sigstore (cosign)        | 中, 需额外服务   | 需 Sigstore 公钥基础设施 | 暂不采用                  |
| 纯 Signed-off-by (DCO)   | 极低             | 无法验签, 仅可追溯       | 作为补充, 不替代 tag 签名 |

**决策**: 采用 Git signed tag (GPG 或 SSH 密钥签名). 发布台账记录 tag 名, 哈希, 签名者公钥指纹. 外部验签命令: `git tag -v <tag-name>`.

### 实施

1. 发布者配置 GPG 或 SSH 签名密钥.
2. release 流水线执行 `git tag -s <version> -m "<version>"`.
3. 台账 `ReleaseRecord` 的 `signed_tag` 字段记录 tag 名与签名指纹.

## 3. Supply Chain Attestation(供应链证明)

### 格式选型

| 格式                | 生态                                 | 外部工具支持   | 建议                    |
| ------------------- | ------------------------------------ | -------------- | ----------------------- |
| in-toto 证明 (SLSA) | 广泛, 含 GitHub Artifact Attestation | 高             | ✅ 采用                 |
| 自写 JSON 摘要      | 仓库内部                             | 需自建校验脚本 | 作为 in-toto 的轻量替代 |

**决策**: 优先采用轻量 JSON 证明 (与 SBOM 同目录, 含文件哈希链路), 外部复验脚本 `scripts/verify-attestation.sh` 逐项重算哈希. 若后续需对接 in-toto/SLSA, JSON 字段映射成本低.

### 证明内容

```json
{
  "version": "0.1.2",
  "commit": "abc123def",
  "timestamp": "2026-05-18T00:00:00Z",
  "artifacts": {
    "crate": {
      "path": "target/package/rust-tokio-supervisor-0.1.2.crate",
      "sha256": "..."
    },
    "sbom": { "path": "artifacts/sbom/sbom.spdx.json", "sha256": "..." }
  },
  "gates": {
    "audit": "passed",
    "deny": "passed",
    "semver_checks": "passed",
    "test": "21/21 passed"
  }
}
```

## 4. 深度门禁 CI 策略

### 时间预算

浅层 + 中层门禁在 PR 与 release 流水线执行, 总耗时 ≤ 5 分钟.
深层门禁在独立 `nightly-gates.yml` 执行, 不限时.

### 失败处理

深层门禁失败生 `QualityGateOutcome` 的 `waived`(豁免) 或 `failed`(失败) 行. 若为 `failed` 且附带 `ExemptionTicket`, 台账仍可标记为 `waived`.

### Nightly 工具链

`nightly-gates.yml` 使用 `rustup default nightly` 并在矩阵中指定 nightly 日期 (如 `nightly-2026-05-01`) 以确保可复现.

## 结论

- 浅层门禁: Rust 内置工具, 标准 CI.
- 中层门禁: `cargo-deny`(统一许可证+安全公告), `cargo-semver-checks`, MSRV 自检脚本.
- 深层门禁: nightly 工具链上的 `cargo-tarpaulin`, `cargo-mutants`, `cargo-fuzz`, `loom`, `miri`.
- 签名标签: GPG/SSH signed tag.
- 证明格式: 轻量 JSON 证明 + 外部复验 shell 脚本.
