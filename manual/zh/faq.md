# 常见问题(FAQ)

语言: [English](../en/faq.html)

## 基础概念

### ChildDeclaration(子任务声明)和 ChildSpec(子任务规格)到底有什么区别?

`ChildDeclaration`(子任务声明)是 YAML(数据序列化格式)配置和 `add_child`(追加子任务)RPC(远程过程调用)载荷中使用的**输入模型**, 侧重可序列化、可校验的声明. `ChildSpec`(子任务规格)是运行时真正用来注册、启动、重启的**运行时模型**, 包含已生成的 `ChildId`(子任务标识)、`Arc<dyn TaskFactory>`(任务工厂)和已物化的策略对象.

详细说明见 [ChildSpec 与 ChildDeclaration](child-spec.md).

### Supervisor(监督器)启动后有哪几种入口方法?

`Supervisor`(监督器)结构体提供 3 个入口方法:

| 方法                                         | 输入                           | 适用场景                   |
| -------------------------------------------- | ------------------------------ | -------------------------- |
| `Supervisor::start(spec)`                    | `SupervisorSpec`(已构建的规格) | 编程式启动                 |
| `Supervisor::start_from_config_state(state)` | `ConfigState`(已验证的配置)    | 从配置加载器获得配置后启动 |
| `Supervisor::start_from_config_file(path)`   | YAML 文件路径                  | 从 YAML 文件直接启动       |

三个方法最终都汇聚到 `start_with_policy()`, 执行验证、创建通道、启动控制循环并返回 `SupervisorHandle`(监督器句柄).

### Shutdown Without Orphaned Tasks(关闭后不留下孤儿任务)是什么意思?

这是本项目的核心关闭目标. 当根 supervisor(监督器)完成关闭后, runtime(运行时)下不能留下任何孤儿 task(任务). 实现方式是通过四阶段关闭协议(request stop -> graceful drain -> abort stragglers -> reconcile), 以及按声明顺序逆序关闭所有子任务, 确保每个 child(子任务)都被妥善终止.

## 配置相关

### YAML 配置中的 `children` 字段支持哪些子任务字段?

`children` 是 `Vec<ChildDeclaration>`(子任务声明列表)类型. 每个声明支持以下 9 类字段:

| 类别     | 字段                  | 说明                                                     |
| -------- | --------------------- | -------------------------------------------------------- |
| 标识     | `name`                | 子任务名称, 必填, 不能为空                               |
| 类型     | `kind`                | `async_worker`, `blocking_worker` 或 `supervisor`        |
| 关键性   | `criticality`         | `critical`(关键)或 `optional`(可选)                      |
| 重启策略 | `restart_policy`      | `permanent`(永久), `transient`(瞬时)或 `temporary`(临时) |
| 依赖     | `dependencies`        | 依赖的其他子任务名称列表                                 |
| 健康检查 | `health_check`        | 健康检查间隔、超时等配置                                 |
| 就绪检查 | `readiness`           | 显式就绪检查配置                                         |
| 资源限制 | `resource_limits`     | CPU(中央处理器)、内存等资源约束                          |
| 命令权限 | `command_permissions` | 允许此子任务执行的命令                                   |
| 环境变量 | `environment`         | 键值对环境变量列表                                       |
| 密钥引用 | `secrets`             | `${SECRET_NAME}`(密钥占位符)格式的密钥引用               |
| 标签     | `tags`                | 低基数分组标签                                           |
| 任务角色 | `task_role`           | `service`, `worker`, `job`, `sidecar`, `supervisor`      |

