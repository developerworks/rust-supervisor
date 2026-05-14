# 运行时控制

语言: [English](../en/runtime-control.html)

## 控制入口

`SupervisorHandle`(监督器句柄)是运行时控制入口. 它通过命令通道把请求发送给 runtime control loop(运行时控制循环), 并返回 `CommandResult`(命令结果).

## 控制命令

- `add_child`: 当 `DynamicSupervisorPolicy`(动态监督器策略) 允许新增 child(子任务) 时, 接受 dynamic child manifest(动态子任务清单文本).
- `remove_child`: 先关闭目标 child(子任务), 再移除注册表记录.
- `restart_child`: 请求目标 child(子任务)重启.
- `pause_child`: 暂停目标 child(子任务)治理.
- `resume_child`: 恢复目标 child(子任务)治理.
- `quarantine_child`: 把目标 child(子任务)放入隔离状态.
- `shutdown_tree`: 关闭整棵监督树.
- `current_state`: 返回当前 `SupervisorState`(监督器状态).
- `subscribe_events`: 订阅生命周期事件.
- `is_alive`: 快速判断 runtime control loop(运行时控制循环) 是否仍可接收普通控制命令.
- `health`: 返回 `RuntimeHealthReport`(运行时健康报告), 包含控制面状态, 启动时间, 最近观测时间和最终失败原因.
- `join`: 等待 runtime control plane(运行时控制面)进入最终态, 并重复返回同一个 `RuntimeExitReport`(运行时退出报告).
- `shutdown`: 只关闭 runtime control plane(运行时控制面), 不替代 `shutdown_tree`(监督树关闭).

## 幂等语义

重复控制命令不应该制造不可恢复错误. 已暂停的 child(子任务)再次暂停时返回当前状态. 已隔离的 child(子任务)再次隔离时返回当前状态. 已完成 shutdown(关闭)后再次关闭时返回已有关闭结果.

`join`(等待结束) 会缓存控制循环的最终 `RuntimeExitReport`(运行时退出报告). 同一个 handle(句柄) 重复调用 `join`(等待结束) 时, 每次都返回相同结果, 不会再次消费底层 `JoinHandle`(任务句柄).

`shutdown`(关闭) 只请求 runtime control loop(运行时控制循环) 正常退出. 如果控制面已经 completed(已完成) 或 failed(失败), 再次调用 `shutdown`(关闭) 会直接返回已有最终报告. `shutdown_tree`(监督树关闭) 仍然负责 child task(子任务)和整棵监督树的关闭语义.

## 运行时健康

`is_alive`(是否存活) 是低成本状态判断. 当控制面处于 alive(存活) 时, 它返回 `true`. 当控制面处于 starting(启动中), shutting_down(正在关闭), completed(已完成) 或 failed(失败) 时, 它返回 `false`.

`health`(健康报告) 返回结构化状态. 控制面异常退出后, `health`(健康报告) 仍然可以读取 failed(失败)状态, failure phase(失败阶段), reason(原因), panic(恐慌)标记和 recoverable(可恢复)标记. 普通控制命令在控制面结束后会返回包含同一退出原因的 `SupervisorError`(监督器错误).

## 动态添加

运行时会在接受 manifest(清单文本) 前执行 dynamic addition(动态添加) 治理. 当 dynamic supervision(动态监督) 被禁用, 或 declared child count(声明子任务数量) 加 dynamic child count(动态子任务数量) 已经达到配置上限时, `add_child`(添加子任务) 会被拒绝. `current_state`(当前状态) 的 `child_count`(子任务数量) 包含已经接受的 dynamic manifest(动态清单文本).

## 审计数据

每个控制命令都带有 `requested_by`(请求者), `reason`(原因), `target_path`(目标路径), `accepted_at`(接受时间)和 `command_id`(命令标识). 这些字段用于 audit event(审计事件)和问题追踪.

`requested_by`(请求者) 和 `reason`(原因) 必须提供非空文本. `SupervisorHandle`(监督器句柄) 会在命令进入 channel(通道) 前拒绝空值, runtime control loop(运行时控制循环) 也会在执行命令前再次校验. 这样做可以保证人工操作, dashboard IPC(看板进程间通信) 转发和内部控制调用都留下可追踪的审计来源.
