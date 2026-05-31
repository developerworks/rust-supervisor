# 配置模型和结构模式

语言: [English](../en/configuration.html)

## 配置入口

配置入口是 `rust_supervisor::config::loader::load_config_from_yaml_file`. 它只接受 YAML(数据序列化格式)主配置文件, 示例路径是 `examples/config/supervisor.yaml`.

配置结构体 `SupervisorConfig`(监督器配置)包含以下顶层组:

| 组 | 类型 | 描述 |
|---|---|---|
| `include` | `Vec<PathBuf>` | `rust-config-tree`(集中配置树) 包含的附加配置文件 |
| `supervisor` | `SupervisorRootConfig` | 根监督器策略 |
| `policy` | `PolicyConfig` | 重启, 退避, 心跳, failure window(失败窗口), restart budget(重启预算), meltdown fuse(故障熔断)和 supervision pipeline(监督流水线)容量 |
| `shutdown` | `ShutdownConfig` | 优雅关闭超时和强制终止等待预算 |
| `observability` | `ObservabilityConfig` | 事件日志容量和指标/审计开关 |
| `audit` | `AuditConfig` | 审计存储后端, JSON Lines(逐行 JSON)文件路径和写入失败策略 |
| `backpressure` | `BackpressureConfig` | 可观测性 subscriber(订阅者) 队列的背压策略, 阈值, 窗口和审计通道容量 |
| `groups` | `GroupsConfigSection` | group(分组)名称与 group-level restart budget(分组级重启预算); 成员由 `children[].group` 声明; 支持 split 文件 `groups.yaml` |
| `group_strategies` | `Vec<GroupStrategyConfig>` | group(分组)级监督策略, 重启限制和升级策略 |
| `group_dependencies` | `Vec<GroupDependencyConfig>` | group(分组)之间的故障传播关系 |
| `child_strategy_overrides` | `Vec<ChildStrategyOverrideConfig>` | child(子任务)级监督策略, 重启限制和升级策略 |
| `severity_defaults` | `Vec<SeverityDefaultConfig>` | TaskRole(任务角色)到 SeverityClass(严重级别)的默认映射 |
| `dashboard` | `Option<DashboardIpcConfig>` | 可选的 dashboard IPC(看板进程间通信) socket(仅 Unix) |
| `children` | `ChildrenConfigSection` | 声明式子任务规格; YAML 中为数组; 支持 split 文件 `children.yaml` |

## 配置状态

`rust_supervisor::config::configurable::SupervisorConfig` 是公开 root configuration struct(根配置结构体). 它支持 `confique::Config`(配置派生), `schemars::JsonSchema`(结构模式生成特征), `Serialize`(序列化) 和 `Deserialize`(反序列化). 使用者可以用同一个模型完成 YAML(数据序列化格式) 加载, template generation(模板生成) 和 schema generation(结构模式生成).

`ConfigState`(配置状态) 是校验后的不可变状态. 运行时不应该在其它模块里保存运行时可调常量.

`ConfigState::to_supervisor_spec` 会派生 `SupervisorSpec`(监督器规格). 当前实现用配置值填充 supervision strategy(监督策略), 策略默认值, 关闭预算, 健康检查时间, 可观测性容量, backpressure(背压)策略, dynamic supervisor(动态监督器)策略, restart budget(重启预算), failure window(失败窗口), meltdown fuse(故障熔断), supervision pipeline(监督流水线)容量, group(分组)策略和 child(子任务)策略覆盖.

## 模板与拆分配置

官方单文件 template(模板) 是 `examples/config/supervisor.template.yaml`.

`groups` 和 `children` 使用透明数组 Section(配置段). 它们可以写在根文件里, 也可以通过 `include` 拆到 `groups.yaml` 和 `children.yaml`. split 文件只写数组体, 不写 `items:`.

- 详细说明: [拆分配置与透明数组 Section](split-config.md)
- 生成模板目录: `config/supervisor_config/`
- 可运行 split 示例: `cargo run --example split_config_supervisor`

生成模板与 schema(结构定义). CLI(命令行) 子命令在顶层, 没有 `config` 前缀. `--config` 是 `run` 与 `validate-config` 子命令的参数; `generate-template` 与 `generate-schema` 以默认路径 `examples/config/supervisor.yaml` 作为模板来源:

```bash
cargo run -- run --config examples/config/supervisor.yaml

cargo run -- validate-config --config examples/config/split/supervisor.yaml

cargo run -- generate-template \
  --output config/supervisor_config/supervisor_config.example.yaml

cargo run -- generate-schema \
  --output config/supervisor_config/supervisor.schema.json
```

## 错误边界

配置加载失败会返回 `SupervisorError::FatalConfig`. 这些情况会拒绝启动:

顶层检查:
- 配置文件不是 YAML(数据序列化格式).
- 文件无法读取.
- YAML(数据序列化格式)无法解析成 `SupervisorConfig`.
- supervision strategy(监督策略) 不是 `OneForOne`, `OneForAll` 或 `RestForOne`.
- 数值为零.
- 初始退避大于最大退避.
- jitter(抖动)比例不在零到一之间.
- `policy.restart_budget.window_secs`, `policy.restart_budget.max_burst` 或 `policy.restart_budget.recovery_rate_per_sec` 不合法.
- `policy.failure_window.window_secs`, `policy.failure_window.max_count` 或 `policy.failure_window.threshold` 不合法.
- `policy.meltdown.*` 中的窗口或阈值为零.
- `policy.supervision_pipeline.*` 中的容量或并发重启限制为零.
- `supervisor.dynamic_supervisor.child_limit` 为零.
- backpressure(背压) 的 `warn_threshold_pct` 不是 1 到 100 之间的数值.
- backpressure(背压) 的 `critical_threshold_pct` 不是 1 到 100 之间的数值.
- backpressure(背压) 的 `warn_threshold_pct` 大于或等于 `critical_threshold_pct`.
- backpressure(背压) 的 `window_secs` 或 `audit_channel_capacity` 为零.

