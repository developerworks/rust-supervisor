# Spec Sync Drift Report(规格同步偏差报告)

Generated(生成时间): 2026-05-15T01:14:53+08:00
Project(项目): rust-supervisor

## Summary(摘要)

| Category(类别) | Count(数量) |
|---|---:|
| Specs Analyzed(已分析规格) | 7 |
| Requirements Checked(已检查需求) | 213 |
| Aligned(已对齐) | 185 |
| Drifted(存在偏差) | 16 |
| Not Implemented(尚未实现) | 12 |
| Unspecced Product Code(无规格产品代码) | 0 |

## Scope(范围)

本次分析读取当前仓库的 `specs/*/spec.md`, `src/`, `tests/`, `manual/`, 以及 `.specify/sync` 中已有同步产物. 当前工作区已经包含 `004-1-runtime-lifecycle-guard`, `004-2-real-shutdown-pipeline`, `004-3-child-slot-control` 和 `004-4-generation-fencing` 的规格与运行时代码改动, 所以本报告按当前工作区状态判断.

## Detailed Findings(详细发现)

### 001-create-supervisor-core - 创建监督器核心

#### Drifted(存在偏差)

- `FR-004`: `TaskContext` 当前携带 `child_id`, `path`, `generation`, `attempt`, `cancellation_token`, `ready_signal` 和 `heartbeat_sender`, 但是没有事件发布 `sink(接收端)`. 位置: `src/task/context.rs:14`.
- `FR-005`: 树构建当前以扁平 `children` 为主, `nested_supervisors()` 只做筛选, 没有把多层监督树作为统一启动树递归展开. 位置: `src/tree/builder.rs:1`.
- `FR-010`: `TaskExit` 当前没有独立 `Unhealthy(不健康)` 终态. 位置: `src/child_runner/attempt.rs:12`.
- `FR-011`: `TaskFailureKind` 当前没有把 `Recoverable(可恢复)`, `FatalBug(致命缺陷)`, `ExternalDependency(外部依赖)` 建模为稳定分类. 位置: `src/error/types.rs:56`.
- `FR-044`: `TaskKind::BlockingWorker` 已存在, 但是 `child runner(子任务运行器)` 没有独立阻塞任务执行和关闭语义. 位置: `src/spec/child.rs:16`, `src/child_runner/runner.rs:80`.
- `FR-045`: 关闭报告可以标记 `journal(事件日志)` 和 `metrics(指标)` 状态, 但是运行时关闭路径没有直接写入真实 `journal sink(事件日志接收端)` 和 `metrics sink(指标接收端)`. 位置: `src/shutdown/report.rs:163`, `src/runtime/control_loop.rs:230`.
- `FR-049`: `observability pipeline(观测流水线)` 已有 `typed event(类型化事件)` 映射, 但是 `runtime watchdog(运行时看门狗)` 仍通过字符串 `broadcast(广播)` 发布控制循环事件. 位置: `src/runtime/watchdog.rs:49`, `src/observe/pipeline.rs:292`.
- `SC-015`: 阻塞任务关闭边界尚未通过独立执行和关闭路径体现. 位置: `src/child_runner/runner.rs:80`.

### 002-config-schema-support - 配置结构体模式支持

#### Aligned(已对齐)

- `FR-001` 到 `FR-017`, `SC-001` 到 `SC-007` 当前和配置结构体, `JSON Schema(数据结构模式)`, `YAML(数据序列化格式)` 模板, 语义校验, 启动拒绝和文档同步实现保持对齐.

### 003-supervisor-dashboard - 监督任务可视化界面

#### Drifted(存在偏差)

- `SC-012`: 目标侧和前端侧 `React(前端框架)` 排除验收仍有未完成任务. 位置: `specs/003-supervisor-dashboard/tasks.md:143`.

#### Not Implemented(尚未实现)

- `SC-003`: 固定数据集和 20 轮刷新性能验收任务仍未闭合. 位置: `specs/003-supervisor-dashboard/tasks.md:143`.

### 004-1-runtime-lifecycle-guard - 运行时生命周期守卫

#### Drifted(存在偏差)

- `FR-002`: 控制循环异常退出可以通过 `health(健康状态)` 和字符串 `broadcast(广播)` 被看到, 但是 `RuntimeControlLoopFailed(运行时控制循环失败)` 还没有由 `watchdog(看门狗)` 作为 `typed event(类型化事件)` 写入 `observability pipeline(观测流水线)`. 位置: `src/runtime/watchdog.rs:49`, `src/event/payload.rs:344`, `src/observe/metrics.rs:242`, `src/observe/pipeline.rs:292`.
- `SC-002`: 控制循环崩溃还没有稳定进入 `typed event(类型化事件)` 观测链路. 位置: `src/runtime/watchdog.rs:49`.

### 004-2-real-shutdown-pipeline - 真实关闭流水线

#### Drifted(存在偏差)

