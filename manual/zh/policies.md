# 策略模型

语言: [English](../en/policies.html)

## 监督策略

`SupervisionStrategy`(监督策略)决定失败后的重启范围.

- `OneForOne`(一对一)只选择失败 child(子任务).
- `OneForAll`(一对全部)选择已选范围内的全部 child(子任务).
- `RestForOne`(从失败处开始)选择已选范围内的失败 child(子任务)和之后声明的 child(子任务).

`restart_scope` 根据 `SupervisorTree`(监督树), 策略和失败 child id(子任务标识)计算重启范围.

`restart_execution_plan`(重启执行计划函数) 会把 supervisor strategy(监督器策略), `GroupStrategy`(分组策略), `ChildStrategyOverride`(子任务级覆盖), `RestartLimit`(重启次数限制), `EscalationPolicy`(升级策略) 和 `DynamicSupervisorPolicy`(动态监督器策略) 合并成 `StrategyExecutionPlan`(策略执行计划). child override(子任务级覆盖) 优先于 group strategy(分组策略), group strategy(分组策略) 优先于 supervisor-wide strategy(监督器全局策略). plan(计划)中存储 `dynamic_supervisor_enabled`(动态监督启用)布尔值; 完整的 `DynamicSupervisorPolicy`(动态监督器策略)结构体(含 `child_limit`(子任务上限))由控制循环在 add_child 时评估, 不嵌入 plan(计划).

runtime control loop(运行时控制循环) 现在会接收 child exit(子任务退出), 并在 policy(策略) 返回重启决策时自动执行选定的 `StrategyExecutionPlan`(策略执行计划). 运行时生命周期事件包含重启范围信息, 让 operator(操作者) 可以看到选中的 strategy(策略), group(分组) 和 child scope(子任务范围).

## 分组策略和子任务覆盖

`GroupStrategy`(分组策略) 使用 child tag(子任务标签) 定义更小的重启范围. 一个 child(子任务) 最多只能属于一个已配置 strategy group(策略分组). `ChildStrategyOverride`(子任务级覆盖) 在单个 child(子任务) 需要比 group(分组) 或 supervisor(监督器) 更严格的重启行为时生效.

`GroupConfig`(分组配置)在分组层级配置重启预算, 成员资格和隔离. `GroupDependencyEdge`(分组依赖边)定义故障传播的跨分组依赖.

## 重启次数限制和升级策略

`RestartLimit`(重启次数限制) 记录选中计划的最大重启次数和计数窗口. `EscalationPolicy`(升级策略) 记录重启治理不能停留在本地时的后续动作, 包含 parent escalation(父级升级), tree shutdown(整棵树关闭) 或 scope quarantine(范围隔离).

## 动态监督器策略

`DynamicSupervisorPolicy`(动态监督器策略) 控制运行时 `add_child`(添加子任务) 是否被接受. 它是一个无状态配置对象, 包含 `enabled`(启用) 和 `child_limit`(子任务上限) 字段. 控制循环在外部维护当前子任务计数, 在 add_child 时调用 `allows_addition(current_child_count)`. 当 dynamic supervision(动态监督) 被禁用, 或已经达到配置的 child limit(子任务上限) 时, 添加会被拒绝.

## 重启策略

`RestartPolicy`(重启策略)包含 `Permanent`(永久), `Transient`(瞬时)和 `Temporary`(临时). `PolicyEngine`(策略引擎)读取 `TaskExit`(任务退出), 失败类别和重启策略, 输出 `RestartDecision`(重启决策).

## 退避和抖动

`BackoffPolicy`(退避策略)描述初始延迟, 最大延迟和 jitter(抖动)比例. 同名的类型有两个:

- `spec::child::BackoffPolicy` — 用于 `ChildSpec`(子任务规格), 字段为 `initial_delay`(初始延迟), `max_delay`(最大延迟), `jitter_ratio`(抖动比例, 0.0-1.0 浮点值).
- `policy::backoff::BackoffPolicy` — 由运行时策略引擎使用, 字段为 `initial`(初始), `max`(最大), `jitter_mode`(抖动模式, 枚举: `Disabled`(禁用), `Deterministic`(确定性), `FullJitter`(完全抖动), `DecorrelatedJitter`(解相关抖动)), `jitter_percent`(抖动百分比) 和 `reset_after`(稳定后重置).

测试可以使用 `JitterMode::Deterministic`(确定性抖动), 避免依赖随机结果.

## 熔断和隔离

`MeltdownPolicy`(熔断策略)限制一个窗口内的重启或失败次数, 分为 child(子任务), group(分组) 和 supervisor(监督器) 三个层级. 超过 child-level fuse(子任务级熔断)会进入 quarantine(隔离). 超过 group-level fuse(分组级熔断)会升级到 supervisor(监督器)层. 超过 supervisor-level fuse(监督器级熔断)会升级到父级.

## 任务退出分类

`TaskExit`(任务退出)区分成功, 取消, 类型化失败, panic(恐慌)和 timeout(超时). 策略层通过 `TaskFailureKind`(任务失败类别, 包含 `Panic`(恐慌), `Timeout`(超时)和类型化失败类别)读取类型化分类, 不应该从字符串推断行为.
