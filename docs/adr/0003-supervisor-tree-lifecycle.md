# ADR-003: 用 Supervisor Tree 表达生命周期治理

- **日期**: 2026-05-05
- **状态**: Accepted

## 背景

`OneForAll`, `RestForOne`, child-level quarantine, supervisor-level meltdown, 局部关闭和父级升级都需要树边界. 稳定路径如 `/root/market/binance_ws` 可以让日志、指标、事件和控制命令使用同一套位置词汇.

## 可选方案

- 方案 A: 只使用 flat registry (扁平注册表). 更简单.
- 方案 B: 使用 actor supervision tree. 表达能力更强.
- 方案 C: 使用 supervisor tree (树结构). 路径稳定, 可表达分组重启和父级升级.

## 决策

选择方案 C: 使用 supervisor tree.

## 理由

- Flat registry 无法表达分组重启顺序或父级升级.
- Actor supervision tree 违反不引入 actor framework 的约束.
- Supervisor tree 以稳定 `SupervisorPath` 为标识, 所有信号 (log/metric/event/command) 共用同一套位置词汇.

## 后果

- 正面: 分组重启、局部关闭、父级升级语义清晰.
- 正面: `SupervisorPath` 成为跨组件的稳定标识.
- 负面: 树结构比 flat registry 复杂.
- 负面: `tree` 模块需要维护节点关系和启动/关闭排序.

## 关联

- 关联 Spec: `specs/001-create-supervisor-core/`
