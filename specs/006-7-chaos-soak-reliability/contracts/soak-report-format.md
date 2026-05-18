# Contract(契约): SoakReport Markdown Format(浸泡报告 Markdown 格式)

**Status(状态)**: Draft(草稿) | **Version(版本)**: 1.0.0
**Applies to(适用范围)**: `tests/soak/report.rs` 的格式化实现

## 1. 文件命名

```
artifacts/validation/soak-{YYYYMMDD}-{HHMMSS}.md
```

时间戳使用测试窗口结束时间的 UTC.

## 2. Markdown 结构

### 2.1 Metadata 段落

```markdown
# SoakReport

## Metadata

- **Window**: {start_utc} - {end_utc}
- **Commit**: {commit_hash}
- **Hardware**: {hardware_description}
```

- `start_utc` / `end_utc`: ISO 8601 格式, 如 `2026-05-19T00:00:00Z`.
- `commit_hash`: `git rev-parse HEAD` 的全量 SHA.
- `hardware_description`: 自由文本, 如 `macOS Apple Silicon, 16GB`.

### 2.2 Thresholds 表格

```markdown
## Thresholds

| Metric                 | p99   | Avg   | Max   | Limit | Passed     |
| ---------------------- | ----- | ----- | ----- | ----- | ---------- |
| p99_latency_ms         | {val} | {val} | {val} | {val} | true/false |
| rss_growth_mb_per_hour | {val} | {val} | {val} | {val} | true/false |
| fd_count_drift         | {val} | {val} | {val} | {val} | true/false |
| event_gap_total        | {val} | {val} | {val} | {val} | true/false |
| shutdown_success_ratio | {val} | {val} | {val} | {val} | true/false |
```

- 数值格式: 浮点数保留 2 位小数, 整数保留整数.
- `Passed` 列: `true`(通过) 或 `false`(越界).

### 2.3 Violations 段落

```markdown
## Violations

| Metric | Actual | Limit | Blocking | Exemption Ticket |
| ------ | ------ | ----- | -------- | ---------------- |
| {name} | {val}  | {val} | yes/no   | {ticket_id} or - |
```

- 如果无越界, 写 `(none)`.
- `Blocking` 标记条件: 连续 5 个采样窗口越阈.
- 非 blocking 的越界必须挂豁免工单编号.

### 2.4 Exemptions 段落

```markdown
## Exemptions

| Ticket ID | Metric | Reason | Expiry |
| --------- | ------ | ------ | ------ |
| {id}      | {name} | {text} | {date} |
```

- 如果没有豁免, 写 `(none)`.
- `Expiry` 使用 ISO 8601 日期格式.

### 2.5 Attachments 段落

```markdown
## Attachments

| File                  | SHA-256 |
| --------------------- | ------- |
| p99_latency_curve.png | {hash}  |
| rss_curve.png         | {hash}  |
| fd_count_curve.png    | {hash}  |
```

- 附件不存在时写 `(not generated)` 而不是跳过行.
- 曲线 PNG 由 CI 后处理脚本(python/matplotlib)根据 CSV 数据生成, 不在 Rust 测试二进制中生成.

## 3. CI 集成

- SoakReport 作为 CI artifact 归档, 保留 90 天.
- 归档路径: `artifacts/validation/soak-*`.
- SHA-256 哈希写入 CI 日志, 供后续追溯.
