# Feature Specification(功能规格): 运行时生命周期守卫

**Feature Branch(功能分支)**: `004-runtime-semantics`
**Created(创建日期)**: 2026-05-14
**Status(状态)**: Draft(草稿)
**Input(输入)**: 用户描述: "第一阶段, 先修正运行时语义. 当前 Supervisor::start_with_policy 会启动 runtime control loop(运行时控制循环), 但是没有把 JoinHandle(任务句柄) 纳入 SupervisorHandle(监督器控制句柄) 管理. 工业级改造应该把控制循环本身作为受监督对象, 保存 JoinHandle(任务句柄), 建立 runtime watchdog(运行时看门狗), 并在控制循环异常退出时发出 typed event(类型化事件), metrics(指标), audit log(审计日志), 同时让 SupervisorHandle(监督器控制句柄) 暴露 is_alive, join, shutdown, health 这类能力."

## User Scenarios & Testing(用户场景和测试) *(mandatory(必填))*

### User Story 1(用户故事一) - 查询运行时健康状态 (Priority(优先级): P1)

操作者需要在 Supervisor(监督器) 启动后立即知道 runtime control loop(运行时控制循环) 是否仍在运行, 而不是等到下一次控制命令失败时才发现控制面已经关闭.

**Why this priority(为什么是这个优先级)**: 控制循环是所有控制命令和状态读取的入口. 如果这个入口失效却不可见, 操作者无法判断系统是否仍可治理.

**Independent Test(独立测试)**: 启动一个 Supervisor(监督器), 立即读取 SupervisorHandle(监督器控制句柄) 的健康状态, 验证结果明确显示控制循环处于 alive(存活) 状态.

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** Supervisor(监督器) 已成功启动, **When(当)** 操作者读取健康状态, **Then(则)** 系统必须返回 alive(存活), 控制循环状态, 启动时间和最近观测时间.
2. **Given(假设)** Supervisor(监督器) 已成功启动, **When(当)** 操作者订阅运行时事件, **Then(则)** 系统必须允许操作者看到控制循环启动事件.

---

### User Story 2(用户故事二) - 提前发现控制循环异常退出 (Priority(优先级): P2)

操作者需要在 runtime control loop(运行时控制循环) 异常退出时收到结构化故障信号, 不能只在后续命令发送失败时看到模糊的通道关闭错误.

**Why this priority(为什么是这个优先级)**: 工业系统需要主动暴露控制面故障, 这样监控和告警可以在业务命令再次到来前触发.

**Independent Test(独立测试)**: 通过测试入口让控制循环异常退出, 验证 watchdog(看门狗) 在没有新控制命令的情况下也发出 typed event(类型化事件), metrics(指标) 和 audit log(审计日志).

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** 控制循环异常退出, **When(当)** watchdog(看门狗) 观察到退出结果, **Then(则)** 系统必须记录异常阶段, 退出原因和是否可恢复.
2. **Given(假设)** 控制循环已经异常退出, **When(当)** 操作者读取健康状态, **Then(则)** 系统必须返回 not alive(非存活) 和结构化失败原因.

---

### User Story 3(用户故事三) - 等待和关闭运行时控制面 (Priority(优先级): P3)

操作者需要通过 SupervisorHandle(监督器控制句柄) 等待控制循环结束, 或主动关闭控制面, 并获得明确结果.

**Why this priority(为什么是这个优先级)**: 调用方需要在服务退出, 集成测试和运维脚本中判断运行时是否已经完全结束.

**Independent Test(独立测试)**: 操作者请求关闭控制面后等待结束, 验证等待结果只返回一次最终状态, 重复调用保持幂等.

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** 控制循环正在运行, **When(当)** 操作者请求关闭并等待结束, **Then(则)** 系统必须返回 completed(已完成) 或 failed(失败) 的结构化结果.
2. **Given(假设)** 控制循环已经结束, **When(当)** 操作者重复等待, **Then(则)** 系统必须返回同一个最终结果, 不得挂起.