- `FR-003`: 真实关闭流水线已经生成 `reconcile report(对账报告)`, 但是 `core_runtime_completed()` 对 `journal(事件日志)` 和 `metrics(指标)` 使用报告默认值, 没有证明真实 `sink(接收端)` 已经持久化. 位置: `src/shutdown/report.rs:163`, `src/runtime/control_loop.rs:230`.

### 004-3-child-slot-control - 子任务槽位控制

#### Drifted(存在偏差)

- `FR-001`: `ActiveChildAttempt(活动子任务尝试)` 已经保存 `child_id`, `path`, `generation`, `attempt`, `cancellation_token`, `abort_handle` 和完成接收端, 但是它只服务关闭流水线, 没有形成包含 `heartbeat(心跳)`, `ready state(就绪状态)`, `restart budget(重启预算)` 和最终状态的通用 `child slot(子任务槽位)`. 位置: `src/runtime/shutdown_pipeline.rs:17`.

#### Not Implemented(尚未实现)

- `FR-002`: `pause_child`, `remove_child` 和 `quarantine_child` 当前只写 `ManagedChildState(受管子任务状态)`, 没有取消或停止真实运行中的 `child attempt(子任务尝试)`. 位置: `src/runtime/control_loop.rs:134`.
- `FR-003`: `current_state(当前状态)` 当前只返回 `child_count` 和 `shutdown_completed`, 没有暴露 `child slot(子任务槽位)` 的 `attempt(尝试)`, `generation(代际)`, `readiness(就绪状态)`, `heartbeat(心跳)` 和停止结果. 位置: `src/control/command.rs:223`, `src/runtime/control_loop.rs:156`.
- `SC-001`: 暂停命令不会等待目标子任务停止, 也不会返回可观察停止结果. 位置: `src/runtime/control_loop.rs:134`.
- `SC-002`: 恢复命令会再次启动子任务, 但是外部状态没有暴露新的 `attempt(尝试)` 和 `generation(代际)`. 位置: `src/runtime/control_loop.rs:151`, `src/control/command.rs:223`.
- `SC-003`: 隔离命令只写状态, 没有强制停止当前运行 `attempt(尝试)`, 也没有阻止后续自动重启的完整槽位规则. 位置: `src/runtime/control_loop.rs:145`.
- `SC-004`: 移除命令只写状态, 没有停止真实任务, 也没有把子任务从控制面状态中移除. 位置: `src/runtime/control_loop.rs:142`.

### 004-4-generation-fencing - 代际隔离重启

#### Drifted(存在偏差)

- `FR-001`: `restart_child(重启子任务)` 当前会 `abort(中止)` 现有 `active attempt(活动尝试)`, 然后立即启动新的 `attempt(尝试)`, 没有先 `cancel(取消)` 并等待旧 `attempt(尝试)` 完成. 位置: `src/runtime/control_loop.rs:137`, `src/runtime/control_loop.rs:813`.
- `FR-002`: 控制循环 `map(映射表)` 中只保留一个 `ActiveChildAttempt(活动子任务尝试)`, 但是旧 `future(异步任务)` 被 `abort(中止)` 后到 `join(汇合)` 完成前仍可能存活. 位置: `src/runtime/control_loop.rs:813`, `src/runtime/shutdown_pipeline.rs:95`.
- `FR-003`: `record_child_exit` 没有校验 `generation(代际)`, 旧 `attempt(尝试)` 的迟到结果仍可能更新 `registry(注册表)`. 位置: `src/runtime/control_loop.rs:628`.

#### Not Implemented(尚未实现)

- `SC-001`: 连续两次 `restart(重启)` 没有验证只有最后一代可以进入 `running(运行中)`. 位置: `src/runtime/control_loop.rs:846`.
- `SC-002`: 旧 `generation(代际)` 的迟到退出没有被标记为 `stale report(过期报告)` 并忽略. 位置: `src/runtime/control_loop.rs:182`.
- `SC-003`: 重启期间还没有 `cancel-wait-start(取消, 等待, 启动)` 顺序, 因此不能保证没有重叠活跃任务. 位置: `src/runtime/control_loop.rs:813`.
- `SC-004`: `current_state(当前状态)` 没有暴露当前 `generation(代际)`, `UI(用户界面)` 和测试无法确认代际隔离结果. 位置: `src/control/command.rs:223`.

## Unspecced Code(无规格代码)

未发现需要创建新产品规格的无规格产品代码. 当前 `.specify/extensions/sync` 属于 `Spec Kit(规格工具包)` 同步工具链, 本次不把它计入产品功能偏差.

## Recommendations(建议)

1. 先处置 `004-3-child-slot-control`, 因为真实 `child slot(子任务槽位)` 是停止类控制命令和代际隔离的共同基础.
2. 再处置 `004-4-generation-fencing`, 因为 `restart(重启)` 必须依赖真实槽位和当前 `generation(代际)`.
3. 随后补齐 `004-1-runtime-lifecycle-guard` 和 `004-2-real-shutdown-pipeline` 的 `typed event(类型化事件)`, `journal(事件日志)` 和 `metrics(指标)` 落点.
4. 最后闭合 `003-supervisor-dashboard` 的 T066 性能基线和 `React(前端框架)` 排除验收.
