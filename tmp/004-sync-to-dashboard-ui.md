是的, 你的判断是对的. 004 全部实现后, UI(用户界面) 端会有功能性缺口.

关键点是: 004 新增的是 runtime semantics(运行时语义), 但现有 003 dashboard(看板) 契约只覆盖了比较粗的 `runtime_state`, `recent_events`, `recent_logs`, `shutdown_state`, `generation`, `attempt`, `restart_count`. 它还不能完整表达 004 新增的控制面健康, 真实关闭流水线, 子任务槽位, 代际隔离这些事实.

需要和 UI(用户界面) 对齐的观测内容至少有这几组:

1. runtime control plane(运行时控制面)

UI(用户界面) 需要看到目标进程级别的控制面状态, 不是只看 child task(子任务).

需要字段:

- `control_plane.alive`
- `control_plane.state`
- `control_plane.started_at`
- `control_plane.last_observed_at`
- `control_plane.exit_phase`
- `control_plane.exit_reason`
- `control_plane.recoverable`

否则 004-1 实现后, 核心库能知道控制循环死了, 但 dashboard(看板) 仍可能只显示目标进程 registered(已注册) 或 connected(已连接), 操作者会误以为控制面还可用.

2. shutdown pipeline(关闭流水线)

UI(用户界面) 需要展示关闭进度, 不能只有一个 `shutdown_state` 字符串.

需要字段:

- shutdown phase(关闭阶段)
- cancellation delivered count(取消送达数量)
- draining count(正在排空数量)
- aborted count(已强制中止数量)
- failed count(失败数量)
- 每个 child task(子任务) 的 shutdown order(关闭顺序), cancel delivered(取消已送达), exit result(退出结果), timeout(超时), abort result(强制中止结果)

否则 004-2 实现后, 后端已经知道哪个任务卡住, UI(用户界面) 仍只能显示“正在关闭”或“已关闭”, 这就是功能缺口.

3. child slot(子任务槽位)

UI(用户界面) 需要从“声明状态”升级到“真实运行槽位状态”.

需要字段:

- `slot_state`
- `active_attempt`
- `generation`
- `attempt`
- `last_heartbeat_at`
- `readiness`
- `restart_budget_remaining`
- `cancellation_state`
- `task_handle_state`

否则 004-3 实现后, 控制命令真的作用到运行任务, 但 UI(用户界面) 仍看不到暂停, 移除, 隔离到底有没有作用到真实任务.

4. generation fencing(代际隔离)

UI(用户界面) 需要能看出重启冲突和旧代报告被丢弃.

需要字段或事件:

- current generation(当前代数)
- current active attempt(当前活动尝试)
- stale report count(过期报告数量)
- rejected stale report event(拒绝过期报告事件)
- restart conflict result(重启冲突结果)
- old attempt stopped before new attempt started(新尝试启动前旧尝试已停止)

否则 004-4 实现后, 核心库已经避免旧任务覆盖新状态, 但 UI(用户界面) 看不到为什么某个迟到事件没有更新状态.

我的建议是不要只加 Prometheus-style metrics(监控指标). UI(用户界面) 真正需要的是三类契约一起对齐:

- `DashboardState(看板状态)` 增加目标级 `runtime_control_plane` 和更细的 `shutdown_summary`.
- `RuntimeState(运行时状态)` 增加 child slot(子任务槽位) 事实字段.
- `EventRecord(事件记录)` 增加 004 的事件类型, 例如 `runtime_control_loop_failed`, `shutdown_child_cancel_delivered`, `child_slot_state_changed`, `stale_generation_report_dropped`.

结论: 004 核心实现本身可以先落地, 但完成后必须补一个 UI(用户界面) 对齐切片. 最干净的做法是新增一个后续规格, 例如 `004-5-dashboard-runtime-observability`, 专门把 004 的 runtime semantics(运行时语义) 接入 003 dashboard(看板) 的 IPC(进程间通信), relay(中继) 和 UI(用户界面) 状态模型.
