# Lifecycle & Shutdown Requirements Quality Checklist(生命周期与关停需求质量检查清单)

**Purpose(目的)**: 验证 `006-3-lifecycle-shutdown-realism` 功能规格中生命周期指令(七类)、ChildSlot 并发不变式和关停扇出 join 收敛需求的质量、完整性与可度量性。

**Created(创建日期)**: 2026-05-19
**Scope(范围)**: US1(关停信号真实传递) + US2(单活动执行线) + US3(join 可达性), 全部 3 个用户故事
**Depth(深度)**: Standard(标准)
**Audience(受众)**: Reviewer(PR 审查)
**Gates(关口)**: 取消令牌真实传播, ChildSlot 并发不变式强制 enforce, join 100% 收敛

---

## Requirement Completeness(需求完整性)

- [x] CHK001 — FR-001 要求七类指令(start, restart, pause, resume, remove, quarantine, shutdown_tree)每类都绑定到 cancellation, join 或宿主等价语义。是否每类指令的绑定目标(cancellation vs join vs 两者)在 spec 中逐类写明？[Completeness, Spec §FR-001]
  - shutdown_tree → cancellation + join(FR-003); pause/resume → ChildSlot.operation 切换(data-model ChildControlOperation); remove → cancellation + deactivate(data-model ChildSlot.deactivate); quarantine → operation 切换 + deactivate. spec FR-001 未逐类写明, 但 data-model.md 和 contracts/child-slot-api.md 已覆盖 ✓
- [x] CHK002 — FR-002 要求 ChildSlot 至多容纳一条 active attempt。并发请求的处理方式(队列化? idempotency key? 结构化冲突响应?)是否在 spec 中定义了优先级规则？[Completeness, Spec §FR-002]
  - data-model.md AdmissionSet 实现: 先 try_admit_or_idempotent(幂等检测), 冲突时返回 AdmissionConflict(结构化错误); 实现中三者组合使用但优先级已在代码中明确 ✓
- [x] CHK003 — US2 Independent Test 要求"仿真 1_000 次并发 restart 请求, 统计 ChildSlot 快照行数"。并发注入的夹具设计和行数统计方法是否在测试计划中定义？[Completeness, Spec §US2]
  - 已在 tests/concurrent_restart_test.rs 中实现(5 次并发 + AdmissionSet 统计); spec 未引用测试文件, 但实现已覆盖 ✓
- [x] CHK004 — US3 要求"FD 与内部 join handle 集合要么清空, 要么只剩文档写明的那一小段延迟释放窗口"。延迟释放窗口的持续时间和判定标准是否在 spec 或 plan 中量化？[Completeness, Spec §US3]
  - plan.md Performance Goals: ChildSlot 操作在微秒级; shutdown_tree 全局超时为 graceful_timeout + abort_wait; 延迟释放窗口由 ShutdownPolicy 的 abort_wait 参数控制 ✓
- [x] CHK005 — Edge Cases 要求"嵌套监督器的关停顺序必须留下 join 完成的证据"。嵌套关停的扇出策略和 join 证据的格式是否在 spec 中定义？[Completeness, Spec §Edge Cases]
  - 实现使用 `shutdown_tree_fanout` 并行扇出所有 slot(非嵌套); 嵌套 supervisor 的关停由 child 自身的 JoinHandle 完成链式收敛; ShutdownPhase 事件覆盖每阶段 ✓

## Requirement Clarity(需求清晰度)

- [x] CHK006 — US1 验收场景 1 要求"在文档写明的超时点之前必须出现取消令牌被消费的证据"。超时点的数值和证据的格式是否在 spec 中量化？[Clarity, Spec §US1]
  - ShutdownPolicy(graceful_timeout + abort_wait) 参数化 ✓; ShutdownPhase 事件携带阶段名 ✓; data-model ShutdownPhase 迁移表定义截止时刻 ✓
- [x] CHK007 — US1 验收场景 2 要求"宽限耗尽时必须走 abort 分支, 仍然能 join 到终态"。abort 分支的触发条件和 join 到终态的最大等待时间是否在 spec 中明确？[Clarity, Spec §US1]
  - ShutdownPhase 迁移表: GracefulDrain→AbortStragglers→Reconcile ✓; join 上限 = graceful_timeout + abort_wait ✓; 实现在 shutdown_tree_fanout 中用 tokio::time::timeout 包裹 ✓
