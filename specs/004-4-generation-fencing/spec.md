# Feature Specification(功能规格): 代次隔离重启

**Feature Branch(功能分支)**: `004-runtime-semantics`
**Created(创建日期)**: 2026-05-14
**Status(状态)**: Draft(草稿)
**Input(输入)**: 用户描述: "当前 RestartChild(重启子任务) 直接生成一个新 attempt(尝试), 没有先停止旧 attempt(尝试). 如果旧任务还没有退出, 可能出现同一个 child id(子任务标识) 有多个运行实例. 正确做法是引入 generation fencing(代次隔离): 每个 child runtime state(子任务运行状态记录) 同一时间最多一个 active attempt(活动尝试), 重启前先发取消, 等待或中止旧任务, 新任务启动时检查 generation(代次), 旧任务迟到上报时必须被丢弃或记为 stale report(过期报告)."

## User Scenarios & Testing(用户场景和测试) *(mandatory(必填))*

### User Story 1(用户故事一) - 重启前停止旧尝试 (Priority(优先级): P1)

操作者执行 RestartChild(重启子任务) 时, 系统必须先停止旧 attempt(尝试), 再启动新 attempt(尝试), 避免同一个 child(子任务) 同时运行两个实例.

**Why this priority(为什么是这个优先级)**: 重复活动实例可能同时消费消息, 写数据库, 持有锁或处理支付, 这是工业系统中的高风险行为.

**Independent Test(独立测试)**: 启动一个长运行任务, 请求重启, 验证新尝试只有在旧尝试进入停止结果后才会启动.

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** 子任务正在运行, **When(当)** 操作者执行 RestartChild(重启子任务), **Then(则)** 系统必须先向旧尝试发送取消信号.
2. **Given(假设)** 旧尝试在关闭时间限制内退出, **When(当)** 系统启动新尝试, **Then(则)** 新尝试必须使用新的 generation(代次).

---

### User Story 2(用户故事二) - 每个子任务只有一个活动尝试 (Priority(优先级): P2)

操作者需要系统保证一个 child id(子任务标识) 在任意时刻最多只有一个 active attempt(活动尝试), 包括手动重启和自动重启.

**Why this priority(为什么是这个优先级)**: 单实例约束是消息消费, 外部写入和锁管理任务的安全前提.

**Independent Test(独立测试)**: 同时触发手动 RestartChild(重启子任务) 和自动重启决策, 验证运行状态记录不会启动第二个活动尝试.

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** 旧尝试尚未结束, **When(当)** 第二个重启请求到达, **Then(则)** 系统必须拒绝, 合并或排队该请求, 但不得启动第二个活动尝试.
2. **Given(假设)** 自动重启和手动重启同时发生, **When(当)** 运行状态记录做出决策, **Then(则)** 系统必须保留一个明确的活动尝试.

---

### User Story 3(用户故事三) - 处理迟到的旧代报告 (Priority(优先级): P3)

操作者需要系统在旧 generation(代次) 的任务迟到上报时保护新状态, 并给出可审计诊断.

**Why this priority(为什么是这个优先级)**: 旧任务迟到上报如果覆盖新状态, 会让操作者看到错误的生命周期结论.

**Independent Test(独立测试)**: 启动新 generation(代次) 后模拟旧 generation(代次) 的退出报告, 验证系统把它丢弃或记录为 stale report(过期报告), 且不会覆盖新尝试状态.

**Acceptance Scenarios(验收场景)**:

1. **Given(假设)** 新 generation(代次) 已经运行, **When(当)** 旧 generation(代次) 迟到上报退出, **Then(则)** 系统不得用旧报告覆盖新子任务尝试状态.
2. **Given(假设)** 旧报告被判定为 stale report(过期报告), **When(当)** 操作者查看事件或诊断, **Then(则)** 系统必须显示旧代次, 当前代次和处理结果.

### Edge Cases(边界情况)

- 旧尝试拒绝响应取消时, 重启流程必须走强制中止路径.
- 新尝试启动失败时, 运行状态记录必须保留旧尝试的最终结果和新尝试失败原因.
- 重启请求重复到达时, 系统必须保持幂等或返回明确的冲突结果.

