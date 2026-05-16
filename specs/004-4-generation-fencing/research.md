# Research(研究结论): 代次隔离重启

## 决策一: `RestartChild(重启子任务)` 使用 pending restart(待重启) 状态机

**Decision(决定)**: `RestartChild(重启子任务)` 不在 control loop(控制循环) 中同步等待旧 attempt(尝试) 完成. 命令到达时, runtime(运行时) 在 `ChildRuntimeState(子任务运行状态记录)` 上写入 `PendingRestart(待重启请求)`, 向当前 active attempt(活动尝试) 发送取消, 记录停止截止时间, 并返回结构化 `GenerationFenceOutcome(代次隔离结果)`. 旧 attempt(尝试) 的退出仍通过现有 completion observer(完成观察任务) 进入 control loop(控制循环). exit handler(退出处理) 确认旧 `(generation, attempt)(代次和尝试)` 已停止后, 才允许启动新的 generation(代次).

**Rationale(理由)**: control loop(控制循环) 是单线程消息循环. 如果 `RestartChild(重启子任务)` 在命令处理中直接等待旧任务退出, 其他控制命令和 `CurrentState(当前状态)` 会被阻塞. pending restart(待重启) 能保持命令响应可观察, 同时用现有退出报告路径完成等待.

**Alternatives considered(备选方案)**:

- 命令处理内直接 `await` 旧任务完成. 该方案会让不响应取消的任务阻塞控制面.
- 直接中止旧任务并立即启动新任务. 该方案满足不了 graceful shutdown(优雅关闭) 语义, 也会隐藏任务能否响应取消的事实.

## 决策二: fence identity(隔离身份) 使用 `(child_id, generation, attempt)` 三元组

**Decision(决定)**: 所有启动, 停止, 退出报告和过期报告判定都使用 `(child_id, generation, attempt)` 作为唯一身份. `child_id(子任务标识)` 只表达声明身份. `generation(代次)` 表达一次重启边界. `attempt(尝试)` 表达同一 generation(代次) 内的实际启动编号.

**Rationale(理由)**: 单独使用 `child_id(子任务标识)` 无法区分新旧运行实例. 单独使用 `generation(代次)` 无法定位同一 child(子任务). 三元组可以让 exit handler(退出处理) 判定报告是否属于当前活动尝试, 待重启旧尝试, 或者已经过期的旧报告.

**Alternatives considered(备选方案)**:

- 只比较 `generation(代次)`. 该方案在多个 child(子任务) 同时运行时不够明确.
- 用时间戳作为隔离身份. 该方案会混淆 `Generation(代次)` 和 `UNIX_EPOCH(Unix 纪元常量)`, 与命名契约冲突.

## 决策三: 自动重启和手动重启共用同一启动门禁

**Decision(决定)**: `spawn_child_start(派生子任务启动)` 或其等价入口必须先检查 `ChildRuntimeState(子任务运行状态记录)` 是否已有 active attempt(活动尝试) 或 pending restart(待重启请求). 如果已有活动尝试, 启动入口不得调用 `abort(强制中止)` 后直接覆盖该尝试. 自动重启和手动重启必须先取得 generation fence(代次隔离) 的启动许可.

**Rationale(理由)**: 当前代码路径中 `spawn_child_start(派生子任务启动)` 在发现已有运行状态记录时直接 `abort(强制中止)` 旧句柄, 然后激活新实例. 这会制造同一 child id(子任务标识) 的多个运行实例窗口. 把启动门禁放在公共入口可以同时保护手动重启和自动重启.

**Alternatives considered(备选方案)**:

- 只修复 `RestartChild(重启子任务)` 分支. 该方案无法保护自动重启和手动重启同时发生的竞态.
- 在 `ChildRunner(子任务运行器)` 内部拒绝重复启动. 该方案让运行器承担控制面状态判断, 模块边界不清晰.

## 决策四: 重复 `RestartChild(重启子任务)` 默认合并到已有待重启请求

**Decision(决定)**: 当同一 `ChildRuntimeState(子任务运行状态记录)` 已有 pending restart(待重启请求) 时, 后续 `RestartChild(重启子任务)` 不启动新 attempt(尝试), 不重复取消, 不覆盖原始 command id(命令标识). 它返回 `GenerationFenceOutcome.decision = AlreadyPending(已存在待重启)` 的结构化结果, 并记录新的审计事实.

**Rationale(理由)**: 规格允许拒绝, 合并或排队第二个重启请求, 但要求不得启动第二个活动尝试. 合并是最小行为, 不需要额外队列, 也不会让同一个 child(子任务) 在旧尝试仍未退出时积压多个重启意图.

**Alternatives considered(备选方案)**:

- 返回错误并拒绝重复命令. 该方案可行, 但会让操作者在网络重试场景中更容易看到失败.
- 为每个重复请求排队. 该方案会扩大范围, 因为本功能不需要连续多次重启语义.

## 决策五: restart fence(重启隔离) 可以在截止时间后请求强制中止

**Decision(决定)**: `RestartChild(重启子任务)` 与 `PauseChild(暂停子任务)`, `RemoveChild(移除子任务)`, `QuarantineChild(隔离子任务)` 的停止语义不同. 为了保证重启最终不会启动第二个活动尝试, pending restart(待重启请求) 在 `ShutdownPolicy.graceful_timeout(关闭策略优雅等待时间)` 经过后可以请求旧 attempt(尝试) 的 `abort(强制中止)`. 新 generation(代次) 仍必须等旧 attempt(尝试) 的完成报告到达后才启动.

**Rationale(理由)**: 停止类命令在 `004-3-child-runtime-state-control` 中只标记停止失败, 不强制中止. 重启命令的业务目标不同, 它必须让新 generation(代次) 最终有机会启动, 同时仍然保持单活动尝试. 因此重启路径可以复用 `abort_handle(强制中止句柄)`, 但不能绕过完成报告直接激活新尝试.

**Alternatives considered(备选方案)**:

- 永远只取消, 不中止. 该方案会让不响应取消的任务永久阻塞手动重启.
- 中止后立即启动新尝试. 该方案仍可能在旧 future(异步任务) 完成前制造重叠窗口.

## 决策六: stale report(过期报告) 必须成为可观察事实

**Decision(决定)**: 当 exit handler(退出处理) 收到的 `(generation, attempt)(代次和尝试)` 既不是当前 active attempt(活动尝试), 也不是 pending restart(待重启请求) 中的 old attempt(旧尝试) 时, runtime(运行时) 必须把它记录为 `StaleAttemptReport(过期尝试报告)`. 该报告不得覆盖当前 `ChildRuntimeState(子任务运行状态记录)`, 但必须发布 `ChildAttemptStaleReport(子任务过期报告)` 事件, 写入 audit(审计), 并增加 metrics(指标).

**Rationale(理由)**: 迟到报告如果被静默丢弃, 操作者无法解释日志中旧任务的退出时间. 如果迟到报告覆盖当前状态, 新 generation(代次) 的事实会被污染. 把它作为可观察事实记录可以同时满足安全性和可诊断性.

**Alternatives considered(备选方案)**:

- 完全忽略迟到报告. 该方案无法审计旧任务行为.
- 让迟到报告覆盖当前状态并增加说明字段. 该方案破坏当前 generation(代次) 的唯一事实来源.
