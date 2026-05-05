# 监督树

## 声明模型

`SupervisorSpec`(监督器规格) 描述一个 supervisor(监督器)节点. 它包含 `path`, `strategy`, `children`, `config_version`, 默认重启策略, 默认退避策略, 默认健康策略, 默认关闭策略和 supervisor-level fuse(监督器级熔断)限制.

`ChildSpec`(子任务规格) 描述一个 child(子任务). 它包含 `id`, `name`, `kind`, `factory`, `restart_policy`, `shutdown_policy`, `health_policy`, `readiness_policy`, `backoff_policy`, `dependencies`, `tags` 和 `criticality`.

## 树构建

`SupervisorTree::build` 会校验 `SupervisorSpec`(监督器规格), 再把 children(子任务集合)转换成带路径的节点. 每个 child(子任务)路径来自父路径和 `ChildId`(子任务标识).

`SupervisorPath::root` 表示根路径. `SupervisorPath::join` 用于拼接子路径. `SupervisorPath::parent` 用于查找父级路径.

## 启动和关闭顺序

`startup_order` 按声明顺序返回节点. `shutdown_order` 按声明顺序的逆序返回节点. 这个顺序是 Shutdown Without Orphaned Tasks(关闭后不留下孤儿任务) 的基础.

## 注册表

`RegistryStore`(注册表存储)按 child id(子任务标识), supervisor path(监督器路径) 和声明顺序保存 `ChildRuntime`(子任务运行态). 运行时控制和当前状态查询不应该绕过注册表直接访问内部状态.