## Requirements(需求) *(mandatory(必填))*

### Functional Requirements(功能需求)

- **FR-001**: RestartChild(重启子任务) 必须先停止当前 active attempt(活动尝试), 再启动新的 generation(代次) 和 attempt(尝试).
- **FR-002**: 每个 child runtime state(子任务运行状态记录) 在任意时刻最多只能拥有一个 active attempt(活动尝试), 手动重启和自动重启都必须遵守该约束.
- **FR-003**: 旧 generation(代次) 的迟到报告必须被丢弃或记录为 stale report(过期报告), 并且不得覆盖当前 generation(代次) 的子任务尝试状态.

### Key Entities(关键实体)

- **Generation(代次)**: 表示同一个 child(子任务) 跨重启产生的新旧运行实例编号. 它用于识别迟到报告和当前运行实例, 不是时间戳起点. 文档必须统一使用本术语, 不得使用其他中文名.
- **Attempt(尝试)**: 表示某次实际启动出来的任务尝试. 新 generation(代次) 启动时必须产生新的活动 attempt(尝试), 旧 attempt(尝试) 的迟到报告不得覆盖当前状态.
- **Epoch(纪元)**: 表示时间戳起点, 例如 `UNIX_EPOCH(Unix 纪元常量)`. 它只能用于时间戳语义, 不能用于表示 generation(代次).
- **GenerationFence(代次隔离)**: 表示用 generation(代次) 保护子任务尝试状态的规则.
- **ActiveAttempt(活动尝试)**: 表示 child runtime state(子任务运行状态记录) 当前唯一允许运行的任务尝试.
- **StaleReport(过期报告)**: 表示旧 generation(代次) 在新 generation(代次) 生效后到达的报告.

## Constitution Alignment(宪章对齐) *(mandatory(必填))*

### Supervision Contract(监督契约)

- **Lifecycle impact(生命周期影响)**: 本规格改变手动重启, 自动重启和任务报告接收语义.
- **Failure behavior(失败行为)**: 重启冲突, 停止失败和过期报告都必须返回结构化诊断.
- **Shutdown behavior(关闭行为)**: 重启前停止旧尝试必须复用关闭流水线中的取消, 等待和强制中止语义.

### Rust Boundary and Observability Requirements(Rust 边界和可观察性需求)

- **Module ownership(模块所有权)**: runtime(运行时) 模块负责代次隔离和报告接收, policy(策略) 模块只产出重启决策.
- **Compatibility exports(兼容导出)**: None(无)
- **Diagnostics(诊断)**: 必须记录重启请求, 旧尝试停止结果, 新代次启动, 重启冲突和 stale report(过期报告).
- **Dependency impact(依赖影响)**: 不预设新增 crate(库). 如果实现阶段需要新增依赖, plan(计划) 必须说明理由.

### Chinese Writing(中文写作)

- **Writing language(写作语言)**: 本文档必须使用中文.
- **Term format(术语格式)**: 英文术语必须写成 `English(中文说明)`.
- **Forbidden style(禁止风格)**: 禁止非中文写作, 片段式语言, 生僻词和方言.

## Success Criteria(成功标准) *(mandatory(必填))*

### Measurable Outcomes(可衡量结果)

- **SC-001**: 在 100% 的重启测试场景中, 同一个 child id(子任务标识) 同一时刻最多只有一个 active attempt(活动尝试).
- **SC-002**: 旧尝试未结束时发起重启, 100% 的场景都能观察到先停止旧尝试再启动新尝试的顺序.
- **SC-003**: 旧 generation(代次) 的迟到报告在 100% 的测试场景中不会覆盖当前 generation(代次) 状态.
- **SC-004**: 重启冲突场景中, 100% 的命令结果都包含冲突或排队处理结论.

## Assumptions(假设)

- 本规格依赖 `004-3-child-runtime-state-control` 提供的 child runtime state(子任务运行状态记录) 和真实活动尝试状态.
- 手动重启和自动重启必须使用同一个代次隔离规则.
- 本规格不要求支持多个并行实例的同名 child(子任务), 如需并行实例必须通过不同 child id(子任务标识) 表达.
