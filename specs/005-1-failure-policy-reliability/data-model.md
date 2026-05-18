# Data Model(数据模型): `005-1` 失败流水线与保护档位

下列实体命名须与 `spec.md` Key Entities(关键实体) 以及 **`FR`** 用词一致; Rust(编程语言) 类型名可在实现阶段微调, 但字段语义不得漂移.

## 1. **`PolicyPipelineStage`(策略流水线阶段)**

| Field(字段) | Meaning(含义) | Audit anchor(可对账锚点) |
|-------------|---------------|-------------------------|
| `stage_id` | 六阶段之一的稳定枚举值 | 事件载荷里的 **`pipeline_stage`** |
| `inputs_digest` | 上一阶段写入订阅或导出通道的关键字段摘要或等价结构化拷贝引用 | 同一 **`CorrelationId`(关联标识)** 下复盘时可逐项对上 |
| `outputs_digest` | 本阶段写入 **`TypedSupervisionEvent`(类型化监督事件)** 或等价导出渠道的字段集合 | 与下一阶段 **`inputs_digest`** 衔接时可逐项核对 |

## 2. **`FailureWindow`(失败窗口)**

| Field(字段) | Meaning(含义) | Validation(校验) |
|-------------|---------------|------------------|
| `window_kind` | 按时间滑动或按次数滑动 | 契约写明其一 |
| `window_span` | 窗口宽度 | 大于零 |
| `failure_samples` | 落在窗口内的失败样本队列或计数 | 与 **`MeltdownTracker`(熔断跟踪器)** 计数同源 |

## 3. **`MeltdownScopeState`(熔断作用域状态)**

| Field(字段) | Meaning(含义) | Relationships(关系) |
|-------------|---------------|---------------------|
| `scope_key` | **`child`**, **`group`**, **`supervisor`** 三元之一加对应标识 | 一对一绑定受监督拓扑节点 |
| `quota_counters` | 窗口内计数与相对阈值的累计进度 | 供 **`evaluate budget`(评估预算)** 读取并与阈值比较 |
| `local_verdict` | 本轮 **`evaluate budget`** 得到的 **`local verdict`(局部判定)** 档位 | 映射到 **`protection restrictiveness ladder`(保护从严档位序)** |

## 4. **`RestartThrottlePlan`(重启节流计划)**

| Field(字段) | Meaning(含义) | Notes(备注) |
|-------------|---------------|---------------|
| `max_parallel_restarts` | 实例全局闸门或分组闸门上界 | 与 **`Assumptions`** 全局闸门默认一致 |
| `cold_start_segment` | **`cold start budget`(冷启动预算)** 生效区间 | 与耗尽依据写入同一事件 |
| `hot_loop_segment` | **`hot loop detection`(热循环检测)** 生效区间 | 与 **`restart limit`** 超限档位在字段上可区分 |
| `next_wait` | **`BackoffPolicy`(退避策略)** 给出的下一次等待时长 | 验收夹具固定 RNG seed 后应能复盘同一取值序列 |

## 5. **`TypedSupervisionEvent`(类型化监督事件)** 增量字段 (相对现状)

下列字段为 **`005-1`** 相对 `src/event/payload.rs` 现状的最小增补集合; 精确 **`serde`** 名在 **`contracts/`** 固化.

| Field(字段) | Meaning(含义) |
|-------------|---------------|
| `scopes_triggered` | 本轮达到或越过阈值的 **`scope`** 列表 |
| `lead_scope` | **`effective meltdown verdict`(有效熔断判定)** 归因到哪一层 **`scope`** |
| `effective_protective_action` | 落在 **`protection restrictiveness ladder`** 上的生效档位 |
| `cold_start_reason` | **`cold start budget`** 耗尽或触发缘由的可读枚举 |
| `hot_loop_reason` | **`hot loop detection`** 触发缘由的可读枚举 |
| `throttle_gate_owner` | 闸门归属 **`supervisor` 实例全局** 或具体 **`group`** |

## 6. 状态迁移 (摘录)

1. **`exit observed`(观察到退出)** → 强制进入 **`classify exit`** → **`exit kind`(退出类别)** 落在最小集合之一.
2. **`evaluate budget`** → 读取 **`restart_execution_plan`** 里的 **`restart limit`**, **`escalation policy`** → 汇总 **`MeltdownTracker`** 三层 **`local verdict`** → 产出 **`effective meltdown verdict`**.
3. **`decide action`** → 产出是否重启, 排队, 拒绝, 暂停, 升级或监督停止的结论字段快照.
4. **`emit typed event`** → **禁止跳过** → **`execute action`** → **禁止与前一阶段字段正面冲突**.
