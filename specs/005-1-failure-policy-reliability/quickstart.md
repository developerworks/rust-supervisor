# Quickstart(快速开始): 在本仓库验收 `005-1`

本节写给要在本机核对 **`spec.md`** 行为的审查者与实现者; **默认命令在仓库根目录执行**.

## 1. 编译与测试入口

```bash
cargo test
```

行为切片落地后, **最小回归集合** 至少包含:

- `src/tests/supervisor_*` 中与 **`restart limit`**, **`policy`**, **`shutdown`** 相关的用例 (具体文件名随 **`tasks.md`** 拆分),
- `src/policy/tests/meltdown_test.rs`,
- 新增的 **`pipeline ordering`** 与 **`typed event`** 字段断言测试.

### US3 新增验收测试:
- `tests/supervisor_backoff_jitter_distribution.rs` — 验证全抖动和去相关抖动的分散程度 (5 个测试).
- `tests/supervisor_concurrent_restart_throttle.rs` — 验证并发闸门原子性和保护档位 (6 个测试,含 10 并发样本原子性测试).
- `tests/supervisor_cold_start_and_hot_loop.rs` — 验证冷启动预算和热循环检测 (9 个测试).
- `tests/supervisor_pipeline_full_integration.rs` — 端到端集成测试,覆盖交叉场景 (11 个测试).

## 2. 代码阅读顺序

### Phase 2-3 (基础与流水线):
1. `src/tree/order.rs` 里的 **`restart_execution_plan`**, 弄清 **`restart_limit`** 与 **`escalation_policy`** 从哪里来.
2. `src/policy/failure_window.rs` — 失败窗口滑动累计逻辑.
3. `src/runtime/pipeline.rs` — 六阶段流水线编排 (`classify exit` → `record failure window` → `evaluate budget` → `decide action` → `emit typed event` → `execute action`).
4. `src/policy/meltdown.rs` 对照 **`FR-002`** 三层 **`scope`** 缺口与 `merge_meltdown_verdicts` 平局判定.
5. `src/event/payload.rs` 对照 **`contracts/pipeline-and-events.md`**.

### Phase 4-5 (退避策略与并发闸门):
6. `src/policy/backoff.rs` — 全抖动 (`calculate_full_jitter`)、去相关抖动 (`calculate_decorrelated_jitter`)、冷启动预算 (`ColdStartBudget`)、热循环检测 (`HotLoopDetector`).
7. `src/runtime/concurrent_gate.rs` — 实例全局闸门 (`SupervisorInstanceGate`)、分组级闸门 (`GroupLevelGate`)、组合闸门 (`CombinedThrottleGate`).
8. `src/test_support/factory.rs` — 测试工厂函数 (`deterministic_backoff_policy`, `full_jitter_backoff_policy`, `decorrelated_jitter_backoff_policy`).
9. `src/test_support/test_time.rs` — 可控时钟 (`advance_test_clock`, `with_auto_clock_drive`).

## 3. 与 `005-2` 合并验收时的额外一步

读完 `specs/005-2-work-role-defaults/spec.md` 的 Dependency Note(依赖说明), 确认 **`RoleDefaultPolicyPack`** 只在 **`evaluate budget`** 之后改变 **`decide action`** 输入, **不得短路六阶段顺序**.

## 4. 当前代码与结构化验收之间的差距

运行时仅靠 **`broadcast::Sender<String>`** 发出的 **`restart_plan:`** 前缀文本 **不能** 单独充当 **`TypedSupervisionEvent`(类型化监督事件)** 的结构化验收证据; **验收须以 **`serde`(序列化)** 载荷字段为准**.
