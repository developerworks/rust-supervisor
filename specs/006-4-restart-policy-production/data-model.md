# Data Model(数据模型): 生产级重启策略

**Feature(功能)**: `006-4-restart-policy-production`

## Entities(实体)

### RestartBudgetConfig(重启预算配置)

| 字段                  | 类型     | 说明                         |
| --------------------- | -------- | ---------------------------- |
| window                | Duration | 故障计数滑动窗口             |
| max_burst             | u32      | 窗口内允许的最大故障突发次数 |
| recovery_rate_per_sec | f64      | 每秒归还的令牌速率           |
| max_tokens            | u32      | 令牌桶容量上限               |

字段约束: `window > 0s`, `max_burst >= 1`, `0.0 < recovery_rate_per_sec <= 1000.0`, `max_tokens >= 1`. 非法值在配置加载阶段以结构化错误拒绝.

### RestartBudgetSnapshot(重启预算快照)

| 字段         | 类型 | 说明                      |
| ------------ | ---- | ------------------------- |
| consumed     | u32  | 当前窗口内已消耗重启次数  |
| remaining    | u32  | 当前窗口内剩余重启配额    |
| tokens       | f64  | 当前令牌桶令牌数          |
| window_start | u128 | 滑动窗口起始时刻(unix ns) |
| sample_time  | u128 | 快照采样时刻(unix ns)     |

### RestartBudgetTracker(重启预算跟踪器)

| 字段                   | 类型                | 说明             |
| ---------------------- | ------------------- | ---------------- |
| config                 | RestartBudgetConfig | 预算配置         |
| failures               | VecDeque\<u128\>    | 故障时间戳队列   |
| tokens                 | f64                 | 当前令牌计数     |
| last_update_unix_nanos | u128                | 最近一次更新时刻 |

方法:

- `try_consume(&mut self, now_unix_nanos: u128) -> BudgetVerdict`
  - 取当前窗口, 驱逐过期故障, 归还令牌
  - 若 tokens >= 1.0, 扣减 1 并返回 `Granted`
  - 否则返回 `Exhausted { retry_after_ns: u128 }`
  - 三步操作(驱逐->归还->检查)在单次 `&mut self` 调用内原子完成, 调用方无需额外加锁
  - `recovery_rate_per_sec` 为 `f64` 浮点型, 连续运行数月累积误差 <=1ms 量级, 对重启调度精度影响可忽略(令牌归还粒度本身为秒级以上)

### BudgetVerdict(预算裁决)

```rust
pub enum BudgetVerdict {
    Granted,
    Exhausted { retry_after_ns: u128 },
}
```

### FairnessProbe(公平性探针)

| 字段                     | 类型                    | 说明                                          |
| ------------------------ | ----------------------- | --------------------------------------------- |
| scheduling_opportunities | u64                     | 累计调度机会计数                              |
| per_child_ops            | HashMap\<ChildId, u64\> | 每个 child 的调度次数                         |
| last_probe_unix_nanos    | u128                    | 最近一次探测时刻                              |
| probe_interval_ns        | u128                    | 探测间隔(默认 10s)                            |
| min_ops_per_window       | u64                     | 窗口内每个就绪 child 最少应获调度次数(默认 1) |

方法:

- `record_opportunity(&mut self, child_id: &ChildId)` — 记录调度机会
- `check(&self, now_unix_nanos: u128, all_child_ids: &[ChildId]) -> Option<StarvationAlert>`

### StarvationAlert(饥饿告警)

```rust
pub struct StarvationAlert {
    pub starved_child_id: ChildId,
    pub skip_count: u64,
    pub probe_start_unix_nanos: u128,
    pub probe_end_unix_nanos: u128,
}
```

### GroupDependencyEdge(分组依赖边)

| 字段        | 类型              | 说明           |
| ----------- | ----------------- | -------------- |
| from_group  | String            | 依赖方分组名   |
| to_group    | String            | 被依赖方分组名 |
| propagation | PropagationPolicy | 故障传播策略   |

### PropagationPolicy(传播策略枚举)

```rust
pub enum PropagationPolicy {
    None,           // 不传播故障 — 分组完全隔离
    EscalateOnly,   // 仅升级到父监督器 — 不影响当前组内 child 调度
    Full,           // 完全传播 — 当前组内所有 child 标记为不可重启, 当前组进入熔断状态. 传播方向: 故障从 to_group 单向传播到 from_group, 不反向传播
}
```

