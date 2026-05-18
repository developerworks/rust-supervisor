# Data Model(数据模型): 配置声明与动态子任务治理

**Branch(分支)**: `006-6-config-dynamic-children` | **Date(日期)**: 2026-05-19
**Source(来源)**: `specs/006-6-config-dynamic-children/spec.md` + `research.md`

## Entities(实体)

### ChildDeclaration(子任务声明)

YAML 文件或运行时载荷中的子任务声明, 用于静态加载或动态追加.

| Field(字段)           | Type(类型)                    | Required(必填) | Default(默认值) | Description(说明)                                                                        |
| --------------------- | ----------------------------- | -------------- | --------------- | ---------------------------------------------------------------------------------------- |
| `name`                | `String`                      | 是             | —               | 子任务唯一标识, 用于 ChildId                                                             |
| `kind`                | `TaskKind`                    | 否             | `async_worker`  | 任务类型(异步/阻塞/supervisor)                                                           |
| `criticality`         | `Criticality`                 | 否             | `optional`      | 关键程度                                                                                 |
| `restart_policy`      | `RestartPolicy`               | 否             | `permanent`     | 重启策略                                                                                 |
| `dependencies`        | `Vec<String>`                 | 否             | `[]`            | 依赖的子任务名列表                                                                       |
| `health_check`        | `Option<HealthCheckConfig>`   | 否             | `None`          | 健康检查配置                                                                             |
| `readiness`           | `Option<ReadinessConfig>`     | 否             | `None`          | 就绪检查配置                                                                             |
| `resource_limits`     | `Option<ResourceLimits>`      | 否             | `None`          | 资源限制                                                                                 |
| `command_permissions` | `Option<CommandPermissions>`  | 否             | `None`          | 命令权限                                                                                 |
| `environment`         | `Vec<EnvVar>`                 | 否             | `[]`            | 环境变量                                                                                 |
| `secrets`             | `Vec<SecretRef>`              | 否             | `[]`            | 密钥引用                                                                                 |
| `restart_budget`      | `Option<RestartBudgetConfig>` | 否             | `None`          | 重启预算覆盖(SupervisorSpec 级, 非 ChildDeclaration 字段; 见 contracts/field-mapping.md) |

#### HealthCheckConfig(健康检查配置)

| Field(字段)           | Type(类型) | Required(必填) | Default(默认值) | Description(说明) |
| --------------------- | ---------- | -------------- | --------------- | ----------------- |
| `check_interval_secs` | `u64`      | 否             | 10              | 检查间隔(秒)      |
| `timeout_secs`        | `u64`      | 否             | 5               | 检查超时(秒)      |
| `max_retries`         | `u32`      | 否             | 3               | 最大重试次数      |

#### ResourceLimits(资源限制)

| Field(字段)            | Type(类型)    | Required(必填) | Default(默认值) | Description(说明) |
| ---------------------- | ------------- | -------------- | --------------- | ----------------- |
| `max_memory_mb`        | `Option<u64>` | 否             | `None`          | 最大内存(MB)      |
| `max_cpu_percent`      | `Option<u8>`  | 否             | `None`          | 最大 CPU 百分比   |
| `max_file_descriptors` | `Option<u64>` | 否             | `None`          | 最大文件描述符数  |

#### CommandPermissions(命令权限)

| Field(字段)       | Type(类型)    | Required(必填) | Default(默认值) | Description(说明)               |
| ----------------- | ------------- | -------------- | --------------- | ------------------------------- |
| `allow_shutdown`  | `bool`        | 否             | `false`         | 允许 child 触发 supervisor 关闭 |
| `allow_restart`   | `bool`        | 否             | `false`         | 允许 child 请求自身重启         |
| `allowed_signals` | `Vec<String>` | 否             | `["SIGTERM"]`   | child 可发送的信号列表          |

#### EnvVar(环境变量)

| Field(字段)  | Type(类型)       | Required(必填) | Description(说明)                        |
| ------------ | ---------------- | -------------- | ---------------------------------------- |
| `name`       | `String`         | 是             | 环境变量名                               |
| `value`      | `String`         | 否             | 环境变量值(与 secret_ref 互斥)           |
| `secret_ref` | `Option<String>` | 否             | 密钥引用 `${SECRET_NAME}`(与 value 互斥) |

#### SecretRef(密钥引用)

| Field(字段) | Type(类型) | Required(必填) | Description(说明)                    |
| ----------- | ---------- | -------------- | ------------------------------------ |
| `name`      | `String`   | 是             | 密钥名                               |
| `key`       | `String`   | 是             | 密钥路径                             |
| `required`  | `bool`     | 否             | 是否必需(true 时 vault 离线视为拒绝) |

### PendingChild(待提交子任务)

add_child 事务暂存区中的子任务记录.

| Field(字段)             | Type(类型)         | Description(说明)  |
| ----------------------- | ------------------ | ------------------ |
| `transaction_id`        | `Uuid`             | 唯一事务编号       |
| `declaration`           | `ChildDeclaration` | 原始声明           |
| `child_spec`            | `Box<ChildSpec>`   | 转换后的运行时规范 |
| `phase`                 | `Phase`            | 当前事务阶段       |
| `created_at_unix_nanos` | `u128`             | 创建时间戳         |

#### Phase(事务阶段枚举)

