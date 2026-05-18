# Contract(契约): ChaosScenario Verdict JSON Schema(混沌场景判决书 JSON 模式)

**Status(状态)**: Draft(草稿) | **Version(版本)**: 1.0.0
**Applies to(适用范围)**: `tests/chaos/verdict.rs` 的序列化实现

## 1. Schema(模式)

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "ChaosScenarioVerdict",
  "description": "Verdict for a single chaos scenario run",
  "type": "object",
  "required": [
    "scenario_id",
    "semver",
    "passed",
    "thresholds",
    "started_at_unix_nanos",
    "duration_ns"
  ],
  "properties": {
    "scenario_id": {
      "type": "string",
      "pattern": "^[a-z][a-z0-9_]*$",
      "description": "Scenario identifier in snake_case"
    },
    "semver": {
      "type": "string",
      "pattern": "^\\d+\\.\\d+\\.\\d+$",
      "description": "Semantic version from CARGO_PKG_VERSION"
    },
    "passed": {
      "type": "boolean",
      "description": "Overall pass/fail for this scenario"
    },
    "thresholds": {
      "type": "object",
      "additionalProperties": {
        "$ref": "#/$defs/ThresholdResult"
      },
      "description": "Per-threshold measurement results"
    },
    "started_at_unix_nanos": {
      "type": "integer",
      "minimum": 0,
      "description": "Unix timestamp in nanoseconds when the scenario started"
    },
    "duration_ns": {
      "type": "integer",
      "minimum": 0,
      "description": "Duration of the scenario in nanoseconds"
    },
    "error": {
      "type": ["string", "null"],
      "description": "Error message if the scenario failed unexpectedly"
    }
  },
  "$defs": {
    "ThresholdResult": {
      "type": "object",
      "required": ["value", "limit", "passed"],
      "properties": {
        "value": {
          "type": "number",
          "description": "Actual measured value"
        },
        "limit": {
          "type": "number",
          "description": "Threshold limit"
        },
        "passed": {
          "type": "boolean",
          "description": "Whether value <= limit (or meets other pass criteria)"
        }
      }
    }
  }
}
```

## 2. 序列化要求

- JSON 输出必须使用 `serde_json::to_string` 或等效工具序列化.
- 顶层字段顺序不重要, 但 `scenario_id` 和 `semver` 应放在前两个字段便于人类阅读.
- `error` 字段: 通过时为 `null`, 失败时填充可读错误信息.
- `thresholds` 的 key 使用 snake_case, 与 ChaosScenario 表的 metric 名称一致.

## 3. 消费端契约

- CI nightly 脚本通过 `jq -e '.passed == true'` 逐条判定 JSON 判决书.
- 任何一条判决书 `passed == false` 则 CI 任务失败.
- `error` 字段不为 `null` 时视为未通过, 等价于 `passed == false`.
