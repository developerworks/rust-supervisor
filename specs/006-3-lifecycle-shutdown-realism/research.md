# Research(研究): ChildSlot 并发安全模型

**Feature(功能)**: 006-3-lifecycle-shutdown-realism | **Date(日期)**: 2026-05-18
**Status(状态)**: Completed(已完成)

## 概述

本文记录将 `RuntimeControlState` 中的 `children: HashMap<ChildId, ManagedChildState>` 升级为 `slots: HashMap<ChildId, ChildSlot>` 过程中必须解决的并发安全问题及其设计决策. 研究的核心问题: 如何保证同一 `ChildId` 在任意时刻至多有一条 active attempt(活动尝试), 同时允许并发生命周期命令不破坏该不变式.

## 研究问题与决策

### 问题 1: ChildSlot 中的 CancellationToken 与 JoinHandle 如何在并发命令间安全共享?

**Decision(决策)**: 使用 `tokio::sync::Mutex` 保护每个 `ChildSlot` 的可变访问, 但控制循环中已存在对 `slots: HashMap<ChildId, ChildSlot>` 的排他性 `&mut` 借用(因为控制循环是单线程 Tokio 任务). 因此不需要 `Mutex` 包装. `CancellationToken` 内部已实现原子操作, `JoinHandle` 的 `abort()` 方法自身是线程安全的.

**Rationale(理由)**:

- Tokio 控制循环单任务执行, 对 `slots` 的访问天然串行.
- `CancellationToken` 是 Tokio 提供的并发安全原语, 内部使用 `AtomicBool`.
- 不引入额外锁降低复杂度.

**Alternatives considered(已考虑的替代方案)**:

- `Arc<Mutex<ChildSlot>>`: 增加堆分配和锁竞争, 无收益因为 Tokio 任务本身就是串行的.
- `RwLock`: 读多于写时有用, 但控制循环中每个命令都需要可变写入, 无益.

### 问题 2: 并发 restart 请求如何保证至多一条进入 execute(执行)

**Decision(决策)**: 引入 `AdmissionSet` (承认集合). 这是一个 `HashSet<ChildId>` 存储在 `RuntimeControlState` 中, 控制循环在激活 `ChildSlot` 的 `activate()` 前必须先 `try_admit()`. 成功后插入 `HashSet`, 活动结束时 `release()`. 同时校验 `ChildSlot.pending_restart` 标志来拒绝在已有待重启期间的额外重启请求.

**Rationale(理由)**:

- `HashSet` 提供 O(1) 查找, 适合高频率准入检查.
- 控制循环的单线程特性保证了 `try_admit/release` 无竞态.
- 幂等支持 (`try_admit_or_idempotent`) 允许调用方安全重试.

**Alternatives considered(已考虑的替代方案)**:

- 使用 Tokio `Semaphore` 每 child 一个许可: 增加异步开销但仅用于同步准入.
- 比较并交换 (CAS) 原子变量: 简单场景可行, 但无法携带 generation/attempt 审计信息.
- 直接在 `ChildSlot` 内用 `bool` 标志: 无法区分"正在运行"和"尚未启动", 且审计信息不足.

### 问题 3: shutdown_tree 扇出时的超时与 abort 策略

**Decision(决策)**: 采用 `ShutdownPolicy` 配置的两阶段超时: `graceful_timeout` 用于协作取消, `abort_wait` 用于强制中止. 实现 `shutdown_tree_fanout` 函数执行扇出逻辑: cancel → 等 `graceful_timeout` → abort → 等 `abort_wait` → force-deactivate 残余.

**Rationale(理由)**:

- 两阶段超时符合 Kubernetes Pod 终止模型的最佳实践.
- `Tokio::time::timeout` 与 `JoinHandle::abort()` 组合已充分验证.
- force-deactivate 最终路径保证任何情况下都不留悬挂句柄.

**Alternatives considered(已考虑的替代方案)**:

- 单阶段超时: 无法区分协作退出与强制终止, 增加误杀风险.
- 无限等待: 导致关停悬挂, 违反无孤儿保证.
- 每个 child 独立的超时: 复杂度高且无实际收益.

### 问题 4: 如何验证"至多一条执行线"不变式

**Decision(决策)**:

- 验收测试使用 `try_admit()` 返回的 `AdmissionConflict` 统计违反次数.
- 预期: 1_000 次并发 restart 请求下违反至多一条的次数为 0(SC-001).
- 生产代码中 `AdmissionConflict` 携带 `generation` 和 `attempt` 值用于审计对账.

**Rationale(理由)**:

- `AdmissionConflict` 是 `#[must_use]` 类型(通过 Result 传播), 保证调用方必须处理.
- `Display` 实现包含 `gen{value}-attempt{value}` 格式, 方便 grep 审计.

**Alternatives considered(已考虑的替代方案)**:

- 运行时全局计数器: 只能知道"发生过"但无法追溯到具体请求.
- panic: 不适合生产, 因为并发请求不应导致服务崩溃.

## 技术预研结论

| 问题域             | 选择方案                                         | 是否引入新 crate |
| ------------------ | ------------------------------------------------ | ---------------- |
| ChildSlot 并发访问 | 单线程控制循环 + `&mut` 借用                     | 否               |
| 准入控制           | `AdmissionSet` (HashSet + 幂等检查)              | 否               |
| 关停超时           | 两阶段 `ShutdownPolicy` + `tokio::time::timeout` | 否               |
| 不变式验证         | `AdmissionConflict` structured error             | 否               |

**最终结论**: 现有 Tokio 原语 (CancellationToken, JoinHandle, timeout) 和 Rust 所有权模型已足够支持本切片的所有并发安全需求. 无需引入新依赖.
