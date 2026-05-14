# Drift Supersession Map(偏差 superseded 映射)

**Authority(依据)**: `proposals.json` 中 **P007** 人工决策 **Option C** — 不修改 `specs/001-create-supervisor-core/spec.md` 正文, 在偏差工作流中用本文件声明「已由 `004` 系列分规格承接」的条款, 避免与 `004-1`..`004-4` 重复开洞.

**Usage(用法)**: 运行 `speckit.sync.analyze` 或人工 triage(分流) 时, 若漂移条目命中左列 **001** 需求编号, 且右列 **004** 规格已存在独立任务或验收, 则可在 `drift-report` 或台账中把该条目标记为 **SUPERSEDED**, 实现优先级以右列规格为准.

| 001 requirement(001 需求编号) | Superseded by(主要由谁承接) | Notes(说明) |
|--------------------------------|-----------------------------|-------------|
| `FR-020`, `FR-045` | `004-2-real-shutdown-pipeline` | 四阶段关闭执行与对账摘要 |
| `FR-049` 中与控制循环失败 typed path 相关部分 | `004-1-runtime-lifecycle-guard` | 看门狗与观测流水线接入 |
| `FR-044`, `SC-015` 与阻塞任务关闭边界相关部分 | `004-3-child-runtime-state-control` | 运行状态记录持有真实尝试与停止语义 |
| `FR-004` 与任务上下文事件 sink 缺口相关部分 | `004-2-real-shutdown-pipeline`, `004-1-runtime-lifecycle-guard` | 关闭与生命周期事件分路径补齐 |
| `FR-005` 与多层监督树统一展开相关部分 | 仍主要归 `001`, 可与 `004-3` 运行状态记录树遍历协同 | 未单独拆规格前不标 superseded |
| `FR-010`, `FR-011` | 仍主要归 `001` | 退出与失败分类模型核心仍在 001 |
| `FR-063` 及 dashboard 命名 | `003-supervisor-dashboard` | 与 001 正交, 不写入本表 superseded |

**Non-goals(不做的事)**: 本文件**不**修改任何 `specs/**/spec.md` 法律效力, **不**自动关闭 `001` 中的条目, 仅作为同步工作流与评审台账的辅助索引.
