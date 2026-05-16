# Contract(契约): Failure Policy Pipeline(失败策略流水线) and Typed Events(类型化事件)

本文件约束 **`005-1`** 交付时调用方或验收夹具能够依赖的稳定语义; **`Rust`** 类型实现必须与本契约字段同名或在本契约末尾 **`Alias mapping`(别名映射)** 表中登记.

## 1. **`policy pipeline`(策略流水线)** 固定顺序

调用方必须把下列六个阶段看成固定全序, **禁止跳过某一阶段直接进入下一阶段**, **`success`(成功)** 分支在后继阶段可以为 **`no-op`(空操作)**, **但每一阶段仍须在事件订阅端或等价导出文件里留下可按序号核对的诊断片段**:

1. **`classify exit`(分类退出)**
2. **`record failure window`(记录失败窗口)**
3. **`evaluate budget`(评估预算)**
4. **`decide action`(决定动作)**
5. **`emit typed event`(发出类型化事件)**
6. **`execute action`(执行动作)**

## 2. **`exit kind`(退出类别)** 最小必选集合

下列标签构成 **`tie-break`(平局判定)** 的不相交基底; **契约只允许增添细分标签**, **禁止删除**:

- **`success`(成功)**
- **`nonzero_exit`(非零退出)**
- **`panic`(崩溃)**
- **`timeout`(超时)**
- **`external_cancel`(外部取消)**
- **`manual_stop`(人工停止)**

## 3. **`protection restrictiveness ladder`(保护从严档位序)**

从左到松, 从右到严; **多层合并时取最靠右一档**:

| Canonical name(标准名称) | Meaning(含义) |
|---------------------------|---------------|
| **`restart_allowed`(允许按计划重启)** | 不因熔断附加闸门改变原计划重启 |
| **`restart_queued`(排队重启)** | 本轮意图仍为重启, 仅推迟或排队 |
| **`restart_denied`(拒绝重启)** | 策略写明的一段时间内禁止新的自动重启 |
| **`supervision_paused`(暂停监督)** | 自动化监督动作暂停直至解除条件 |
| **`escalated`(升级)** | 进入 **`escalation policy`(升级策略)** 写明步骤 |
| **`supervised_stop`(监督停止)** | 停止自动拉起直至人工明确要求再运行 |

## 4. **`evaluate budget`(评估预算)** 输入输出契约

**输入侧至少包含下列字段来源**:

- **`restart_execution_plan.restart_limit`**
- **`restart_execution_plan.escalation_policy`**
- 配置启用时 **`MeltdownTracker`(熔断跟踪器)** 在 **`child`**, **`group`**, **`supervisor`** 三套 **`scope`(作用域)** 下的计数取值

**输出侧须能在 **`TypedSupervisionEvent`(类型化监督事件)** 或等价导出通道上逐项读出下列字段**:

- **`effective_protective_action`**; **`cold start budget`(冷启动预算)** 与 **`hot loop detection`(热循环检测)** 在同一轮同时触发时须附带 **`tie-break`(平局判定)** 取舍说明
- **`scopes_triggered`**; **只要有任一 **`scope`** 计数越过阈值**, **该列表不得为空**
- **`lead_scope`**; **`local verdict`(局部判定)** 从严程度并列时按 **`child` → `group` → `supervisor`** 次序取值

## 5. **`BackoffPolicy`(退避策略)** 可重复性契约

**生产配置须开启** **`full jitter`(全抖动)**, **`decorrelated jitter`(去相关抖动)**, **并行重启上限**, **`cold start budget`(冷启动预算)**, **`hot loop detection`(热循环检测)**.

**验收夹具须能固定 **`RNG seed`(随机种子)** 与 **`clock`(时钟)**, **使同一输入连续两次运行在同一 **`restart throttle plan`(重启节流计划)** 的诊断输出里得到相同的 **`next_wait`** 序列**.

### 5.3 Dispersion metric for SC-004(SC-004 分散程度度量)

**`spec.md`** 成功标准 **SC-004** 要求启用 **`full jitter`(全抖动)** 或 **`decorrelated jitter`(去相关抖动)** 时,等待间隔的分散程度比固定 **`jitter`(抖动)** 基准高出至少三成.验收夹具必须使用下列公式量化分散程度:

**Coefficient of Variation (CV,变异系数)** = `std_deviation(next_wait_sequence)` / `mean(next_wait_sequence)`

其中:
- **`std_deviation`**: 等待时长序列的标准差
- **`mean`**: 等待时长序列的算术平均值
- **`next_wait_sequence`**: 同一批因相近原因触发的 N 次重启的等待时长数组,N ≥ 10

**验收条件**: `CV_jitter_strategy` / `CV_fixed_baseline` ≥ 1.3


### 5.1 Default thresholds(默认阈值)

下列默认值用于验收测试在未覆盖自定义配置时的稳定触发; 生产环境可通过上层配置覆盖:

| Threshold name(阈值名称) | Default value(默认值) | Meaning(含义) |
|---------------------------|------------------------|---------------|
| **`cold_start_window_secs`** | `60` | 监督实例启动后视为冷启动的时间窗, 单位秒 |
| **`cold_start_max_restarts`** | `5` | 冷启动时间窗内允许的最大自动重启次数配额 |
| **`hot_loop_window_secs`** | `10` | 热循环检测的滑动时间窗宽度, 单位秒 |
| **`hot_loop_min_restarts`** | `3` | 热循环检测窗口内触发保护的最小重启次数 |

### 5.2 **`throttle_gate_owner`** serialization format(序列化格式)

**`TypedSupervisionEvent`** 中的 **`throttle_gate_owner`** 字段必须使用下列字符串格式之一:

| Value(取值) | Meaning(含义) |
|-------------|---------------|
| **`"supervisor_global"`** | 闸门作用于当前 **`supervisor`(监督器)** 实例全局, 计数不与进程内其他监督器实例共享 |
| **`"group:{group_id}"`** | 闸门作用于指定分组, **`{group_id}`** 为分组标识符的实际值, 例如 **`"group:worker-pool-a"`** |


## 6. **`Alias mapping`(别名映射)**

| Contract term(契约术语) | Current code anchor(当前代码锚点) | Migration note(迁移说明) |
|------------------------|-------------------------------------|---------------------------|
| **`restart_execution_plan`** | `StrategyExecutionPlan` in `src/tree/order.rs` | 字段继续保持 |
| **`MeltdownTracker`** | `MeltdownTracker` in `src/policy/meltdown.rs` | 增补 **`group`** **`scope`(作用域)** |
| Lifecycle payloads | `src/event/payload.rs` | 增补 **`005-1`** 字段 |
