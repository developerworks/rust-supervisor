# Implementation Plan(实现计划): 代次隔离重启

**Branch(分支)**: `004-runtime-semantics` | **Date(日期)**: 2026-05-15 | **Spec(规格)**: `/specs/004-4-generation-fencing/spec.md`

**Input(输入)**: 功能规格来自 `/specs/004-4-generation-fencing/spec.md`

**Note(说明)**: 本文件由 `/speckit-plan` 写入。设计与契约见同目录下的 `research.md`, `data-model.md`, `contracts/`, `quickstart.md`。任务拆分由 Spec Kit(规格工具套件) 中的 `/speckit-tasks` 命令产出 `tasks.md`; 此处「任务列表产出步骤」勿与同目录 `tasks.md` 正文内的 Phase 2(阶段二) Foundational(阻塞前置基础) 编号混淆。

## Summary(摘要)

本功能解决 `RestartChild(重启子任务)` 在未先收敛旧尝试的情况下直接再起新实例的问题。规格要求同一时间每个 `ChildId(子任务标识)` 至多一个 `ActiveAttempt(活动尝试)`, 重启前先发取消并经 `generation fencing(代次隔离)` 等待或升级中止旧尝试, `late report(迟到上报)` 必须成为 `StaleReport(过期报告)` 或可审计事实且不覆盖当前代次事实。

Phase 0(研究阶段) 已确定技术路线: `RestartChild(重启子任务)` 采用异步 `pending restart(待重启)` 状态机, 不向 `control loop(控制循环)` 同步阻塞等待退出。隔离身份使用 `(child_id, generation, attempt)(子任务标识,代次与尝试)` 三元组。手动与自动重启共用同一 `spawn`(派生启动)门禁。重复重启请求默认合并并返回 `AlreadyPending(已存在待重启)`. 超时后可请求 `abort(强制中止)`, 但仍须等退出报告到达后才能启动目标代次新尝试。

## Technical Context(技术背景)

**Language/Version(语言和版本)**: Rust 2024(编程语言版本), `rust-version` 以根目录 `Cargo.toml` 为准, 当前为 `1.88`.

**Primary Dependencies(主要依赖)**: `tokio` 异步运行时, `tracing`(结构化追踪), `serde`/`serde_json`/`serde_yaml` 序列化, `metrics` 指标, `thiserror` 错误建模, `confique`, `uuid` 等仓库既有依赖体。功能规格不写死新增 `crate`(库)。若实现阶段确需引入新依赖, 必须在本小节补写理由并把条目落入宪章 Complexity Tracking(复杂度跟踪).

**Storage(存储)**: `N/A(不适用)`。运行时状态常驻内存并由现有监督状态结构持有。

**Testing(测试)**: 矩阵以 `quickstart.md` 为准. 核心门禁包含新增 `cargo test --test supervisor_generation_fencing_test`, 以及与 `004-3-child-runtime-state-control(子任务运行状态控制)` 对齐的受影响集成测试目标和 `cargo test`. 根目录 `tests/dashboard_protocol_shape_test.rs` 为 Cargo(构建工具) 自动发现的集成测试目标, 本功能只允许原地扩展返回结果断言. 新增顶层集成测试文件名若注册为 `--test`, 必须与 `Cargo.toml` 的 `[[test]]` 或自动发现路径一致.

**Stale report test replay(过期报告测试回放)**: 为实现 **`spec.md`** **`FR-003`** 与 **`SC-003`** 在集成测试中的可注入路径, **`SupervisorHandle`** (`src/control/handle.rs`) 提供 **`#[doc(hidden)]`** 的 **`async fn generation_fencing_replay_child_exit_for_test`**, 返回 **`Result<(), SupervisorError>`**. 该方法在校验 **`control plane`** 存活后, 将携带合成 **`ChildRunReport`** 的 **`ControlPlaneMessage::ReplayChildExitForTest`** 封装进 **`RuntimeLoopMessage::ControlPlane`**, **`mailbox`** 投递并由 **`runtime control loop`** 按与真实退出一致的 **`stale report`** 语义处理. **`supervisor_generation_fencing_test`** 用它断言 **`ChildAttemptStaleReport`** 等载荷. **生产二进制**, **`Rustdoc`** **公开示例**, **`dashboard`** **二进制不得调用**, **也不得把该方法当作稳定 `API`(应用程序接口)**. **读者还以 `contracts/generation-fencing.md` `Stale Report` 专节为准**.

