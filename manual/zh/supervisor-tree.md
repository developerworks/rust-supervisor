# 监督树

语言: [English](../en/supervisor-tree.html)

## 声明模型

`SupervisorSpec`(监督器规格) 描述一个 supervisor(监督器)节点. 它包含:

- `path` — 监督器稳定路径
- `strategy` — 重启范围策略 (`OneForOne`, `OneForAll`, `RestForOne`)
- `children` — 声明顺序的子任务规格集合
- `config_version` — 生成此规格的配置版本
- `default_restart_policy`, `default_backoff_policy`, `default_health_policy`, `default_shutdown_policy` — 子任务不覆盖时继承的策略
- `supervisor_failure_limit` — 监督器失败上限, 超限后升级到父级
- `restart_limit` — 可选的监督器级重启限制
- `escalation_policy` — 可选的监督器级升级策略
- `group_strategies` — 分组级策略覆盖
- `group_configs` — 分组级重启预算, 成员资格和隔离配置
- `group_dependencies` — 故障传播的跨分组依赖边
- `severity_defaults` — 每个任务角色的默认严重等级, 用于升级分叉
- `child_strategy_overrides` — 逐子任务策略和治理覆盖
- `dynamic_supervisor_policy` — 运行时 add_child 接受策略
- `control_channel_capacity` — mpsc 命令通道容量
- `event_channel_capacity` — broadcast 事件通道容量

`ChildSpec`(子任务规格) 描述一个 child(子任务). 它包含:

- `id`, `name`, `kind` — 稳定标识和任务类型
- `factory` — 可选的 `Arc<dyn TaskFactory>`(任务工厂), 用于工作子任务
- `restart_policy`, `shutdown_policy`, `health_policy`, `readiness_policy`, `backoff_policy` — 逐子任务策略覆盖
- `dependencies` — 必须在当前子任务之前就绪的子任务标识
- `tags` — 低基数分组标签
- `criticality` — `Critical`(关键) 或 `Optional`(可选)
- `task_role` — 可选的 `TaskRole`(任务角色), 用于选择默认生命周期策略语义
- `sidecar_config` — 可选的边车绑定(角色为 `Sidecar` 时必须)
- `severity` — 可选的显式严重等级覆盖
- `group` — 可选的分组名称, 用于分组级隔离和预算跟踪
- `health_check`, `readiness` — 可选的健康检查/就绪检查配置
- `resource_limits` — 可选的资源限制
- `command_permissions` — 授予此子任务的命令权限
- `environment`, `secrets` — 环境变量和密钥引用

## 树构建

`SupervisorTree::build` 会校验 `SupervisorSpec`(监督器规格), 再把 children(子任务集合)转换成带路径的节点. 每个 child(子任务)路径来自父路径和 `ChildId`(子任务标识).

`SupervisorPath::root` 表示根路径. `SupervisorPath::join` 用于拼接子路径. `SupervisorPath::parent` 用于查找父级路径.

## 启动和关闭顺序

`startup_order` 按声明顺序返回节点. `shutdown_order` 按声明顺序的逆序返回节点. 这个顺序是 Shutdown Without Orphaned Tasks(关闭后不留下孤儿任务) 的基础.

## 重启计划

`restart_execution_plan`(重启执行计划函数) 根据 tree(监督树) 和 `SupervisorSpec`(监督器规格) 解析运行时重启范围. 它把 per-child override(子任务级覆盖), group strategy(分组策略), restart limit(重启次数限制), escalation policy(升级策略) 和 dynamic supervisor policy(动态监督器策略) 放入同一个 plan(计划), 运行时控制循环不需要重复编写策略选择逻辑.

## 注册表

`RegistryStore`(注册表存储)按 child id(子任务标识), supervisor path(监督器路径) 和声明顺序保存 `ChildRuntime`(子任务运行态). 运行时控制和当前状态查询不应该绕过注册表直接访问内部状态.