### GroupIsolationPolicy(分组隔离策略)

| 字段         | 类型                       | 说明             |
| ------------ | -------------------------- | ---------------- |
| dependencies | Vec\<GroupDependencyEdge\> | 声明的跨组依赖边 |

方法:

- `affected_by(&self, my_group: &str, failed_group: &str) -> bool`

分组依赖边构成有向无环图(DAG). 配置加载时检测环形依赖, 若存在则拒绝加载并返回结构化错误(指出环路上的 group 名序列).

### BackoffJitter(退避抖动) 参数

| 字段       | 类型 | 说明                            |
| ---------- | ---- | ------------------------------- |
| jitter_min | f64  | 抖动系数下限(默认 0.5, 即 -50%) |
| jitter_max | f64  | 抖动系数上限(默认 1.5, 即 +50%) |

实际延迟 = base_delay × random(jitter_min, jitter_max).

### EscalationBifurcated 事件 Metrics 标签(诊断键)

`EscalationBifurcated` 事件在 typed event 和 metrics 双通道中至少携带以下互不混淆的诊断键:

| 诊断键 (metrics label / event field) | 类型                    | 说明                                                       |
| ------------------------------------ | ----------------------- | ---------------------------------------------------------- |
| `severity_class`                     | `SeverityClass`         | Critical / Optional / Standard                             |
| `escalation_path`                    | `String`                | `"upgrade"`(升级) 或 `"noise_reduction"`(降噪)             |
| `budget_verdict`                     | `Option<BudgetVerdict>` | 若参与预算评估则携带 Granted/Exhausted, 否则 None          |
| `fuse_active`                        | `bool`                  | 该 child 所在 group 是否处于熔断状态                       |
| `tie_break_reason`                   | `Option<String>`        | 若触发平局裁决则携带原因, 否则 None                        |
| `correlation_id`                     | `CorrelationId`         | 关联标识(与同链路 BudgetExhausted/GroupFuseTriggered 共享) |

以上 6 个键满足 spec.md US3 验收场景中"至少多出 3 个互不混淆的诊断键"的要求.

### SeverityClass(严重程度分类)

```rust
pub enum SeverityClass {
    Critical,   // 关键: 失败必须升级
    Optional,   // 可选: 失败降噪处理
    Standard,   // 默认: 标准策略路径
}
```

### EffectivePolicy 扩展

在现有 `EffectivePolicy` 中新增:

- `severity: SeverityClass` — 严重程度分类
- `group_name: Option<String>` — 所属分组名(用于分组隔离)

## State Transitions(状态转换)

### RestartBudgetTracker 状态机

```
Idle ──try_consume(tokens≥1)──▶ Granted
Idle ──try_consume(tokens<1)──▶ Exhausted
Granted ──(tokens归还)──▶ Idle
Exhausted ──(tokens归还,retry_after到期)──▶ Idle
```

### MeltdownTracker 分组维度

```
Normal ──child_fuse──▶ ChildQuarantined
Normal ──group_fuse──▶ GroupIsolated
GroupIsolated ──affected_by(groupA,groupB)─▶ [传播判定]
GroupIsolated ──reset_after到期──▶ Normal
```

熔断恢复规则: `reset_after` 倒计时期间若出现零星故障, 不重置倒计时(继续等待原倒计时结束); 只有当故障密度再次超过熔断阈值时, 才重新触发熔断并重置计时.
child 未归属任何 group(`group_name: None`) 时: 该 child 的故障仅影响自身(child 级熔断), 不触发任何 group 级熔断, 也不受其他 group 熔断的连带影响.

## Relationships(关系)

```
SupervisorSpec
  └─ groups: Vec<GroupConfig>
       ├─ name: String
       ├─ children: Vec<ChildId>
       └─ budget: RestartBudgetConfig
  └─ group_dependencies: Vec<GroupDependencyEdge>
  └─ severity_defaults: HashMap<WorkRole, SeverityClass>

ChildSpec
  ├─ role: WorkRole
  ├─ severity: Option<SeverityClass>     (覆盖角色默认值)
  └─ group: Option<String>               (所属分组)
```