**Target Platform(目标平台)**: 异步 Tokio(异步运行时) 监督器运行时, 开发与 CI(持续集成) 以 POSIX(可移植操作系统接口族) 类环境为主。

**Project Type(项目类型)**: 单 workspace crate `rust-tokio-supervisor`(包名), 提供 `rust_supervisor` 库并可作为示例或 CLI 使用。

**Performance Goals(性能目标)**: 本功能不改变监督吞吐量的明确数值 SLA(服务级别目标)。关注点为控制面不会因同步等待单次重启而饿死其他控制命令, 以及指标与事件的基数约束见 `contracts/generation-fencing.md` 中对 `child_id(子任务标识)` 不作为高基数指标的约定。

**Constraints(约束)**: 不得在 `shutdown pipeline(关闭流水线)` 语义上分叉两套取消与截止时间规则。停止类命令在 `004-3-child-runtime-state-control` 中已约定的非强制中止语义不因本功能而变成默认可中止, 但重启路径允许在专用 `pending restart`(待重启) 上下文中按规格升级 `abort(强制中止)`。命名上 `Generation(代次)` 不得与 `Epoch(纪元)` 混用。

**Scale/Scope(规模和范围)**: 影响 `src/runtime/control_loop.rs` 中 `RestartChild(重启子任务)` 与 `spawn_child_start(派生子任务启动)` 路径, `ChildRuntimeState(子任务运行状态记录)` 模型, 退出处理与 `event`(事件), `observe`(可观测性), `dashboard`(仪表盘) 投影. 不引入多实例同名 `child`(子任务).

**Delayed spawn mailbox(延期派生邮箱)**: **`ChildStartMessage::DelayedSpawnAttached`(延迟附着启动子任务消息)** 是发往 **`runtime control loop`(运行时控制循环)** 的内部 **`mailbox`(邮箱)** 变体, **绑定** **`spec.md` `FR-004`** 与 **`SC-005`**: **`正 backoff`** 到期后 **`activate_instance`** 与 **`ChildRuntimeState`** 写回 **必须** 仍在 **`control loop`** 单线程轮次上发生, **不得**只在未再次进入 **`control loop`** 的 **`tokio::spawn`** 任务里悄悄完成绑定.

## Constitution Check(宪章检查)

*GATE(关口): Phase 0(研究阶段) 前必须通过。Phase 1(设计阶段) 后必须重新检查。*

### Phase 0(研究阶段) 前自检

- **Module Ownership(模块所有权)**: 代次隔离与退出报告判定放在 `runtime`(运行时) 与既有 `child_runtime_state`(子任务运行状态) 模块边界内。`policy`(策略) 仅产出重启决策, 不持有活动尝试句柄。对外 `pub` 扩展保持最小必要集合, 不新增宪章定义的 **兼容导出(compatibility exports)** (例如仅为兼容外部调用而加的 `pub use` 重导出, 别名模块路径或薄封装).

- **Supervision Contract(监督契约)**: 见下方专节。本功能改变监督行为, 不适用 `N/A(不适用)`.

- **Test Gate(测试关口)**: `tasks.md` 生成时必须遵守测试前置于实现的行为变化顺序. 类型化事件的发送阶段以 `tasks.md` 开篇 Event Timing(事件时序) 与 `contracts/generation-fencing.md` 内 Implementation phase note(实现阶段说明) 对齐, 避免契约与任务双源漂移. 自动化门禁至少覆盖 `quickstart.md` 第 2 节至第 6 节列出的 `cargo test` 类命令及全量 `cargo test`. 第 7 节「人工检查结果摘要」属人工核对清单, 由 `tasks.md` 中 **T030** 收口, 合并不与本条「自动化门禁」对立。

