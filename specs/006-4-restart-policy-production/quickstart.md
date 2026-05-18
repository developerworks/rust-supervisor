# Quickstart(快速开始): 生产级重启策略

**Feature(功能)**: `006-4-restart-policy-production`

## 阅读顺序

1. `spec.md` — 功能规格与验收场景
2. `plan.md` — 实现计划与模块结构
3. `research.md` — 技术决策与替代方案
4. `data-model.md` — 实体定义与状态转换
5. `contracts/restart-budget-api.md` — 预算跟踪器 API 契约
6. `contracts/group-isolation-api.md` — 分组隔离 API 契约
7. `tasks.md` — 可执行任务列表(由 `/speckit-tasks` 生成)

## 源码阅读入口

| 文件                          | 说明                                                           |
| ----------------------------- | -------------------------------------------------------------- |
| `src/policy/budget.rs`        | RestartBudgetTracker 实现                                      |
| `src/policy/group.rs`         | GroupIsolationPolicy 实现                                      |
| `src/policy/role_defaults.rs` | SeverityClass 枚举, EffectivePolicy 扩展                       |
| `src/policy/meltdown.rs`      | MeltdownTracker 分组隔离增强                                   |
| `src/policy/decision.rs`      | PolicyEngine 预算集成                                          |
| `src/observe/fairness.rs`     | FairnessProbe 实现                                             |
| `src/event/payload.rs`        | BudgetExhausted, GroupFuseTriggered, EscalationBifurcated 事件 |
| `src/runtime/pipeline.rs`     | SupervisionPipeline evaluate_budget 阶段增强                   |
| `src/runtime/control_loop.rs` | Budget + Fairness 探测接入                                     |

## 编译与测试

```bash
# 在仓库根目录执行
cargo check                          # 编译检查
cargo test                           # 全量测试
cargo test --test policy_budget_waveform_test    # 预算波形测试
cargo test --test policy_group_isolation_test    # 分组隔离测试
cargo test --test policy_critical_optional_test  # 分叉观测测试
```

## 关键概念

- **RestartBudget(重启预算)**: 混合滑动窗口 + 令牌桶, 限制有效重启速率
- **FairnessProbe(公平性探针)**: 控制循环主路径轻量探针, 检测调度饥饿
- **GroupStrategy(分组策略)**: 基于声明的依赖边判定故障传播范围
- **SeverityClass(严重程度分类)**: Critical(关键) 与 Optional(可选) 分叉路径标签