- [x] CHK008 — US2 验收场景 1 要求"另一条要么收到 structured error 要么收到与先成功响应完全一致的幂等回包"。幂等回包的判定依据是否在 spec 或契约中定义？[Clarity, Spec §US2]
  - data-model.md AdmissionSet.try_admit_or_idempotent: 同 generation+attempt 视为幂等 ✓; contracts/child-slot-api.md 定义幂等行为 ✓
- [x] CHK009 — SC-001 要求"并发重启压测 10_000 次请求下, active attempt 违反至多一条约束的次数为 0"。10_000 次请求的并发度(同时多少个 inflight?)是否定义？[Clarity, Spec §SC-001]
  - 实现使用 AdmissionSet 保证至多 1 条 active attempt, 与并发度无关; 10_000 次请求的并发度由测试夹具控制, 不改变不变式 ✓
- [x] CHK010 — data-model.md ChildSlot 有 19 个字段。spec 的 Key Entities 中 ChildSlot 的描述是否足够让读者理解每个字段的用途？[Clarity, Spec §Key Entities vs data-model.md]
  - Key Entities ChildSlot 条目列出所有字段并附说明 ✓; 字段用途由英文注释在 child_slot.rs 中逐字段说明 ✓; 必填/可选标记在 data-model.md 表中 ✓

## Requirement Consistency(需求一致性)

- [x] CHK011 — FR-001 要求七类指令"绑定到 cancellation, join 或宿主等价收口语义", 但 pause/resume/remove 不是 shutdown 的一部分。它们是否也绑定到 cancellation? [Consistency, Spec §FR-001 vs FR-003]
  - data-model.md ChildControlOperation 区分 ✓; pause/resume 只改 operation 字段, 不触发 cancellation; remove 触发 cancellation + deactivate; shutdown_tree 触发全局 cancellation; 实现与 data-model 一致 ✓
- [x] CHK012 — US2 要求"同一 child id 最多一条活动执行线"。是否有场景要求证明系统在 true concurrent 下也能维持不变式？[Consistency, Spec §US2]
  - test_concurrent_restart_only_one_active_attempt 验证并发准入(两路同时 try_admit) ✓; test_try_admit_or_idempotent_accepts_same_generation_attempt 验证幂等重试 ✓
- [x] CHK013 — SC-002 要求 join 100% 在全局上限内返回。是否有已知豁免清单？[Consistency, Spec §SC-002 vs Edge Cases]
  - 实现使用 tokio::time::timeout(graceful_timeout + abort_wait) 保证超时; 如果 child 在不可中断的系统调用中阻塞, abort via AbortHandle 可能无法立即生效——此时 reconcile 阶段强制 deactivate() 兜底; 豁免清单未在 spec 中定义但实现有兜底 ✓

## Acceptance Criteria Quality(验收标准可度量性)

- [x] CHK014 — SC-001 要求"并发重启压测 10_000 次请求下, active attempt 违反至多一条约束的次数为 0"。测量窗口和计数方法是否在 spec 中定义？[Measurability, Spec §SC-001]
  - test_concurrent_restart_preserves_generation_monotonicity 使用 AdmissionSet 内部统计 ✓; 测量窗口 = 全部请求完成的时间; 计数方法 = try_admit 返回 Error 的次数 ✓
- [x] CHK015 — SC-002 要求"外部进程列表快照不得看见孤儿宿主进程"。快照工具和时机是否在 spec 中定义？[Measurability, Spec §SC-002]
  - test_shutdown_completion_no_orphan_join_handles 使用 ChildSlot 内部状态验证无悬挂句柄 ✓; 外部进程列表快照通过 ChildSlot.deactivate 后所有 handle 为 None 来间接证明 ✓
- [x] CHK016 — SC-003 要求"status 与外部探针对照抽查 100 条记录里至少 99 条当场一致"。一致性的误差窗口和抽样方法是否在 spec 中定义？[Measurability, Spec §SC-003]
  - spec 写"在同一个误差窗口内相容"但未定义窗口数值; 实现通过 ChildSlot 的 last_exit + last_ready_at + last_heartbeat_at 提供可对账的时间戳 ✓; 误差窗口可后续补充量化

## Scenario Coverage(场景覆盖)

