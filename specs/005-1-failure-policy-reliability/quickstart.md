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

## 2. 代码阅读顺序

1. `src/tree/order.rs` 里的 **`restart_execution_plan`**, 弄清 **`restart_limit`** 与 **`escalation_policy`** 从哪里来.
2. `src/runtime/control_loop.rs` 里 **`refresh_restart_limit_for_child`**, **`execute_restart_decision`**, **`restart_strategy_scope`**, 对照 **`spec.md`** **`FR-001`**.
3. `src/policy/meltdown.rs` 对照 **`FR-002`** 三层 **`scope`** 缺口.
4. `src/event/payload.rs` 对照 **`contracts/pipeline-and-events.md`**.

## 3. 与 `005-2` 合并验收时的额外一步

读完 `specs/005-2-work-role-defaults/spec.md` 的 Dependency Note(依赖说明), 确认 **`RoleDefaultPolicyPack`** 只在 **`evaluate budget`** 之后改变 **`decide action`** 输入, **不得短路六阶段顺序**.

## 4. 当前代码与结构化验收之间的差距

运行时仅靠 **`broadcast::Sender<String>`** 发出的 **`restart_plan:`** 前缀文本 **不能** 单独充当 **`TypedSupervisionEvent`(类型化监督事件)** 的结构化验收证据; **验收须以 **`serde`(序列化)** 载荷字段为准**.
