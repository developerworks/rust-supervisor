# Contract(契约): Release Gate Matrix Format(放行矩阵格式)

**Status(状态)**: Draft(草稿) | **Version(版本)**: 1.0.0
**Applies to(适用范围)**: 发布页面和 `artifacts/quality-gate-outcome.csv`

## 1. 数据源

权威数据源为 CSV 文件: `artifacts/quality-gate-outcome.csv`.

## 2. CSV 格式

```csv
gate_id,gate_name,outcome,exemption_ticket,archive_ref
unit-test,Unit Test,passed,,
integration-test,Integration Test,passed,,
property-test,Property Test,passed,,
fuzz-test,Fuzz Test,skipped,TKTK-001,
loom-test,Loom Test,skipped,TKTK-001,
chaos-test,Chaos Test,passed,,specs/006-7-chaos-soak-reliability/
soak-24h,Soak 24h,passed,,artifacts/validation/soak-20260519-120000.md
dep-audit,Dependency Audit,passed,,
sbom,SBOM,passed,,artifacts/sbom/
dry-run,Release Dry Run,passed,,
```

### 字段说明

| 字段               | 类型   | 必填 | 说明                                                   |
| ------------------ | ------ | ---- | ------------------------------------------------------ |
| `gate_id`          | string | 是   | 闸门代号, snake_case                                   |
| `gate_name`        | string | 是   | 人类可读名称                                           |
| `outcome`          | string | 是   | `passed` / `failed` / `waived` / `skipped` / `missing` |
| `exemption_ticket` | string | 否   | 豁免工单编号, 为空表示无豁免                           |
| `archive_ref`      | string | 否   | 归档路径或 URL                                         |

## 3. HTML 渲染规则

- CSV 转换为 HTML `<table>`, 每行一个 `<tr>`.
- `outcome == "passed"` 时单元格显示绿色勾选 ✅.
- `outcome == "failed"` 时单元格显示红色叉 ❌.
- `outcome == "skipped"` 时单元格显示灰色斜杠 — 并附带 exemption_ticket 链接.
- 空白 `exemption_ticket` + `outcome != "passed"` 视为 blocking 缺陷.

## 4. 空白 td 检查

CI 脚本 `scripts/validate-release-matrix.sh` 执行以下检查:

```bash
# 转换 CSV 为 HTML
# 检查是否存在空 <td></td>
# 如果存在空白 td, 退出码非 0
```