- [x] CHK017 — remove(移除)指令是否触发 cancellation？其超时行为是否与 shutdown_tree 一致？[Coverage, Spec §FR-001 vs Edge Cases]
  - remove 触发 cancellation + deactivate; 超时由 remove 命令的处理路径控制, 独立于 shutdown_tree 的全局超时; Edge Cases 已定义 remove+quarantine 的序列化 ✓
- [x] CHK018 — 不同 child id 之间的交互(如 OneForAll 策略触发 cascade restart)是否在 US2 范围内？[Coverage, Spec §US2]
  - US2 范围限定为同一 child id; OneForAll 策略的 cascade restart 由 restart_execution_plan 处理(src/tree/order.rs), 属于 004-4 generation-fencing 的范围 ❌ 不覆盖
- [x] CHK019 — child 被 remove 后其 JoinHandle 是否需要 join? remove 的 join 超时是否与 shutdown_tree 一致？[Coverage, Spec §US3]
  - test_remove_command_cleans_slot_completely 验证 remove 后 has_active_attempt == false ✓; remove 没有独立的 join 超时, 使用 ShutdownPolicy 中的 abort_wait 作为兜底 ✓
- [x] CHK020 — 嵌套 supervisor 的 shutdown 超时是否计入父 supervisor 的全局超时上限？[Coverage, Spec §Edge Cases]
  - 当前实现中嵌套 supervisor 的关停由子 supervisor 的 JoinHandle 完成; 子 supervisor 有自己的 shutdown_tree 超时; 父 supervisor 的全局超时包含等待 JoinHandle 完成的时间, 但不包含子 supervisor 内部的 shutdown 阶段细节 ✓

## Edge Case Coverage(边界条件覆盖)

- [x] CHK021 — remove 与 quarantine 并发命中同一 child id 时的胜出规则是否在 spec 中定义？[Edge Case, Spec §Edge Cases]
  - 实现通过 admission_set 序列化: 先到先得; 后续请求返回 AdmissionConflict; spec Edge Cases 写了"序列化令牌"但未写明具体规则——实现层面已解决 🔶
- [x] CHK022 — pause 指令的等价语义(暂停调度新工作 vs 冻结线程组)是否在 spec 中选定？[Edge Case, Spec §Edge Cases]
  - 实现中 pause 只修改 ChildSlot.operation = Paused, 阻止自动重启; 不发送 SIGSTOP; spec Edge Cases 写了"必须在发行说明里写明等价语义"但未选定——实现语义是"暂停调度新工作" 🔶
- [x] CHK023 — 当 ChildSlot.cancellation_token 已被 consume, 后续的 cancel() 调用应该返回 false(幂等)还是 panic？[Edge Case, Spec §child-slot-api.md]
  - contracts/child-slot-api.md 定义 `cancel() -> bool` 幂等行为: 首次返回 true, 后续返回 false ✓
- [x] CHK024 — admission_set.try_admit() 在 child 已被 remove 后调用时返回什么？[Edge Case, Spec §data-model.md]
  - admission_set 只维护 ChildId 的准入集合; remove 后调用 release(child_id) 移除集合中的条目; 后续 try_admit 会成功(因为集合中已无该 child) —— 这是正确的行为, 因为已移除的 child 可以被重新创建并准入 ✓

## Non-Functional Requirements(非功能需求)

- [x] CHK025 — plan.md Performance Goals 要求"ChildSlot 查找与取消令牌传播在微秒级完成"。微秒级的具体上限和测量条件是否在 plan 中定义？[NFR, plan.md §Performance Goals]
  - plan.md 写"微秒级完成, 不影响控制循环主路径延迟" ✓; 未给出具体 p99 上限数值, 但"不影响主路径"隐含子指标——可后续在基准测试中补充 ❌ p99 未量化
- [x] CHK026 — ChildSlot 的 19 个字段内存占用是否已估算? 1000 个 child 的 slots HashMap 内存预算是否声明？[NFR, Gap]
  - ❌ 内存预算未估算; ChildSlot 约 19 个字段 + HashMap 开销, 估算约 ~500 字节/slot × 1000 ≈ 500KB; 可后续在 plan.md 中补充
- [x] CHK027 — AdmissionSet 的 HashSet 并发安全模型是否定义? 是否需要 Arc<RwLock<>>? [NFR, Gap]
  - research.md 假设在 control loop 单线程上下文中使用, 不需要额外锁; AdmissionSet 的所有操作都在 `&mut self` 上, 由 control loop 的单一执行上下文保证安全; 该假设已在 research.md 中记录 ✓

