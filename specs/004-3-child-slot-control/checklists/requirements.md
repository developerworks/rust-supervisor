# Specification Quality Checklist(规格质量检查清单): 子任务槽位控制

**Purpose(目的)**: 在进入 plan(计划) 前检查规格是否完整, 可测试, 并且与运行时语义边界一致.
**Created(创建日期)**: 2026-05-14
**Updated(更新日期)**: 2026-05-15
**Feature(功能规格)**: [spec.md](../spec.md)

## Content Quality(内容质量)

- [X] 本功能规格为技术向文档: 允许使用 `runtime(运行时)`, `child slot(子任务槽位)`, `CancellationToken(取消令牌)`, `join_handle(任务句柄)`, `heartbeat(心跳)`, `ready_state(就绪状态)` 和 `restart_budget(重启预算)` 等实现向术语, 因为本功能直接修正控制命令与真实任务生命周期之间的语义边界.
- [X] 规格以操作者和库调用者可验证的槽位事实为中心, 包括活动尝试, 取消送达, 等待结果, 幂等结果, 心跳, 就绪状态和重启预算余量.
- [X] 主要读者是维护者和实现者, 可以与后续 `plan.md`, `data-model.md`, `contracts/` 和 `tasks.md` 交叉阅读, 不要求非技术干系人单独读懂所有运行时细节.
- [X] 所有必填章节已经完成, 并且每个用户故事都能映射到可执行的测试场景.

## Requirement Completeness(需求完整性)

- [X] No [NEEDS CLARIFICATION] markers remain
- [X] 功能需求可测试且边界清楚, 每条需求都说明了 child slot(子任务槽位), 停止类控制命令或控制结果的真实行为.
- [X] 成功标准可度量, 并且能够通过状态读取, 控制命令结果和运行时测试验证.
- [X] 成功标准允许引用运行时字段和控制命令行为, 因为本功能的验收对象就是真实槽位状态, 不是抽象业务流程.
- [X] 所有验收场景已经定义, 覆盖状态读取, 停止真实任务, 幂等返回和失败原因.
- [X] 边界情况已经列出, 覆盖未收到心跳, 心跳后立即退出, 自动重启推进代际, 重启预算耗尽, 就绪状态缺失和控制命令并发.
- [X] 范围边界清楚, 本功能不新增动态子任务声明格式, 不改变 supervision strategy(监督策略) 的重启决策算法.
- [X] 依赖与假设已经写明, 本功能依赖 `004-2-real-shutdown-pipeline` 的取消和等待语义.

## Feature Readiness(功能就绪度)

- [X] 每条 functional requirement(功能需求) 都有对应的用户故事, 验收场景和 measurable outcome(可衡量结果).
- [X] 用户场景覆盖三条主路径: 读取真实槽位状态, 停止真实活动任务, 让控制结果反映槽位事实.
- [X] 成功标准与规格正文一致, 不再声称本规格是 technology-agnostic(技术无关) 或面向 non-technical stakeholders(非技术干系人).
- [X] 实现向术语只用于定义运行时事实和控制命令边界, 没有绑定新增 crate(库), 外部服务, 数据库或部署方式.

## Notes

- 2026-05-15 修订: 原检查清单错误沿用了通用非技术模板, 与当前技术向运行时规格发生冲突. 本次已改为运行时语义检查口径.
- `generation(代际)`, `attempt(尝试)`, `cancellation_token(取消令牌)`, `join_handle(任务句柄)`, `last_heartbeat(最后心跳)`, `ready_state(就绪状态)` 和 `restart_budget(重启预算)` 是本功能必须表达的槽位事实, 不是需要隐藏的实现泄漏.
- 后续 `/speckit-plan` 必须以 `004-2-real-shutdown-pipeline` 已交付的取消和等待语义为基础, 不另起一套关闭路径.
