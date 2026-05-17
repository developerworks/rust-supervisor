# Feature Specification (功能规格): 真实生命周期与无孤儿关停

**Feature Branch (功能分支)**: `[006-3-lifecycle-shutdown-realism]`
**Created (创建日期)**: 2026-05-17
**Status (状态)**: Draft (草稿)
**Input (输入)**: 本规格对应第一序列里程碑: 让 start, restart, pause, resume, remove, quarantine, shutdown 都能真实作用到任务句柄和取消令牌上. 同一个 child id(子任务标识) 不得并发运行两个 activity attempt(活动尝试). shutdown_tree 必须让长时间运行的任务收到取消并退出, 超时后中止任务, 所有任务都能被 join(等待结束), 当前状态能返回每个 child(子任务) 的真实状态.

## Dependency Note (依赖说明)

本切片与 `specs/004-1-runtime-lifecycle-guard/spec.md`, `specs/004-2-real-shutdown-pipeline/spec.md`, `specs/004-3-child-runtime-state-control/spec.md`, `specs/004-4-generation-fencing/spec.md` 递进衔接. 规格层面的生命周期契约已在 004 系列中定义, 本切片的目标是把"状态标记型监督"升级为"真实生命周期治理型监督". 核心变化是: `RuntimeControlState` 中的 `children: HashMap<ChildId, ManagedChildState>` 升级为 `slots: HashMap<ChildId, ChildSlot>`, 每个 ChildSlot 绑定取消令牌和 join handle, 而不是靠内存字段状态机自述.

## User Scenarios & Testing (用户场景和测试) _(mandatory (必填))_

### User Story 1 (用户故事一) - 关停信号真实传给目标任务 (Priority (优先级): P1)

运行时操作员需要每次 shutdown(关停) 或 cancel(取消) 指令真实触发目标任务的取消令牌, 并在任务超时后执行 abort(中止). 操作员不能只看到内存状态变成"stopped"却让外部进程继续存活.

**Why this priority (为什么是这个优先级)**: 状态假阳性会把事故范围放大到错误的主机分区.

**Independent Test (独立测试)**: 为每类生命周期指令各装一个低成本探针. 进程存活位图, SIGUSR1(用户自定义信号一号) 计数器, 或显式睡眠任务收到取消的时刻戳. 对照事件流与 status(状态视图) 行.

**Acceptance Scenarios (验收场景)**:

1. **Given (假设)** 子任务线程或子进程卡在 sleep(休眠) 循环, **When (当)** 操作员下发 shutdown_tree(关停树), **Then (则)** 在文档写明的超时点之前必须出现取消令牌被消费或宿主等价的 kill(终止信号) 证据, 并在事件里携带阶段名与截止时刻.
2. **Given (假设)** 关停宽限时间配置得比恶意忽略取消的任务还短, **When (当)** 宽限耗尽, **Then (则)** 必须走 abort(中止) 分支, 仍然能 join(等待收敛) 到终态, 且 ChildSlot(子任务槽) 中不再残留进行中尝试.

### User Story 2 (用户故事二) - 同一 child id 最多一条活动执行线 (Priority (优先级): P1)

编排脚本作者需要监督器保证: 对于同一 child id(子任务标识), ChildSlot(子任务槽) 的 pending_restart 与 active attempt(活动尝试) 始终互斥. 并发风暴不能把准许集合拆成多条仍在跑的执行线.

**Why this priority (为什么是这个优先级)**: 双开执行线会打穿预算与健康检查语义.

**Independent Test (独立测试)**: 仿真 1_000 次并发 restart(重启) 请求. 统计 ChildSlot(子任务槽) 快照行数, 对照日志里冲突或幂等命中次数.

**Acceptance Scenarios (验收场景)**:

1. **Given (假设)** 两个客户端几乎同时调用 restart(重启), **When (当)** 调度器完成仲裁, **Then (则)** 只有一条调用进入真实执行上下文, 另一条要么收到 structured error(结构化错误) 要么收到与先成功的响应完全一致的幂等回包, 且审计流水能区分这两种归并路径.

### User Story 3 (用户故事三) - join 在所有生命周期路径上都可达 (Priority (优先级): P2)

收尾自动化工程师需要关停流程结束后, FD(文件描述符) 与内部 join handle(异步等待句柄) 集合要么清空, 要么只剩文档写明的那一小段延迟释放窗口. 不得在宿主机上留下孤儿进程.

**Why this priority (为什么是这个优先级)**: 句柄与进程残留会伪装成健康并持续吃光资源上限.

**Independent Test (独立测试)**: 比对关停前后的 /proc 或宿主 API(接口) 给出的句柄计数与事件里声明的 ShutdownPhase(关停阶段枚举) 完成位. 断言在窗口边界外计数回到基线.

**Acceptance Scenarios (验收场景)**:

1. **Given (假设)** 关停流程被事件标记为完成, **When (当)** 外部立刻查询拓扑 API(接口), **Then (则)** 每个 child id(子任务标识) 的状态列必须与最近一次退出码或取消原因摘要一致, 不允许出现 running(运行中) 却无外漂 PID(进程标识) 的矛盾组合.

### Edge Cases (边界情况)

