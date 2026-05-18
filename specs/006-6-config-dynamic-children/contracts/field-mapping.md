# Contract(契约): ChildDeclaration → ChildSpec 字段映射

本文件定义 `ChildDeclaration`(YAML 声明) 到 `ChildSpec`(运行时规范) 的字段级映射, 用于 golden YAML 一致性测试(T010)中的逐字段比对.

## 映射表

| ChildDeclaration 字段 | ChildSpec 字段                | 转换逻辑                                                         |
| --------------------- | ----------------------------- | ---------------------------------------------------------------- |
| `name`                | `name` + `id`                 | `name` 直接复制; `id` 通过 `ChildId::from(name)` 生成            |
| `kind`                | `kind`                        | 直接复制                                                         |
| `criticality`         | `criticality`                 | 直接复制                                                         |
| `restart_policy`      | `restart_policy`              | 直接复制                                                         |
| `dependencies`        | `dependencies`                | `Vec<String>` → `Vec<ChildId>` 逐个转换                          |
| `health_check`        | `health_policy`               | `HealthCheckConfig` → `HealthPolicy` 转换(见下方映射)            |
| `readiness`           | `readiness_policy`            | `ReadinessConfig` → `ReadinessPolicy` 转换(见下方映射)           |
| `resource_limits`     | `resource_limits`(新字段)     | 直接复制(新字段, 待添加到 ChildSpec)                             |
| `command_permissions` | `command_permissions`(新字段) | 直接复制(新字段, 待添加到 ChildSpec)                             |
| `environment`         | `environment`(新字段)         | 直接复制(新字段, 待添加到 ChildSpec)                             |
| `secrets`             | `secrets`(新字段)             | 直接复制(新字段, 待添加到 ChildSpec)                             |
| `restart_budget`      | 不直接对应                    | 转换为 `Option<RestartBudgetConfig>`, 存储于 `SupervisorSpec` 级 |

## 嵌套类型映射

### HealthCheckConfig → HealthPolicy

| HealthCheckConfig 字段 | HealthPolicy 字段    | 转换逻辑                                      |
| ---------------------- | -------------------- | --------------------------------------------- |
| `check_interval_secs`  | `heartbeat_interval` | `Duration::from_secs(check_interval_secs)`    |
| `timeout_secs`         | `stale_after`        | `Duration::from_secs(timeout_secs)`           |
| `max_retries`          | —                    | 不映射到 HealthPolicy(仅影响重试, 非心跳超时) |

### ReadinessConfig → ReadinessPolicy

| ReadinessConfig 字段  | ReadinessPolicy 字段 | 转换逻辑                             |
| --------------------- | -------------------- | ------------------------------------ |
| `check_interval_secs` | —                    | 不映射(ReadinessPolicy 另有信号机制) |
| `timeout_secs`        | —                    | 不映射                               |

## 比对策略(T010 测试)

1. **序列化格式**: 两侧均使用 `serde_json::to_string` 生成 JSON.
2. **比对范围**: 仅比对映射表中列出的字段. ChildSpec 中 ChildDeclaration 不存在的字段(如 `factory`, `shutdown_policy`, `backoff_policy`, `tags`, `work_role`, `sidecar_config`, `severity`, `group`) 不参与比对.
3. **差异计数定义**: 对映射表中的每个 ChildDeclaration 字段, 从 ChildSpec 的 JSON 中提取对应字段值(按映射表路径), 进行严格相等比较. 不匹配的字段数计为差异计数.
4. **预期结果**: 差异计数 = 0.
5. **依赖方向约定**: `dependencies: ["A"]` 表示"当前 child 依赖 A, A 必须在当前 child 之前启动".