| Variant(变体)  | Description(说明)  |
| -------------- | ------------------ |
| `Parsed`       | 解析完成           |
| `Validated`    | 校验通过           |
| `Registered`   | 注册到拓扑         |
| `Started`      | child 已拉起       |
| `Audited`      | 审计已持久化       |
| `Committed`    | 事务完成           |
| `Compensating` | 事务失败, 正在补偿 |
| `Compensated`  | 补偿完成           |

### CompensatingRecord(补偿记录)

审计通道中的补偿段落实体.

| Field(字段)             | Type(类型)          | Description(说明)                                                |
| ----------------------- | ------------------- | ---------------------------------------------------------------- |
| `transaction_id`        | `Uuid`              | 唯一事务编号                                                     |
| `operation`             | `String`            | 操作类型("add_child")                                            |
| `state`                 | `CompState`         | 状态枚举: `pending` / `committed` / `compensated`                |
| `child_name`            | `String`            | 子任务名                                                         |
| `declaration_hash`      | `String`            | ChildDeclaration 的 SHA-256                                      |
| `error`                 | `Option<CompError>` | 失败原因枚举(见下方)                                             |
| `correlation_id`        | `Option<String>`    | 006-5 关联标识, 用于关联运行时事件链(可选, 仅在运行时可用时填充) |
| `child_id`              | `Option<String>`    | 运行时分配的 ChildId(如果已分配), 用于重启恢复时精确锁定拓扑节点 |
| `created_at_unix_nanos` | `u128`              | 创建时间戳                                                       |

#### CompState(补偿状态枚举)

| Variant(变体) | Description(说明) |
| ------------- | ----------------- |
| `pending`     | 事务正在进行中    |
| `committed`   | 事务已提交        |
| `compensated` | 补偿已完成        |

#### CompError(补偿错误枚举)

| Variant(变体)           | Description(说明)                          |
| ----------------------- | ------------------------------------------ |
| `validation_failed`     | 密钥占位符语法校验失败                     |
| `runtime_secret_miss`   | 密钥语法合法但 vault 离线或密钥缺失        |
| `registration_failed`   | 注册到拓扑时失败(如名称冲突)               |
| `startup_failed`        | 拉起 child 时失败                          |
| `audit_write_failed`    | 审计持久化写入失败                         |
| `compensation_required` | 断电等非优雅中断后恢复时标记的通用补偿原因 |

## Relationships(关系)

```
SupervisorConfig
    ├── 1:N ──► ChildDeclaration(静态 YAML 中的 children 声明)
    └── 1:1 ──► SupervisorSpec(转换后的运行时规范)

ConfigState
    ├── 1:N ──► ChildSpec(运行时子任务集合)
    ├── 0:N ──► PendingChild(待提交事务)
    └── 1:1 ──► spec_hash: String(当前快照 SHA-256)

PendingChild
    ├── 1:1 ──► ChildDeclaration(原始声明)
    ├── 1:1 ──► ChildSpec(转换后规范)
    └── 1:1 ──► Phase(事务阶段)

CompensatingRecord(审计通道)
    ├── 1:1 ──► transaction_id
    └── 0:1 ──► PendingChild(关联的待提交事务)
```

## Validation Rules(校验规则)

1. **子任务名唯一**: 同一 supervisor 下所有 child 的 name 必须唯一. 冲突时拒绝加载/追加.
2. **依赖存在性**: dependencies 中的每个子任务名必须在同层 children 中存在. 不存在时拒绝.
3. **DAG 无环**: 依赖图必须是有向无环图. 环路检测使用 Kahn 算法, 检测到时拒绝并在错误消息中列出环路节点.
4. **密钥占位符语法**: secrets 和 env.secret*ref 中的 `${SECRET_NAME}` 必须匹配 `^\$\{[A-Za-z*][A-Za-z0-9_]\*\}$`. 不匹配时触发 validation_failed.
5. **值互斥**: EnvVar 的 value 和 secret_ref 不能同时设置. 同时设置时拒绝.
6. **资源限制兼容性**: 宿主机内核不支持的 resource_limit(如 macOS 上设置 max_file_descriptors)时按 `ignore` 策略处理(静默忽略不支持的字段, 不拒绝), 与全局默认设置一致.
7. **事务不可重入**: 正在执行 add_child 事务时, 新的 add_child 请求返回 `Err(TransactionInProgress)`.
8. **审计容量保证**: 审计通道容量必须 ≥ 2 × max_children(1000) + SC-002 压力脚本条目数. 超出容量时 add_child 返回 `Err(AuditStorageFailure)`, 不静默覆盖. 默认容量 ≥ 8192.

## State Transitions(状态迁移)

### add_child 事务阶段

```
Parsed → Validated → Registered → Started → Audited → Committed
    ↓          ↓           ↓          ↓         ↓
    └──────────┴───────────┴──────────┴─────────┘──→ Compensating → Compensated
```

任一步失败时进入 Compensating 阶段, 执行回滚: 已注册的 child 从拓扑中移除, 已拉起的 child 发送停止信号(通过 CancellationToken), 审计写入 CompensatingRecord. 补偿完成后进入 Compensated.

### 加载后恢复流程(重启)

```
启动 → 加载 YAML → 枚举审计中的 compensating records
       ↓
       如果有 pending/compensating 记录:
       └→ 按 declaration_hash 重建 ChildDeclaration → 比对拓扑
            ├ 一致 → 标记为 committed (事务已完成, 审计已写入)
            └ 不一致 → 执行 compensating (拓扑回退到调用前值)
       ↓
       进入正常运行
```
