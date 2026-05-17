# Quickstart(快速开始): 发布操作

**Feature(功能)**: 006-2-release-supply-chain-gates
**Phase(阶段)**: 1 (设计)
**Date(日期)**: 2026-05-18

## 谁会需要这份文档

- **发布责任人**: 执行一次正式 release(版本发布).
- **供应链安全专员**: 验签, 验 SBOM, 验 attestation.
- **质量工程师**: 审查深度门禁结论, 填写豁免工单.

## 阅读顺序

### 步骤 1: 理解门禁层级 (5 分钟)

三层门禁:

| Tier(层级)    | When(何时执行)         | Blocking(是否阻断)           | Duration(耗时) |
| ------------- | ---------------------- | ---------------------------- | -------------- |
| shallow(浅层) | 每次 PR + 每次 release | ✅ 阻断                      | <5min          |
| middle(中层)  | 每次 release           | ✅ 阻断 (除豁免外)           | <5min          |
| deep(深层)    | 夜间队列               | ❌ 不阻断 PR; release 需结论 | 不限           |

### 步骤 2: 运行一次全量发布检查 (5 分钟)

```bash
# 在仓库根目录执行
bash scripts/release-check.sh
```

该脚本按顺序执行所有浅层与中层门禁, 并打印每项 `gate_id` 与 `passed/failed`.

### 步骤 3: 签署发布标签 (2 分钟)

```bash
# 确认当前 commit 是发布候选
git log --oneline -1

# 用 GPG 签署标签 (SSH 密钥也可, 替换 -s 为 -u <key>)
git tag -s v0.2.0 -m "v0.2.0: IPC security control points (006-1)"

# 推送到远端
git push --tags
```

验签命令:

```bash
# 验证标签签名
git tag -v v0.2.0
```

台账 `signed_tag` 字段填写:

- `tag`: 标签名, 如 `v0.2.0`
- `signature_type`: `gpg` 或 `ssh`
- `fingerprint`: 通过 `gpg --list-keys` 或 `ssh-keygen -lf` 获取公钥指纹
- `verify_command`: 外部验签命令, 如 `git tag -v v0.2.0`

### 步骤 4: 填充发布台账 (5 分钟)

复制 `artifacts/release-record.json` 模板:

```bash
cp artifacts/release-record.json artifacts/releases/v0.2.0-record.json
```

逐字段填写:

- `version`: semver(语义化版本) 版本号, 如 `0.1.2`
- `semver_level`: `major`, `minor`, `patch` 之一
- `commit`: `git rev-parse HEAD` 输出
- `signed_tag.*`: 按步骤 3 签名的标签信息
- `msrv.verified`: 运行 `bash scripts/verify-msrv.sh` 确认后填 `true`
- `changelog.section`: 指向 `CHANGELOG.md` 中对应版本小节
- `sbom.path`: SBOM(软件物料清单) 文件绝对路径
- `sbom.sha256`: 运行 `sha256sum <sbom.path>` 计算结果
- `supply_chain_attestation.*`: 运行 `bash scripts/generate-attestation.sh` 生成后填入
- `gates.shallow.*`: 从 CI 日志或 `scripts/release-check.sh` 输出中逐项填入 `passed`/`failed`
- `gates.middle.*`: 同上
- `gates.deep.*`: 从夜间队列归档中填入, 本轮未执行则写 `waived (EX-YYYY-NNN)` 或 `missing`
- `released_at`: ISO 8601 时间戳, 门禁全部通过后方可填写; 阻断时留空
- `released_by`: 发布者标识 (GitHub handle 或邮箱)
- `waived_by`: 如人工审批放行, 填写审批人标识; 否则 `null`

### 步骤 5: 外部复验 (3 分钟)

```bash
# 验签
git tag -v v0.2.0

# 验 SBOM
bash scripts/verify-sbom.sh artifacts/releases/v0.2.0-record.json

# 验供应链证明
bash scripts/verify-attestation.sh artifacts/releases/v0.2.0-record.json
```

### 步骤 6: 填写豁免工单 (如需)

若任何门禁失败但有合理理由, 复制模板:

```bash
cp artifacts/exemption-ticket.md artifacts/exemptions/EX-2026-003.md
```

填写工单编号, 闸门代号, 风险评估, 缓解措施. 将编号填回 `quality-gate-outcome.csv`.

## 常见问题

### Q: 非 MAJOR 版本时 semver_checks 失败怎么处理?

选项: (1) 抬升 semver(语义化版本) 等级到 MAJOR, (2) 回退接口破坏性变更. 不允许在非 MAJOR 版本中静默破坏接口.

### Q: 夜间队列未跑 deep 门禁, 能发布吗?

能, 但 `QualityGateOutcome` 中对应行必须填 `missing` 并附带豁免编号. 连续 3 次 release 缺少同一 deep 门禁时, 豁免自动失效.

### Q: 如何配置 deny.toml?

参考 `cargo-deny` 官方模板. 至少配置:

- `[licenses]`: 允许的许可证列表 (MIT, Apache-2.0, BSD-3-Clause 等)
- `[advisories]`: `vulnerability = "deny"`, `unmaintained = "warn"`
- `[bans]`: 禁止的 crate 列表 (初始为空). 以下情况应追加条目: (1) 发现同一功能存在多个重复 crate 时禁止较低维护者; (2) 安全公告确认恶意 crate 时立即封锁; (3) 组织政策禁止特定许可证或来源的 crate. 修改 `deny.toml` 后需同时在 `CHANGELOG.md` 中记录.
