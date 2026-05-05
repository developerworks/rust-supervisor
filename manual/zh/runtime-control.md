# 运行时控制

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

## 幂等语义

重复控制命令不应该制造不可恢复错误. 已暂停的 child(子任务)再次暂停时返回当前状态. 已隔离的 child(子任务)再次隔离时返回当前状态. 已完成 shutdown(关闭)后再次关闭时返回已有关闭结果.

## 动态添加

运行时会在接受 manifest(清单文本) 前执行 dynamic addition(动态添加) 治理. 当 dynamic supervision(动态监督) 被禁用, 或 declared child count(声明子任务数量) 加 dynamic child count(动态子任务数量) 已经达到配置上限时, `add_child`(添加子任务) 会被拒绝. `current_state`(当前状态) 的 `child_count`(子任务数量) 包含已经接受的 dynamic manifest(动态清单文本).

## 审计数据

每个控制命令都带有 `requested_by`(请求者), `reason`(原因), `target_path`(目标路径), `accepted_at`(接受时间)和 `command_id`(命令标识). 这些字段用于 audit event(审计事件)和问题追踪.