子任务声明检查:
- Child ID(子任务标识)和 name(名称)不能为空.
- tags(标签)不能为空.
- `kind: Supervisor` 的子任务不能有 factory(工厂); `kind: AsyncWorker` 或 `kind: BlockingWorker` 必须有 factory(工厂).
- Sidecar(辅助进程)任务角色需要 `sidecar_config`, 反之亦然.
- 依赖循环会被拒绝.
- 分组成员只在 `children[].group` 声明; `child.group` 引用的 group(分组)名称必须在 `groups` 中存在.
- `group_strategies` 和 `group_dependencies` 引用的 group(分组)名称必须存在.
- `child_strategy_overrides` 引用的 child(子任务)名称必须存在.
- `severity_defaults` 不能为同一个 TaskRole(任务角色)声明多次默认值.

IPC(进程间通信)检查(当 `dashboard.enabled = true` 时):
- `target_id` 不能为空.
- `path` 是必填字段且必须是绝对路径.
- 注册 `relay_registration_path` 是必填字段且必须是绝对路径.
- `lease_seconds` 必须大于零.
- `heartbeat_interval_seconds` 必须为正且小于 `lease_seconds`.

`Supervisor::start_from_config_file` 会在创建 runtime channel(运行时通道) 或派生 control loop(控制循环) 之前拒绝非法配置.

## 示例配置

```yaml
supervisor:
  strategy: OneForAll
  escalation_policy: escalate_to_parent
  control_channel_capacity: 256
  event_channel_capacity: 256
  dynamic_supervisor:
    enabled: true
    child_limit: 16
policy:
  child_restart_limit: 10
  child_restart_window_ms: 60000
  supervisor_failure_limit: 30
  supervisor_failure_window_ms: 60000
  initial_backoff_ms: 100
  max_backoff_ms: 5000
  jitter_ratio: 0.10
  heartbeat_interval_ms: 1000
  stale_after_ms: 3000
  restart_budget:
    window_secs: 60
    max_burst: 10
    recovery_rate_per_sec: 0.50
  failure_window:
    mode: time_sliding
    window_secs: 60
    max_count: 5
    threshold: 5
  meltdown:
    child_max_restarts: 3
    child_window_secs: 10
    group_max_failures: 5
    group_window_secs: 30
    supervisor_max_failures: 10
    supervisor_window_secs: 60
    reset_after_secs: 120
  supervision_pipeline:
    journal_capacity: 100
    subscriber_capacity: 10
    concurrent_restart_limit: 5
shutdown:
  graceful_timeout_ms: 5000
  abort_wait_ms: 1000
observability:
  event_journal_capacity: 256
  metrics_enabled: true
  audit_enabled: true
audit:
  enabled: true
  backend: memory
  failure_strategy: fail_closed
  max_defer_queue: 1000
backpressure:
  strategy: alert_and_block
  warn_threshold_pct: 80
  critical_threshold_pct: 95
  window_secs: 30
  audit_channel_capacity: 1024
groups:
  - name: core
    children:
      - api
    budget:
      window_secs: 60
      max_burst: 10
      recovery_rate_per_sec: 0.50
  - name: upstream
    children: []
group_strategies:
  - group: core
    strategy: OneForOne
    restart_limit:
      max_restarts: 5
      window_ms: 60000
    escalation_policy: quarantine_scope
group_dependencies:
  - from_group: core
    to_group: upstream
    propagation: Full
child_strategy_overrides:
  - child_id: api
    strategy: RestForOne
    restart_limit:
      max_restarts: 3
      window_ms: 30000
    escalation_policy: shutdown_tree
severity_defaults:
  - task_role: service
    severity: Critical
children:
  - name: api
    kind: supervisor
    criticality: critical
    tags:
      - core
    task_role: supervisor
    severity: Critical
    group: core
    restart_policy: transient
dashboard:
  enabled: true
  target_id: payments-worker-a
  path: /tmp/rust-supervisor-demo/payments-worker-a.sock
  permissions: "0600"
  bind_mode: replace_stale
  registration:
    enabled: true
    relay_registration_path: /tmp/rust-supervisor-demo/dashboard-relay-registration.sock
    display_name: "payments worker a"
    lease_seconds: 30
    registration_heartbeat_interval_seconds: 15
```

## 密钥占位符

配置值中引用敏感信息的字段使用 `${SECRET_NAME}` 占位符格式.
在启动 supervisor(监督器) 前, 需要将这些占位符替换为实际的环境变量值或密钥管理方案的值. 示例:

```yaml
dashboard:
  security_config:
    peer_identity:
      allowed_uids: [ "${SUPERVISOR_UID}" ]
```

`dashboard.security_config` 不携带审计设置. IPC(进程间通信)审计持久化使用顶层 `audit` 配置, 因此全局只有一个权威 `AuditConfig`(审计配置).

supervisor(监督器) 本身不解析运行时占位符; 替换必须在配置加载前完成(例如通过 `envsubst` 或部署流水线).

TLS(传输层安全协议)由 relay(中继)层(`rust-supervisor-relay`)通过 `wss://` 处理. supervisor(监督器)目标进程只暴露本地 Unix domain socket(Unix 域套接字), 不终结 TLS(传输层安全协议).

## 升级

本版本不支持原地升级. 如需升级, 请部署新版本的全新实例, 并通过外部 IPC(进程间通信) 接口迁移状态.
