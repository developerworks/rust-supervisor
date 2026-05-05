# 配置模型

## 配置入口

配置入口是 `rust_supervisor::config::loader::load_config_state`. 它只接受 YAML(数据序列化格式)主配置文件, 示例路径是 `examples/config/supervisor.yaml`.

当前配置形状包含四组数据: `supervisor`, `policy`, `shutdown` 和 `observability`. 它们分别进入 `SupervisorRootConfig`, `PolicyConfig`, `ShutdownConfig` 和 `ObservabilityConfig`.

## 配置状态

`SupervisorConfig`(监督器配置) 是文件反序列化后的形状. `ConfigState`(配置状态) 是校验后的不可变状态. 运行时不应该在其它模块里保存运行时可调常量.

`ConfigState::to_supervisor_spec` 会派生 `SupervisorSpec`(监督器规格). 当前实现用配置值填充 supervision strategy(监督策略),策略默认值,关闭预算,健康检查时间和可观测性容量.

## 错误边界

配置加载失败会返回 `SupervisorError::FatalConfig`. 这些情况会拒绝启动:

- 配置文件不是 YAML(数据序列化格式).
- 文件无法读取.
- YAML(数据序列化格式)无法解析成 `SupervisorConfig`.
- supervision strategy(监督策略) 不是 `OneForOne`, `OneForAll` 或 `RestForOne`.
- 数值为零.
- 初始退避大于最大退避.
- jitter(抖动)比例不在零到一之间.

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
