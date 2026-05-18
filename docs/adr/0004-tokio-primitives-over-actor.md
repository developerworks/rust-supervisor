# ADR-004: 直接使用 Tokio 原语而非 Actor 框架

- **日期**: 2026-05-05
- **状态**: Accepted

## 背景

项目面向 Tokio 应用, 需要结构化并发和父子关闭传播. `JoinSet` 和 `CancellationToken` 是 Tokio 内置原语.

## 可选方案

- 方案 A: 使用 actor framework (如 ractor). 自带 supervision tree.
- 方案 B: 直接使用 Tokio 原语: `JoinSet`, `CancellationToken`, `mpsc`, `broadcast`.

## 决策

选择方案 B: 直接使用 Tokio 原语.

## 理由

- `JoinSet` 提供结构化并发: drop 时 abort 所有任务, `abort_all` 后可通过 `join_next` 排空.
- `CancellationToken` 提供父子关闭传播: child token 取消不会取消 parent token.
- `mpsc` 通道承载控制命令, `broadcast` 通道分发事件.
- Actor framework 的 supervision tree 与本项目需要的语义不同.

## 后果

- 正面: 无额外框架假设, 依赖简洁.
- 正面: Tokio 原语的语义与项目需求精确匹配.
- 负面: 每个 child 只保存一个 `JoinHandle` 对作用域关闭和无孤儿任务保证较弱, 需 `JoinSet` 辅助.

## 关联

- 关联 ADR: ADR-001, ADR-003
- 关联 Spec: `specs/001-create-supervisor-core/contracts/public-api.md`
