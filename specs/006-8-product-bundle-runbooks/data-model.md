# Data Model(数据模型): 交付包与放行矩阵数据模型

**Branch(分支)**: `main` | **Date(日期)**: 2026-05-19 | **Spec(规格)**: `specs/006-8-product-bundle-runbooks/spec.md`

## 1. 实体定义

### DeliveryBundle(交付包清单)

表示一次对外发布的最小构件集合.

| 字段            | 类型         | 说明                                   |
| --------------- | ------------ | -------------------------------------- |
| `semver`        | `string`     | 与 Cargo.toml version 一致的语义化版本 |
| `artifacts`     | `Artifact[]` | 构件列表, 每个包含 `path` + `sha256`   |
| `release_tag`   | `string`     | Git tag 指针                           |
| `changelog_url` | `string`     | 变更日志 URL                           |

**Artifact(构件)**: `{ path: string, sha256: string, category: "source" | "binary" | "doc" | "config" }`

### RunbookProcedure(值守程序块)

值守手册中的单个可执行步骤.

| 字段                 | 类型                  | 必填 | 说明                        |
| -------------------- | --------------------- | ---- | --------------------------- |
| `step_id`            | `string`              | 是   | 编号, 如 `RBP-001`          |
| `title`              | `string`              | 是   | 步骤标题                    |
| `expected_metrics`   | `map<string, string>` | 是   | 期望的 metrics 字段名和取值 |
| `escalation`         | `string[]`            | 否   | 升级分叉条件列表            |
| `estimated_duration` | `string`              | 是   | 预计耗时, 如 `5min`         |

### ReleaseGateMatrixPointer(放行矩阵指针)

指向 006-2 和 006-7 归档路径的一组 URL.

| 字段                       | 类型             | 必填 | 说明                              |
| -------------------------- | ---------------- | ---- | --------------------------------- |
| `quality_gate_outcome_url` | `string`         | 是   | 006-2 QualityGateOutcome CSV 路径 |
| `soak_report_url`          | `string`         | 是   | 006-7 SoakReport 路径             |
| `chaos_verdicts_url`       | `string`         | 是   | 006-7 混沌判决书路径              |
| `exemption_tickets`        | `ExemptionRef[]` | 否   | 豁免工单引用列表                  |

**ExemptionRef(豁免引用)**: `{ ticket_id: string, gate_id: string, reason: string }`

## 2. 数据流

```text
发布流水线
  |
  +-> cargo package --list            -- 生成 tarball 构件清单
  +-> mdbook build manual/            -- 生成手册 HTML
  +-> cargo test                      -- 单元/集成测试
  +-> [006-7 chaos_suite]            -- 混沌测试 JSON 判决书
  +-> [006-7 soak_suite]             -- 浸泡报告 Markdown
  |
  v
artifacts/release-record.json        -- 发布记录(006-2)
artifacts/quality-gate-outcome.csv   -- 放行矩阵
  |
  v
发布页面 (GitHub Release / HTML)
  ├-> 放行矩阵表格 (CSV -> HTML)
  ├-> 健康自检 JSON schema 引用
  └-> tarball 下载链接
```