### Edge Cases(边界情况)

- 控制循环启动后立即退出时, 系统必须仍然产生故障事件和健康状态.
- watchdog(看门狗) 自身无法发布事件时, 健康状态必须保留失败原因.
- 操作者在控制循环结束后发送命令时, 系统必须返回包含已知退出原因的结构化错误.

## Requirements(需求) *(mandatory(必填))*

### Functional Requirements(功能需求)

- **FR-001**: 系统必须把 runtime control loop(运行时控制循环) 作为 SupervisorHandle(监督器控制句柄) 可拥有和可观察的受监督对象, 并保存其生命周期句柄.
- **FR-002**: 系统必须在 runtime control loop(运行时控制循环) 异常退出时主动发出 typed event(类型化事件), metrics(指标), audit log(审计日志) 和结构化健康状态.
- **FR-003**: SupervisorHandle(监督器控制句柄) 必须提供 alive(存活), health(健康), join(等待结束) 和 shutdown(关闭) 语义, 并且这些语义必须幂等和可测试.

### Key Entities(关键实体)

- **RuntimeControlPlane(运行时控制面)**: 表示控制循环和它的生命周期状态.
- **RuntimeWatchdog(运行时看门狗)**: 表示观察控制循环退出结果并发布诊断的运行时监督单元.
- **RuntimeHealthReport(运行时健康报告)**: 表示调用者可见的控制面健康状态, 最近观测时间和失败原因.

## Constitution Alignment(宪章对齐) *(mandatory(必填))*

### Supervision Contract(监督契约)

- **Lifecycle impact(生命周期影响)**: 本规格改变控制循环的启动, 监控, 关闭和等待语义.
- **Failure behavior(失败行为)**: 控制循环异常退出必须变成调用者可见的结构化故障, 不能只表现为后续命令通道关闭.
- **Shutdown behavior(关闭行为)**: 控制面关闭必须支持幂等等待, 并返回最终运行时状态.

### Rust Boundary and Observability Requirements(Rust 边界和可观察性需求)

- **Module ownership(模块所有权)**: runtime(运行时) 模块拥有控制面生命周期, control(控制) 模块只暴露调用者可见的句柄能力.
- **Compatibility exports(兼容导出)**: None(无)
- **Diagnostics(诊断)**: 必须提供控制循环启动, 正常退出, 异常退出, 等待完成和关闭请求的结构化诊断.
- **Dependency impact(依赖影响)**: 不预设新增 crate(库). 如果实现阶段需要新增依赖, plan(计划) 必须说明理由.

### Chinese Writing(中文写作)

- **Writing language(写作语言)**: 本文档必须使用中文.
- **Term format(术语格式)**: 英文术语必须写成 `English(中文说明)`.
- **Forbidden style(禁止风格)**: 禁止非中文写作, 片段式语言, 生僻词和方言.

## Success Criteria(成功标准) *(mandatory(必填))*

### Measurable Outcomes(可衡量结果)

- **SC-001**: 控制循环异常退出后, 100% 的测试场景都能在下一次控制命令发送前读取到 not alive(非存活) 健康状态.
- **SC-002**: 控制循环异常退出后, 100% 的测试场景都能获得包含阶段和原因的 typed event(类型化事件).
- **SC-003**: 对同一个已结束运行时重复调用 join(等待结束) 10 次, 每次都必须在 1 秒内返回相同最终结果.
- **SC-004**: 正常启动的 Supervisor(监督器) 在 100% 的健康查询中返回 alive(存活) 状态.

## Assumptions(假设)

- 本规格只覆盖当前核心库中的运行时控制面, 不覆盖 relay(中继) 或 dashboard client(看板客户端).
- 控制循环异常退出默认视为运行时故障, 不默认自动重启控制循环.
- 后续规格会处理真实 child task(子任务) 关闭和代次隔离.