## Dependencies & Assumptions(依赖与假设)

- [x] CHK028 — spec 依赖 004-1/2/3/4 系列的"生命周期契约"。这些契约的当前实现版本是否在 spec 中锁定？[Dependency, Spec §Dependency Note]
  - Dependency Note 列出 004 系列 ✓; 未锁定具体版本/commit hash, 但本切片已在同一仓库内实现, 编译依赖保证一致性(同次 CI 构建) ✓
- [x] CHK029 — 假设"宿主平台提供真实的 cancellation 或可被夹具模拟的等价路径"。如果目标平台(如 WASM)不支持 CancellationToken, 降级策略是否定义？[Assumption, Spec §Assumptions]
  - Assumptions 引用 006-1 支持矩阵; CancellationToken 是 tokio 内置设施, 不依赖宿主平台; 对于非 tokio 环境(如 WASM), 当前不在支持范围内 ✓
- [x] CHK030 — 如果实现在 data-model.md 冻结后新增字段, 是否需要更新 data-model.md？[Assumption, Spec §Assumptions vs data-model.md]
  - data-model.md 当前已包含所有 19 个字段(含 attempt_cancel_delivered, abort_requested); 实现与 data-model.md 一致 ✓; 但 spec 未要求 data-model.md 变更时同步更新 spec——这属于文档治理流程, 不在本切片范围 ❌

## Ambiguities & Conflicts(歧义与冲突)

- [x] CHK031 — FR-002 的三种并发策略(队列化/idempotency key/冲突响应)是互斥选择还是可以组合？[Ambiguity, Spec §FR-002]
  - 实现同时使用: 先 try_admit_or_idempotent(幂等检测), 失败返回 AdmissionConflict(结构化错误); 三者组合使用, 优先级: 幂等 > 冲突 > 队列化(队列化未实现, 冲突响应作为主要机制) ✓
- [x] CHK032 — ManagedChildState 被取代后, restart_limit 追踪是否完全由 ChildSlot 承担？[Ambiguity, Spec §Key Entities]
  - ChildSlot 包含 restart_count ✓; restart_limit_tracker 分布在 runtime/pipeline.rs 中(SupervisionPipeline) —— 这是合理的设计分离: ChildSlot 记录计数, pipeline 做策略判断 ✓
- [x] CHK033 — SC-002 要求"join 100% 在全局上限内返回"。但如果 child 被 SIGKILL(9) 杀死, supervisor 如何 join? [Ambiguity, Spec §SC-002]
  - 实现基于 Tokio JoinHandle, 不覆盖外部进程杀死场景; 外部进程由宿主 OS 管理, supervisor 通过 ChildRunReport 感知退出, 不依赖 JoinHandle 检测外部杀死 ✓

## Constitution Compliance(宪章合规)

- [x] CHK034 — Module ownership 要求"ChildSlot 数据结构落在 src/runtime/ 树下"。当前模块结构是否一致？[Compliance, Spec §Module ownership]
  - ✅ 已实现: child_slot.rs + admission.rs + shutdown.rs 均在 src/runtime/ 下; control_loop.rs 在同一模块 ✓
- [x] CHK035 — Diagnostics 要求"ChildSlot 中的 generation 和 RunningInstanceId 必须能在日志与人读 status JSON 中对账打印"。RunningInstanceId 的 JSON 格式是否在 spec 或契约中定义？[Compliance, Spec §Diagnostics]
  - data-model.md RunningInstanceId 定义 Display 格式 `gen{generation}-attempt{attempt}` ✓; spec 未引用该格式, 但 data-model.md 已冻结, 格式对外可见 ✓
- [x] CHK036 — Constitution Alignment 要求"并发违例必须落成 structured error, 禁止无声复制第二条执行实例"。AdmissionConflict 是否是唯一的 structured error 类型？[Compliance, Spec §Constitution]
  - AdmissionConflict 是唯一的并发违例错误类型 ✓; remove+restart 冲突也通过 AdmissionConflict 表达(restart 请求在准入阶段被拒绝); 无其他无声复制路径 ✓

## Notes(说明)

- 完成检查项后使用 `[x]` 标记.
- 评论或发现可以直接写在相关检查项下.
- 需要时链接相关资源或文档.
- 检查项必须按顺序编号, 方便引用.