完整配置示例见[配置模型](configuration.md#示例配置).

### 配置校验会拒绝哪些情况?

配置加载失败会返回 `SupervisorError::FatalConfig`. 拒绝启动的情况包括:

- 配置文件不是 YAML 格式或无法读取
- 监督策略不是 `OneForOne`, `OneForAll` 或 `RestForOne`
- 数值为零或超出合法范围
- 初始退避大于最大退避
- jitter(抖动)比例不在 0.0 到 1.0 之间
- 重启预算、失败窗口、熔断配置不合法
- 子任务声明中存在依赖循环
- 子任务 ID 或名称不能为空
- Sidecar(辅助进程)任务角色缺少 `sidecar_config`
- Dashboard IPC(进程间通信)路径不是绝对路径

详细错误列表见[配置模型](configuration.md#错误边界).

## 运行时控制

### add_child(追加子任务)的五步事务是什么?

`add_child`(追加子任务)把"解析 -> 校验 -> 注册 -> 拉起 -> 审计持久化"五步连成一串, 当作同一桩事务执行:

1. **解析**: 把 RPC 载荷反序列化为 `ChildDeclaration`(子任务声明)
2. **校验**: 执行 `validate_child_declaration`(校验子任务声明), 检查名称格式、依赖名称存在性、密钥占位符语法等
3. **注册**: 更新拓扑, 把新 child(子任务)插入注册表, 执行环路检测
4. **拉起**: 通过 `TaskFactory`(任务工厂)创建并启动 child future(子任务异步任务)
5. **审计持久化**: 写入审计记录, 包含声明 SHA-256(安全散列算法)哈希

任一步失败时, 整体回退到调用前的拓扑视图, 或者写入 compensating(补偿)段落供善后使用.

### 运行时控制命令有哪些是幂等的?

重复控制命令不会制造不可恢复错误:

- 已暂停的 child(子任务)再次暂停返回当前状态
- 已隔离的 child(子任务)再次隔离返回当前状态
- 已完成 shutdown(关闭)后再次关闭返回已有关闭结果
- `join`(等待结束)会缓存最终 `RuntimeExitReport`(运行时退出报告), 重复调用返回同一结果

### pause(暂停), quarantine(隔离)和 remove(移除)有什么区别?

三个都是停止类控制命令, 但行为不同:

| 命令                           | `operation`(运行状态记录操作) | 运行状态记录保留            | 自动重启           |
| ------------------------------ | ----------------------------- | --------------------------- | ------------------ |
| `pause_child`(暂停子任务)      | `Paused`(已暂停)              | 保留                        | 暂停期间不自动重启 |
| `quarantine_child`(隔离子任务) | `Quarantined`(已隔离)         | 保留                        | 不再自动重启       |
| `remove_child`(移除子任务)     | `Removed`(已移除)             | attempt(尝试)退出后物理删除 | 不适用             |

暂停可以恢复(`resume_child`), 隔离后也可以手动移除. 移除是最终操作, 运行状态记录会被物理删除.

## 策略与失败处理

### RestartPolicy(重启策略)的三个取值分别适用于什么场景?

| 取值              | 行为                   | 适用场景                             |
| ----------------- | ---------------------- | ------------------------------------ |
| `Permanent`(永久) | 始终重启               | 关键服务, 如 API 服务、数据库连接    |
| `Transient`(瞬时) | 只在特定失败类别下重启 | 外部依赖失败时重启, 致命缺陷时不重启 |
| `Temporary`(临时) | 最多重启一次           | 一次性任务(job), 失败后不重试        |

### Meltdown(熔断)的三个层级是如何级联的?

熔断策略(MeltdownPolicy)限制一个窗口内的重启或失败次数, 分为三个层级:

1. **child-level(子任务级)**: 超过 `child_max_restarts` / `child_window_secs` -> 进入 quarantine(隔离)
2. **group-level(分组级)**: 超过 `group_max_failures` / `group_window_secs` -> 升级到 supervisor(监督器)层
3. **supervisor-level(监督器级)**: 超过 `supervisor_max_failures` / `supervisor_window_secs` -> 升级到父级

熔断触发后, 会在 `reset_after_secs`(熔断重置秒数)后自动重置.

## 可观测性

### 如何订阅生命周期事件?

通过 `SupervisorHandle::subscribe_events()`(监督器句柄订阅事件方法)获取 `broadcast::Receiver`(广播接收器). 事件类型为 `SupervisorEvent`(监督器事件), 包含 `When`(何时, 墙钟/单调时间/运行时长/代次/尝试次数)、`Where`(何处, 监督器路径/子任务标识/任务名称)和 `What`(发生内容, 状态迁移/策略决定/健康状态/退出原因/控制命令).

### event journal(事件日志缓冲区)满了怎么办?

event journal(事件日志缓冲区)是固定容量的环形缓冲区. 写满时覆盖最旧条目. 容量通过 `observability.event_journal_capacity`(可观测性事件日志缓冲区容量配置)配置. 但如果使用的是 add_child 事务专用的审计通道, 写满时不会静默覆盖, 而是返回 `Err(AuditStorageFailure)`(审计存储失败错误).

## Dashboard(看板)

### Dashboard(看板)功能需要哪三个仓库配合?

dashboard(看板)功能由三个仓库共同完成:

| 仓库                       | 职责                                                                     |
| -------------------------- | ------------------------------------------------------------------------ |
| `rust-supervisor` (本项目) | target process(目标进程)本机 IPC(进程间通信)和 shared contract(共享契约) |
| `~/rust-supervisor-relay`  | relay(中继)和外部 `wss://` session(会话)                                 |
| `~/rust-supervisor-ui`     | browser dashboard client(浏览器看板客户端)                               |

target process(目标进程)只暴露本机 Unix domain socket(Unix 域套接字), 不能直接把 IPC(进程间通信)暴露到外网.

### IPC(进程间通信)支持哪些方法?

支持的 method(方法)包括: `hello`, `state`, `events.subscribe`, `logs.tail`, `command.restart_child`, `command.pause_child`, `command.resume_child`, `command.quarantine_child`, `command.remove_child`, `command.add_child` 和 `command.shutdown_tree`.

## 项目与构建

### target/debug/rust-tokio-supervisor generate-template 不带参数会做什么?

`generate-template` 不带参数时, **不会输出到 stdout**(终端). 它默认写入 `config/<root-config-name>/<root-config-name>.example.yaml`.

以当前项目为例:

```bash
# 运行后没有终端输出
./target/debug/rust-tokio-supervisor generate-template

# 但实际文件已写入
ls config/supervisor_config/
# supervisor_config.example.yaml
# supervisor_config.schema.json
```

详细参数:

```bash
# 指定输出路径
./target/debug/rust-tokio-supervisor generate-template --output /tmp/my-config.yaml

# 同时生成 JSON Schema
./target/debug/rust-tokio-supervisor generate-template --schema /tmp/schema.json
```

输出格式由文件扩展名推断, 未知或无扩展名时默认使用 YAML(数据序列化格式).

### 为什么 Cargo.toml 里只声明了一个 `[[bin]]`(rust-tokio-supervisor), 但 target/debug/ 下有多个二进制文件?

Cargo 有两种二进制目标声明方式:

1. **显式声明**: 在 `Cargo.toml` 中用 `[[bin]]` 条目声明, 如 `src/main.rs` -> `rust-tokio-supervisor`
2. **自动发现**: `src/bin/` 目录下的每个 `.rs` 文件自动成为一个 binary(二进制)目标, 文件名就是目标名

所以 `Cargo.toml` 中只看到 `[[bin]] name = "rust-tokio-supervisor"`, 但 `src/bin/generate_supervisor.rs` 和 `src/bin/generate_supervisor_config.rs` 被 Cargo 自动发现, 产生了额外的二进制文件.

> 注意: `src/bin/` 目录在功能完成后可能会被清理或移动, 以保持项目结构整洁.

## 常见错误

### `SupervisorError::FatalConfig` 是什么?

`FatalConfig`(致命配置错误)表示配置加载阶段检测到不可恢复的错误. 错误信息中会包含 `field_path`(字段路径, JSON Pointer 格式)和 `hint`(提示段落), 帮助定位具体问题.

### add_child 返回 `Err(SupervisorShuttingDown)`(监督器关闭中错误)怎么办?

这表明 supervisor(监督器)正在执行关停流程, 无法接受新的 add_child 请求. 应该等待 supervisor 完成关闭后重新启动, 再重试添加操作.

### add_child 返回 `Err(ChildLimitExceeded)`(子任务数量超限错误)怎么办?

运行时 child(子任务)数量已经达到上限(当前上限 1000). 需要先移除不必要的子任务(`remove_child`), 或者调整 `dynamic_supervisor.child_limit`(动态监督器子任务上限配置)配置.

### 审计存储失败时会发生什么?

当审计通道(环形缓冲区)写入失败时:

- add_child 会进入 compensating(补偿)流程并返回 `Err(AuditStorageFailure)`(审计存储失败错误)
- 拓扑视图回退到调用前的状态
- 不会留下孤立的半解析状态
