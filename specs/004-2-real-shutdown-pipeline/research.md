# Research(研究结论): 真实关闭流水线

## 决策一: 关闭取消继续使用 `CancellationToken(取消令牌)`

**Decision(决定)**: 本功能继续使用 `tokio_util::sync::CancellationToken(取消令牌)`, 并且让 runtime(运行时) 在每个 active attempt(活动尝试) 上保存一个 clone(克隆) 后的 token(令牌).

**Rationale(理由)**: `TaskContext(任务上下文)` 已经把 token(令牌) 交给 child task(子任务), 任务可以通过 `is_cancelled` 或 `cancellation_token` 主动观察关闭请求. 运行时保存同一 token(令牌) 的 clone(克隆) 后, `ShutdownTree(关闭监督树)` 就能向真实运行中的任务发送取消请求.

**Alternatives considered(备选方案)**: 自定义 watch channel(观察通道) 会重复现有能力, 并且会让任务上下文出现两个取消来源. 只调用 `JoinHandle::abort(强制中止)` 会跳过优雅释放阶段, 不符合功能规格.

## 决策二: 运行时必须保存实际 child future(子任务 future) 的 `AbortHandle(强制中止句柄)`

**Decision(决定)**: `ChildRunner(子任务运行器)` 必须暴露能够中止真实 child future(子任务 future) 的句柄, 或者让 `RuntimeControlState(运行时控制状态)` 直接保存该句柄. 句柄必须指向执行 `TaskFactory::build(任务工厂构建)` 结果的任务, 而不是只指向外层上报任务.

**Rationale(理由)**: 当前 `ChildRunner::run_once` 内部会再创建一个 `tokio::spawn(异步任务)`. 如果控制循环只中止外层上报任务, 内层 child future(子任务 future) 仍可能继续运行. 真实关闭流水线必须能在超时后中止滞留任务, 所以句柄边界必须收敛到真实任务.

**Alternatives considered(备选方案)**: 保持当前双层 spawn(派生任务) 结构并中止外层任务无法保证没有 orphaned task(孤儿任务). 把 child future(子任务 future) 放到同步阻塞线程不符合当前 Tokio runtime(Tokio 运行时) 结构.

## 决策三: `ShutdownCoordinator(关闭协调器)` 只保留阶段状态

**Decision(决定)**: `ShutdownCoordinator(关闭协调器)` 继续拥有 `ShutdownPhase(关闭阶段)`, `ShutdownCause(关闭原因)` 和 `ShutdownPolicy(关闭策略)`. `ShutdownPipelineReport(关闭流水线报告)` 和相关公开报告类型放入 `src/shutdown/report.rs`. 真实取消, 等待, 强制中止和对账逻辑放入 `src/runtime/shutdown_pipeline.rs`.

**Rationale(理由)**: `ShutdownResult(关闭结果)` 属于 shutdown(关闭) 模块, 所以它引用的公开报告类型也必须属于 shutdown(关闭) 模块. 任务句柄, registry(注册表), event(事件), metrics(指标) 和 control loop mailbox(控制循环邮箱) 都属于 runtime(运行时) 边界. 如果把句柄放入 shutdown(关闭) 模块, shutdown(关闭) 模块会越过原有职责. 如果把报告类型放入 runtime(运行时) 模块, shutdown(关闭) 模块会反向依赖 runtime(运行时) 模块.

**Alternatives considered(备选方案)**: 把所有逻辑塞入 `src/runtime/control_loop.rs` 会让控制循环同时承担消息路由和关闭执行, 不利于并行开发和测试. 把 `ShutdownCoordinator(关闭协调器)` 改成拥有句柄会破坏现有阶段状态机的清晰职责.

## 决策四: `ShutdownTree(关闭监督树)` 返回完整 `ShutdownResult(关闭结果)`

**Decision(决定)**: `CommandResult::Shutdown(关闭命令结果)` 继续作为调用者入口. `ShutdownResult(关闭结果)` 需要增加可序列化的 pipeline report(流水线报告), 用来返回每个 child(子任务) 的取消送达, 等待结果, 强制中止结果和最终对账状态. 该报告类型必须来自 `src/shutdown/report.rs`.

**Rationale(理由)**: 当前公开命令语义不能改变, 但是操作者需要看到每个 child(子任务) 的最终结果. 在 `ShutdownResult(关闭结果)` 中加入报告可以让 dashboard(仪表盘), 测试和库调用者读取同一个结构化事实.

**Alternatives considered(备选方案)**: 只通过日志暴露结果无法满足调用者可见结果要求. 新增一个单独命令会让 `ShutdownTree(关闭监督树)` 仍然不完整.

## 决策五: 重复关闭请求必须复用同一个关闭过程

**Decision(决定)**: 第一次 `ShutdownTree(关闭监督树)` 创建并执行关闭流水线. 关闭进行中或完成后再次请求时, 控制循环返回当前进度或已缓存的最终报告, 并把 `idempotent(幂等)` 标记设为 `true`.

**Rationale(理由)**: 重复关闭在控制面很常见. 如果每次请求都重新发送取消和重启阶段推进, 结果会出现重复事件和不稳定摘要. 缓存报告可以保证调用者看到同一关闭事实.

**Alternatives considered(备选方案)**: 对重复请求返回错误会让控制面难以安全重试. 静默忽略重复请求会让调用者失去可观察结果.

## 决策六: 对账要区分 runtime-owned(运行时拥有) 和 not-owned(非运行时拥有) 资源

**Decision(决定)**: 关闭对账报告必须分别说明 registry(注册表), runtime handles(运行时句柄), journal(日志), metrics(指标) 和 socket(套接字) 的状态. 核心 runtime(运行时) 当前不直接拥有 dashboard IPC socket(仪表盘进程间通信套接字), 所以 socket(套接字) 对账状态必须记录为 `NotOwned(非运行时拥有)`.

**Rationale(理由)**: 功能规格要求处理 socket(套接字), journal(日志) 和 metrics(指标) 的最终状态. 当前仓库中核心 supervisor runtime(监督器运行时) 只拥有运行时句柄和注册表, 观测管线拥有 event(事件), audit(审计) 和 metrics(指标) 输出. 对账报告必须反映真实所有权, 不能伪造资源清理.

**Alternatives considered(备选方案)**: 在运行时内新增 socket(套接字) 管理会扩大本功能范围. 省略 socket(套接字) 字段会让规格和实现之间继续存在观测缺口.

## 决策七: 测试必须覆盖合作关闭和非合作关闭

**Decision(决定)**: 新增 `supervisor_real_shutdown_pipeline_test` 覆盖所有任务收到取消, 已退出任务不重复取消, 没有运行中任务时输出 `AlreadyExited(已经退出)`, `shutdown_order(关闭顺序)` 等待, 忽略取消的任务被强制中止, 重复关闭返回缓存结果和迟到报告归并.

**Rationale(理由)**: 真实关闭流水线的风险集中在异步边界. 只测试阶段枚举无法证明任务真的停止. 测试必须通过真实 task factory(任务工厂) 观察 token(令牌), completion(完成), abort(强制中止) 和最终摘要.

**Alternatives considered(备选方案)**: 只写 coordinator(协调器) 单元测试不能覆盖运行时句柄. 只依赖 dashboard protocol(仪表盘协议) 形状测试不能证明关闭行为.