- **Observable Failures(可观察失败)**: 重启冲突, 停止失败, 合并重复请求, `StaleAttemptReport(过期尝试报告)` 必须有结构化结果或契约规定的 `ChildControlFailure(子任务控制失败)` 路径, 并配合事件, `audit`(审计) 与 `metrics`(指标), 能定位到 `child_id(子任务标识)`, 生命周期阶段与根因类别。

- **Small Increment(小增量)**: 不引入新异步运行时, 不引入新持久化层。代次隔离状态机复用现有单线程 `control loop(控制循环)` 与完成报告通道。若后续必须新增 `crate`(库), 必须补写理由与被拒绝的更简单方案。

- **Chinese Writing(中文写作)**: 本计划与 Phase 1(设计阶段) 产出已用中文完整句写作, 英文术语采用 `English(中文说明)` 格式. **`RunningInstanceId`(运行实例标识)** 读者向口径以 **`spec.md`** **Constitution Alignment(宪章对齐)** 中专节 **RunningInstanceId(运行实例标识) 与本功能术语** 为准.

### Supervision Contract Detail(监督契约专节)

下列对象均为 `supervised unit(受监督单元)` 语境下的子任务一次启动运行实例。

**Lifecycle states(生命周期状态) 与阶段表**

| `GenerationFencePhase(代次隔离阶段)` | 含义 | 允许的监督动作 |
|--------------------------------------|------|----------------|
| `Open(开放)` | 无活动尝试或活动尝试正常运行 | 非重启类命令按既有规则。任何新启动必须过公共启动门禁。 |
| `WaitingForOldStop(等待旧尝试停止)` | 已接受 `RestartChild(重启子任务)`, 已向旧尝试发取消 | 不得启动第二个活动尝试。控制循环按截止时间调和。 |
| `AbortingOld(正在中止旧尝试)` | 优雅窗口已过, 已请求 `abort(强制中止)` | 仍不得启动新尝试, 直到退出报告对齐 `pending restart`(待重启) 的旧身份。 |
| `ReadyToStart(可以启动新尝试)` | 旧尝试已收敛, `fence`(隔离边界)释放 | 仅允许在满足数据模型校验下启动目标代次新尝试一次。 |
| `Closed(关闭)` | 子任务已移除或监督树关闭中 | `spawn_child_start(派生子任务启动)` 不得启动新尝试。重启返回 `BlockedByShutdown(被关闭阻止)` 或等价结构化结论。 |

**启动**: 只有通过 `spawn_child_start(派生子任务启动)` 及其调用链创建的尝试才算新的 `attempt(尝试)`。自动重启与手动重启在同一入口前检查是否已有活动尝试或待重启冲突。

**停止与取消**: `pending restart`(待重启) 路径必须先向当前活动尝试送达取消, `stop_deadline_at_unix_nanos(停止截止时间)` 由取消送达时刻加当前生效 `ShutdownPolicy.graceful_timeout(关闭策略优雅等待时间)` 推导, 与其它控制命令对齐时间基准语义。

**重启**: `RestartChild(重启子任务)` 接受后返回结构化 `GenerationFenceOutcome(代次隔离结果)`, 不阻塞等待退出。新旧代次交接只在退出报告与 `fencing`(隔离规则)一致后发生。

**超时**: 截止时间到达且仍未退出时可升级请求 `abort(强制中止)`. 不得在请求中止的同一刻启动新代次尝试。

**关闭**: `ShutdownTree(关闭监督树)` 进行中时重启不得引入新尝试, 结果必须可区分于普通冲突。

**调用者可见错误**: 未知 `child`(子任务), 监督树关闭, 不可执行的冲突等沿用或扩展结构化 `ChildControlFailure(子任务控制失败)`。过期报告不产生可恢复的操作失败, 但产生可观测事件。

### Phase 1(设计阶段) 产出冻结后复查

- **Module Ownership(模块所有权)**: `data-model.md` 与 `contracts/generation-fencing.md` 将状态与 `API`(接口) 边界固定到 `runtime`(运行时), `event`(事件), `observe`(可观测性), `dashboard`(仪表盘) 模块, 与宪章一致。

