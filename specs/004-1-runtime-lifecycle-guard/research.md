# Research(研究结论): 运行时生命周期守卫

## Decision(决策): 用 `RuntimeControlPlane(运行时控制面)` 保存可重复读取的生命周期状态

**Rationale(理由)**: `tokio::task::JoinHandle`(任务句柄) 只能被等待一次. 规格要求 `SupervisorHandle`(监督器控制句柄) 重复调用 `join` 10 次都返回同一个最终结果, 因此公共句柄不能把原始 `JoinHandle(任务句柄)` 直接暴露给调用者. 运行时需要一个共享的 `RuntimeControlPlane(运行时控制面)` 存储 started_at(启动时间), last_observed_at(最近观测时间), state(状态), exit_report(退出报告) 和失败原因. `join` 方法等待这个共享状态进入最终态, 然后返回缓存的最终结果.

**Alternatives considered(备选方案)**:

- 直接在 `SupervisorHandle`(监督器控制句柄) 中保存 `JoinHandle(任务句柄)`: 被拒绝, 因为它不能支持重复 `join` 幂等.
- 只依赖 `mpsc::Sender::is_closed` 判断 alive(存活): 被拒绝, 因为它不能说明异常退出阶段和原因.
- 在每个控制命令失败时临时构造健康状态: 被拒绝, 因为规格要求在下一次控制命令发送前主动可见.

## Decision(决策): 用 `RuntimeWatchdog(运行时看门狗)` 消费控制循环 `JoinHandle(任务句柄)`

**Rationale(理由)**: 运行时控制循环可能正常返回, 也可能 panic(恐慌). watchdog(看门狗) 持有控制循环的 `JoinHandle(任务句柄)`, 等待它完成, 再把 `JoinError(任务等待错误)` 或正常退出结果转换成 `RuntimeExitReport(运行时退出报告)`. 这样控制循环的退出不会被丢弃, 操作者也不需要等到下一次命令失败才知道控制面已经结束.

**Alternatives considered(备选方案)**:

- 在 `run_control_loop` 内部自行更新最终状态: 被拒绝, 因为 panic(恐慌) 路径不会执行内部收尾代码.
- 让调用者显式等待 `JoinHandle(任务句柄)`: 被拒绝, 因为这会把内部任务所有权泄漏给公共 API(应用程序编程接口).
- 自动重启控制循环: 被拒绝, 因为本规格明确不默认自动重启控制循环, 后续可以单独规格化恢复策略.

## Decision(决策): 新增显式 control-plane shutdown(控制面关闭) 命令

**Rationale(理由)**: 当前 `shutdown_tree` 是监督树业务命令, 它不等同于关闭 runtime control loop(运行时控制循环). 本规格需要 `SupervisorHandle::shutdown` 主动结束控制面并可等待最终结果. 因此控制循环需要接收一个内部 `RuntimeCommand::ShutdownControlPlane` 或等价消息, 该消息请求控制循环返回 completed(已完成) 状态. 真实 child task(子任务) 关闭由后续规格处理, 本阶段只保证控制面最终态可见.

**Alternatives considered(备选方案)**:

- 通过丢弃所有 `command_sender` 关闭控制循环: 被拒绝, 因为克隆句柄存在时不可控, 并且调用者无法得到结构化关闭结果.
- 复用 `shutdown_tree` 关闭控制面: 被拒绝, 因为它会混淆监督树生命周期和控制面任务生命周期.
- 给 `current_state` 加特殊参数: 被拒绝, 因为查询命令不应该承担关闭副作用.

## Decision(决策): 扩展现有 typed event(类型化事件), metrics(指标) 和 audit log(审计日志) 管线

**Rationale(理由)**: 仓库已经有 `event::payload::SupervisorEvent`, `observe::metrics::MetricsFacade` 和 `observe::pipeline::ObservabilityPipeline`. 本功能应在这些已有边界内新增控制面生命周期事件和指标, 而不是新建并行诊断格式. 事件必须覆盖 started(已启动), shutdown requested(已请求关闭), completed(已完成), failed(失败) 和 join completed(等待结束已完成). metrics(指标) 使用低基数标签, 至少记录控制循环退出总数和 alive(存活) 状态. audit log(审计日志) 复用 `CommandAudit(命令审计)` 的请求者, 原因, command_id(命令标识) 和结果字段.

**Alternatives considered(备选方案)**:

- 继续只发送 `broadcast::Sender<String>` 文本事件: 被拒绝, 因为字符串事件不能满足 typed event(类型化事件) 和 metrics(指标) 映射要求.
- 为运行时控制面新增独立日志结构: 被拒绝, 因为它会绕开已有 observability(可观察性) 管线.
- 把 metrics(指标) 只放在测试断言中: 被拒绝, 因为指标名称和标签必须成为稳定契约.

## Decision(决策): 本阶段只改核心库, 不扩展 relay(中继) 和 dashboard client(看板客户端)

**Rationale(理由)**: 规格假设本阶段只覆盖当前核心库中的运行时控制面. relay(中继) 和 dashboard client(看板客户端) 可以在后续规格读取新的健康状态, 但是本规格不修改跨仓库通信.

**Alternatives considered(备选方案)**:

- 同时更新 relay(中继) 和 dashboard client(看板客户端): 被拒绝, 因为这会扩大当前第一阶段运行时语义修正的边界.
- 在手册中承诺 dashboard(看板) 已展示新健康状态: 被拒绝, 因为当前规格没有交付该 UI(用户界面) 能力.
