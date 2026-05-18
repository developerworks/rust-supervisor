# Contract(契约): Child Declaration Schema(子任务声明方案)

本文件约束 `006-6-config-dynamic-children` 交付时 YAML 配置文件中 `ChildDeclaration` 的字段语义与序列化格式. Rust 类型实现必须与本契约字段同名.

## 1. YAML 顶层结构

```yaml
supervisor:
  path: "/"
  strategy: one_for_one
  children:
    - name: "child-a"
      kind: async_worker
      criticality: critical
      restart_policy: permanent
      dependencies: ["child-b"]
      health_check:
        check_interval_secs: 10
        timeout_secs: 5
        max_retries: 3
      readiness:
        check_interval_secs: 5
        timeout_secs: 3
      resource_limits:
        max_memory_mb: 256
        max_cpu_percent: 50
      command_permissions:
        allow_shutdown: false
        allow_restart: true
        allowed_signals: ["SIGTERM", "SIGUSR1"]
      environment:
        - name: "DATABASE_URL"
          secret_ref: "${DB_URL}"
        - name: "LOG_LEVEL"
          value: "debug"
      secrets:
        - name: "DB_URL"
          key: "production/database/url"
          required: true
      restart_budget:
        window_secs: 60
        max_burst: 5
    - name: "child-b"
      kind: async_worker
      criticality: optional
```

## 2. 字段类型与校验规则

### 2.1 必需字段

- `name`: 非空字符串, 匹配 `^[a-zA-Z_][a-zA-Z0-9_-]*$`, 在 `children` 列表中唯一.

### 2.2 可选字段(默认值)

| 字段             | 类型     | 默认值         | 说明                                                    |
| ---------------- | -------- | -------------- | ------------------------------------------------------- |
| `kind`           | string   | `async_worker` | 可选值: `async_worker`, `blocking_worker`, `supervisor` |
| `criticality`    | string   | `optional`     | 可选值: `critical`, `optional`                          |
| `restart_policy` | string   | `permanent`    | 可选值: `permanent`, `transient`, `temporary`           |
| `dependencies`   | string[] | `[]`           | 引用 `children` 列表中其他 `name`                       |

### 2.3 嵌套对象

健康检查、资源限制等嵌套对象的所有字段均有默认值, 在 YAML 中可整体省略.

## 3. 校验错误格式

校验错误必须返回结构化错误, 包含:

```json
{
  "error": "validation_failed",
  "field_path": "supervisor.children[0].secrets[0]",
  "reason": "secret name 'DB_URL' uses invalid characters",
  "hint": "Secret names must match ^[A-Za-z_][A-Za-z0-9_]*$"
}
```

- `field_path`: JSON Pointer 格式的字段路径.
- `reason`: 人读的失败原因.
- `hint`: 可操作的建议(可选).