- **Supervision Contract(监督契约)**: 专节与 `contracts/generation-fencing.md` 中 Runtime Semantics(运行时语义) 一致, 无未写明的默认。

- **Test Gate(测试关口)**: `quickstart.md` 划出验证顺序外包络. 实现阶段必须先完成 `tasks.md` 中与代次隔离相关的集成测试任务条目, 再在同一分支合入 `RestartChild`(重启子任务) 主体生产路径. 类型化事件的发送分段必须以 `tasks.md` 开篇 Event Timing(事件时序) 小段与 `contracts/generation-fencing.md` 内 Implementation phase note(实现阶段说明) 对齐, 口径与上文 Phase 0(研究阶段) 前自检中 Test Gate(测试关口) 条目一致, 含 **quickstart.md** 第 7 节人工核对由 **T030** 执行而非单独的 CI 自动化门禁.

- **Observable Failures(可观察失败)**: 契约要求事件与 `metrics`(指标) 清单完整, 满足 Principle IV(原则四).

- **Small Increment(小增量)**: 研究结论未引入计划外子系统。

- **Chinese Writing(中文写作)**: Phase 1(设计阶段) 文档已符合 Principle VI(原则六). **`RunningInstanceId`(运行实例标识)** 口径仍以 **`spec.md`** **Constitution Alignment(宪章对齐)** 中专节 **RunningInstanceId(运行实例标识) 与本功能术语** 为准.

## Project Structure(项目结构)

### Documentation(文档，本功能)

```text
specs/004-4-generation-fencing/
├── plan.md              # 本文件
├── research.md          # Phase 0(研究阶段) 输出
├── data-model.md        # Phase 1(设计阶段) 输出
├── quickstart.md        # Phase 1(设计阶段) 输出
├── contracts/           # Phase 1(设计阶段) 输出
│   └── generation-fencing.md
├── checklists/          # 需求核查清单
└── tasks.md             # Spec Kit `/speckit-tasks` 产出的任务列表, 勿混读为本文 Phase 编号
```

### Source Code(源代码，仓库根目录)

```text
src/
├── lib.rs
├── main.rs
├── runtime/
│   ├── control_loop.rs          # RestartChild(重启子任务), spawn_child_start(派生子任务启动), 自动重启范围
│   ├── child_runtime_state.rs   # ChildRuntimeState(子任务运行状态记录), 代次隔离状态嵌入点
│   ├── lifecycle.rs
│   ├── message.rs
│   ├── supervisor.rs
│   └── ...
├── child_runner/                # ChildRunner(子任务运行器), 启动与完成报告
├── control/                     # ControlCommand(控制命令), ChildControlResult(子任务控制结果)
├── event/                       # 类型化事件 payload(载荷)
├── observe/                     # pipeline(流水线), metrics(指标)
├── dashboard/                   # protocol(协议), model(模型), ipc_server(进程间通信服务)
└── shutdown/                    # 关闭阶段与协调, 复用语义
src/tests/                       # 多数 [[test]] 注册的外部集成测试
tests/
└── dashboard_protocol_shape_test.rs   # 根目录自动发现, 协议与返回形状回归
```

**Structure Decision(结构决定)**: 采用仓库既有单 `crate`(包) 布局。代次隔离核心逻辑落在 `src/runtime/` 与 `src/runtime/child_runtime_state.rs`, 与 `src/child_runner/` 的启动报告边界清晰。可观测性与对外展示分别落在 `src/event/`, `src/observe/`, `src/dashboard/`。不新增平行 `backend/` 或 `frontend/` 目录。

## Complexity Tracking(复杂度跟踪)

本功能实现未在根目录 `Cargo.toml` 中新增外部 **`crate`(库)** 依赖条目. **过期报告测试钩子**的权威叙述见上文 **`Stale report test replay`**. **`generation_fencing_replay_child_exit_for_test`** **不是**出货面稳定 **`API`(应用程序接口)**. **`README`(仓库说明)** 若必须提及该钩子, **应只**引用 **`plan.md`** 本段或 **`contracts/generation-fencing.md`** **`Stale Report`** 专节, **避免**口径双源分叉.