- pause(暂停) 若无法一对一映射到宿主 SIGSTOP(作业控制暂停), 必须在发行说明里写明等价语义究竟是"暂停调度新工作"还是"冻结线程组", 并点名操作系统差异表.
- 嵌套监督器的关停顺序无论叶到根还是扇出并行, 每一层都必须留下 join(等待收敛) 完成的证据, 禁止内部层级遗留悬挂等待句柄.
- remove(移除) 与 quarantine(隔离) 并发命中同一 child id(子任务标识) 时, 必须写明胜出规则或序列化令牌, 以免并发写入撕裂拓扑视图.

## Requirements (需求) _(mandatory (必填))_

### Functional Requirements (功能需求)

- **FR-001**: 系统必须把 start, restart, pause, resume, remove, quarantine, shutdown_tree 七类指令各自的执行链路绑定到 cancellation(取消语义), join(等待收敛) 或宿主等价收口语义上. 禁止只在惰性缓存里改写状态标签而不触发外部副作用. 任一指令失败时必须返回 structured error(结构化错误).
- **FR-002**: 对于任一 child id(子任务标识), ChildSlot(子任务槽) 在任意时刻至多容纳一条 active attempt(活动尝试). 并发请求必须通过队列化, idempotency key(幂等键) 或可读冲突响应维持该不变式, 并在结构化事件里写明裁决序号.
- **FR-003**: shutdown_tree(关停树) 必须向下扇出 cancellation(取消语义). 超过文档时限仍未收敛的单元必须 abort(中止) 或写明宿主等价强制终止路径. join(等待收敛) 必须在全局配置上限时间内结束且不留悬挂执行上下文. status(状态视图) 各行必须与外部探针能看见的事实 (PID(进程标识) 是否存在, 最近一次退出摘要, 最近一次就绪时间戳) 在同一个误差窗口内相容.

### Key Entities (关键实体) _(涉及数据时填写)_

- **ChildSlot(子任务槽)**: 升级后的数据结构, 至少包含 status(状态), generation(代次), attempt(尝试计数), restart_count(重启计数), cancellation_token(取消令牌), join_handle(异步等待句柄), last_exit(最近一次退出摘要), last_ready_at(最近一次就绪时间戳), last_heartbeat_at(最近一次心跳时间戳), restart_window(重启窗口), pending_restart(待重启指示器). 取代当前 RuntimeControlState 中的 ManagedChildState.
- **AdmissionSet(承认集合)**: 描述当前已经被调度器准许进入真实执行阶段的 active attempt(活动尝试) 主键集合, 用作并发不变式断言输入.
- **ShutdownPhase(关停阶段枚举)**: 面向运维解释的关停扇出层级标签以及 shutdown_tree(关停树) 何时算本轮完结的外显枚举.

## Constitution Alignment (宪章对齐) _(mandatory (必填))_

### Supervision Contract (监督契约)

- **Lifecycle impact (生命周期影响)**: 高. 本条牵动默认监督语义, 必须与 004 系列契约并排评审.
- **Failure behavior (失败行为)**: 任一并发违例必须落成 structured error(结构化错误), 禁止无声复制第二条执行实例.
- **Shutdown behavior (关闭行为)**: join(等待收敛) 与 abort(中止) 的先后顺序以及全局超时上限必须在 shutdown(关闭) 小节写成表格.

### Rust Boundary and Observability Requirements (Rust 边界和可观察性需求)

- **Module ownership (模块所有权)**: ChildSlot 数据结构落在 src/runtime/ 树下, 运行时控制循环不能塞入 main.rs.
- **Compatibility exports (兼容导出)**: None (无).
- **Diagnostics (诊断)**: ChildSlot 中的 generation(代次) 和 RunningInstanceId(运行实例标识) 必须能在日志与人读 status(状态视图) JSON 中对账打印.
- **Dependency impact (依赖影响)**: 不适用, 除非计划新增系统调用级取消依赖.

### Chinese Writing (中文写作)

- **Writing language (写作语言)**: 本文档必须使用中文.
- **Term format (术语格式)**: 英文术语必须写成 English(中文说明).
- **Forbidden style (禁止风格)**: 禁止全角标点, 禁止口语省略主语的片段句, 禁止把英文形容词堆叠进汉语名词短语.

## Success Criteria (成功标准) _(mandatory (必填))_

### Measurable Outcomes (可衡量结果)

- **SC-001**: 并发重启压测 10_000 次请求下, 针对单个固定 child id(子任务标识), active attempt(活动尝试) 违反至多一条约束的次数为 0. 若出现违反样本则 100% 必须在同一时钟窗口内附带可读冲突 structured error(结构化错误).
- **SC-002**: 含慢任务的 shutdown_tree(关停树) 合成集成样例里, join(等待收敛) 100% 在全局上限内返回. 外部进程列表快照不得看见孤儿宿主进程.
- **SC-003**: status(状态视图) 与外部探针对照抽查 100 条记录里至少 99 条当场一致. 剩余延迟样本必须在文档阈值分钟内自愈或被标记为已知延迟窗口.

## Assumptions (假设)

- 宿主平台提供真实的 cancellation(取消语义) 或可被夹具模拟的等价路径. 若目标平台不支持, 按 006-1 支持矩阵表格标明.
- ChildSlot 的字段命名以计划阶段 data-model.md 冻结为准, 本规格只约束其存在性义务和行为语义.
