# Data Model(数据模型): 发布台账与闸门记录

**Feature(功能)**: 006-2-release-supply-chain-gates
**Phase(阶段)**: 1 (设计)
**Date(日期)**: 2026-05-18

## 概述

本切片不涉及 Rust 代码实体. 数据模型定义三类发布台账文件的 JSON/CSV/Markdown 结构.

## ReleaseRecord(发布记录)

每次对外版本的不可变指针元组, 绑定 tag, changelog, SBOM, attestation 与各闸口摘要.

### JSON 结构 (`artifacts/release-record.json`)

```json
{
  "version": "0.1.2",
  "semver_level": "patch",
  "commit": "abc123def456",
  "signed_tag": {
    "tag": "v0.1.2",
    "signature_type": "gpg",
    "fingerprint": "AAAA BBBB CCCC DDDD",
    "verify_command": "git tag -v v0.1.2"
  },
  "msrv": {
    "version": "1.88",
    "verify_script": "scripts/verify-msrv.sh",
    "verified": true
  },
  "changelog": {
    "path": "CHANGELOG.md",
    "section": "## 0.1.2"
  },
  "sbom": {
    "path": "artifacts/sbom/sbom.spdx.json",
    "sha256": "abcdef...",
    "format": "spdx-2.3",
    "format_version": "2.3",
    "migration_note": "artifacts/sbom-migration.md",
    "verify_script": "scripts/verify-sbom.sh"
  },
  "supply_chain_attestation": {
    "path": "artifacts/attestation.json",
    "sha256": "abcdef...",
    "verify_script": "scripts/verify-attestation.sh"
  },
  "gates": {
    "shallow": {
      "fmt": "passed",
      "check": "passed",
      "clippy": "passed",
      "test": "21/21 passed",
      "doc": "passed",
      "publish_dry_run": "passed"
    },
    "middle": {
      "dependency_audit": "passed",
      "license_check": "passed",
      "advisory_check": "passed",
      "semver_checks": "passed",
      "msrv_verify": "passed"
    },
    "deep": {
      "coverage": "85.2% (threshold 80%)",
      "mutation_testing": "waived (EX-2026-001)",
      "fuzzing": "passed (3600s, 0 crashes)",
      "loom": "waived (EX-2026-002)",
      "miri": "passed"
    }
  },
  "released_at": "2026-05-18T00:00:00Z",
  "released_by": "release-bot",
  "waived_by": null
}
```

### 字段说明

| 字段                        | 类型   | 必填 | 说明                                    |
| --------------------------- | ------ | ---- | --------------------------------------- |
| `version`                   | String | ✅   | semver(语义化版本) 版本号, e.g. `0.1.2` |
| `semver_level`              | Enum   | ✅   | `major`, `minor`, `patch` 之一          |
| `commit`                    | String | ✅   | 发布对应的 commit SHA                   |
| `signed_tag`                | Object | ✅   | 签名标签信息                            |
| `signed_tag.tag`            | String | ✅   | 标签名, e.g. `v0.1.2`                   |
| `signed_tag.signature_type` | String | ✅   | `gpg` 或 `ssh`                          |
| `signed_tag.fingerprint`    | String | ✅   | 公钥指纹                                |
| `signed_tag.verify_command` | String | ✅   | 外部验签命令                            |
| `msrv`                      | Object | ✅   | 最低 Rust 版本信息                      |
| `changelog`                 | Object | ✅   | 变更日志指针                            |
| `sbom`                      | Object | ✅   | 软件物料清单指针与哈希                  |
| `supply_chain_attestation`  | Object | ✅   | 供应链证明指针与哈希                    |
| `gates`                     | Object | ✅   | 按层级分组的闸门结果                    |
| `released_at`               | String | ✅   | ISO 8601 时间戳                         |
| `released_by`               | String | ✅   | 发布者标识                              |
| `waived_by`                 | String | ❌   | 人工审批放行者标识, 仅阻断后放行时填写  |

## QualityGateOutcome(质量闸口结果)

单行闸口结论, CSV 格式 (`artifacts/quality-gate-outcome.csv`), 每行一个闸门.

### CSV 列

| 列名               | 说明         | 合法值                                             |
| ------------------ | ------------ | -------------------------------------------------- |
| `gate_id`          | 闸门代号     | e.g. `fmt`, `clippy`, `dependency_audit`, `loom`   |
| `tier`             | 层级         | `shallow`, `middle`, `deep`                        |
| `outcome`          | 结论         | `passed`, `failed`, `waived`, `skipped`, `missing` |
| `detail`           | 详情         | 退出码, 通过/总数, 百分比等                        |
| `exemption_ticket` | 豁免工单编号 | 仅 `waived` 行填写, e.g. `EX-2026-001`             |
| `exemption_url`    | 豁免工单链接 | 仅 `waived` 行填写                                 |
| `log_path`         | 日志归档路径 | 闸门完整输出路径                                   |
| `timestamp`        | 执行时间     | ISO 8601                                           |

### 示例行

```csv
gate_id,tier,outcome,detail,exemption_ticket,exemption_url,log_path,timestamp
fmt,shallow,passed,0 diffs,,,,2026-05-18T00:00:00Z
clippy,shallow,passed,0 warnings,,,,2026-05-18T00:00:00Z
dependency_audit,middle,passed,0 vulnerabilities,,,,2026-05-18T00:00:00Z
loom,deep,waived,not executed this cycle,EX-2026-002,https://example.com/exemptions/EX-2026-002,artifacts/logs/loom-nightly-2026-05-17.log,2026-05-18T00:00:00Z
miri,deep,passed,0 UB,,,,2026-05-18T00:00:00Z
```

## ExemptionTicket(豁免工单)

人工批准的绕行凭证, Markdown 格式 (`artifacts/exemption-ticket.md` 或每工单一个文件).

### 模板

```markdown
# Exemption Ticket(豁免工单) EX-2026-001

- **Number(编号)**: EX-2026-001
- **Gate ID(闸门代号)**: mutation_testing
- **Requested by(申请人)**: release-bot
- **Approved by(批准人)**: security-officer
- **Effective from(生效日期)**: 2026-05-01
- **Expires on(截止日期)**: 2026-08-01
- **Risk assessment(风险评估)**: 本轮未修改 `src/policy/` 下任何代码, 变异测试对策略模块的覆盖
  在上轮 release 已经通过. 本豁免仅限 0.1.2 发布.
- **Mitigation(缓解措施)**: 夜间队列将继续执行变异测试并在失败时自动撤回本豁免.
```

### 字段

| 字段              | 类型   | 必填 | 说明                             |
| ----------------- | ------ | ---- | -------------------------------- |
| `number`          | String | ✅   | 唯一工单编号, 格式 `EX-YYYY-NNN` |
| `gate_id`         | String | ✅   | 被豁免的闸门代号                 |
| `requested_by`    | String | ✅   | 申请人标识                       |
| `approved_by`     | String | ✅   | 批准人标识                       |
| `effective_from`  | String | ✅   | 生效日期                         |
| `expires_on`      | String | ✅   | 截止日期, 到期后豁免自动失效     |
| `risk_assessment` | String | ✅   | 风险评估段落, 禁止留白           |
| `mitigation`      | String | ✅   | 缓解措施段落                     |
