# Contract(契约): Health Self-Check JSON Schema(健康自检 JSON 模式)

**Status(状态)**: Draft(草稿) | **Version(版本)**: 1.0.0
**Applies to(适用范围)**: supervisor 健康自检命令的 stdout 输出

## 1. Schema(模式)

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "HealthSelfCheck",
  "type": "object",
  "required": ["status", "supervisor_version", "uptime_secs", "children"],
  "properties": {
    "status": {
      "type": "string",
      "enum": ["ready", "degraded", "failed"],
      "description": "Overall health status"
    },
    "supervisor_version": {
      "type": "string",
      "pattern": "^\\d+\\.\\d+\\.\\d+$",
      "description": "Semantic version from Cargo.toml"
    },
    "uptime_secs": {
      "type": "integer",
      "minimum": 0,
      "description": "Seconds since supervisor started"
    },
    "children": {
      "type": "object",
      "required": ["total", "running", "failed"],
      "properties": {
        "total": { "type": "integer", "minimum": 0 },
        "running": { "type": "integer", "minimum": 0 },
        "failed": { "type": "integer", "minimum": 0 }
      }
    },
    "dashboard_link": {
      "type": "string",
      "enum": ["connected", "disconnected"],
      "description": "Dashboard IPC link status"
    }
  }
}
```

## 2. 输出示例

```json
{
  "status": "ready",
  "supervisor_version": "0.1.2",
  "uptime_secs": 3600,
  "children": { "total": 5, "running": 5, "failed": 0 },
  "dashboard_link": "connected"
}
```

## 3. 消费端契约

- 调用者通过 `jq -r '.status'` 获取健康状态.
- `status == "ready"` 表示可以接受流量.
- `status == "degraded"` 表示部分 child 失败但监督器仍在运行.
- `status == "failed"` 表示监督器无法正常运作.
