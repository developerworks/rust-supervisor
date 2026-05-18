# 配置模型和结构模式

语言: [English](../en/configuration.html)

## 配置入口

配置入口是 `rust_supervisor::config::loader::load_config_from_yaml_file`. 它只接受 YAML(数据序列化格式)主配置文件, 示例路径是 `examples/config/supervisor.yaml`.

当前配置形状包含四组数据: `supervisor`, `policy`, `shutdown` 和 `observability`. 它们分别进入 `SupervisorRootConfig`(监督器根配置), `PolicyConfig`(策略配置), `ShutdownConfig`(关闭配置) 和 `ObservabilityConfig`(可观测性配置).

## 配置状态

`rust_supervisor::config::configurable::SupervisorConfig` 是公开 root configuration struct(根配置结构体). 它支持 `confique::Config`(配置派生), `schemars::JsonSchema`(结构模式生成特征), `Serialize`(序列化) 和 `Deserialize`(反序列化). 使用者可以用同一个模型完成 YAML(数据序列化格式) 加载, template generation(模板生成) 和 schema generation(结构模式生成).

`ConfigState`(配置状态) 是校验后的不可变状态. 运行时不应该在其它模块里保存运行时可调常量.

`ConfigState::to_supervisor_spec` 会派生 `SupervisorSpec`(监督器规格). 当前实现用配置值填充 supervision strategy(监督策略),策略默认值,关闭预算,健康检查时间和可观测性容量.

## 模板边界

官方 template(模板) 是 `examples/config/supervisor.template.yaml`. 它默认保持单个 YAML(数据序列化格式) 文件, 并覆盖 `supervisor`, `policy`, `shutdown` 和 `observability`.

本 crate(包) 不会在公开配置结构体, 官方 schema(结构模式) 或官方 template(模板) 中添加 `x-tree-split`(树形拆分扩展). 如果使用者项目需要拆分配置文件, 可以在自己的项目中包装或复用 `SupervisorConfig`(监督器配置), 并自行决定 tree split layout(树形拆分布局).

## 错误边界

配置加载失败会返回 `SupervisorError::FatalConfig`. 这些情况会拒绝启动:

- 配置文件不是 YAML(数据序列化格式).
- 文件无法读取.
- YAML(数据序列化格式)无法解析成 `SupervisorConfig`.
- supervision strategy(监督策略) 不是 `OneForOne`, `OneForAll` 或 `RestForOne`.
- 数值为零.
- 初始退避大于最大退避.
- jitter(抖动)比例不在零到一之间.

`Supervisor::start_from_config_file` 会在创建 runtime channel(运行时通道) 或派生 control loop(控制循环) 之前拒绝非法配置.

## 示例配置

```yaml
supervisor:
  strategy: OneForAll
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
shutdown:
  graceful_timeout_ms: 5000
  abort_wait_ms: 1000
observability:
  event_journal_capacity: 256
  metrics_enabled: true
  audit_enabled: true
```

## 密钥占位符

配置值中引用敏感信息的字段使用 `${SECRET_NAME}` 占位符格式.
在启动 supervisor(监督器) 前, 需要将这些占位符替换为实际的环境变量值或密钥管理方案的值. 示例:

```yaml
ipc:
  tls_cert_path: "${IPC_TLS_CERT}"
  tls_key_path: "${IPC_TLS_KEY}"
```

supervisor(监督器) 本身不解析运行时占位符; 替换必须在配置加载前完成(例如通过 `envsubst` 或部署流水线).

## 升级

本版本不支持原地升级. 如需升级, 请部署新版本的全新实例, 并通过外部 IPC(进程间通信) 接口迁移状态.
