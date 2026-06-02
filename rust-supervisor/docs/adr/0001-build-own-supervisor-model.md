# ADR-001: 构建项目自有 Supervisor 模型, 不包装现成 crate

- **日期**: 2026-05-05
- **状态**: Accepted

## 背景

项目需要精确的领域模型: `ChildSpec`, `SupervisorTree`, `TaskFactory`, typed exit reason, child/supervisor fuse, control-plane audit, current state 和 `When/Where/What` 事件. 现有 crate 分别覆盖部分能力, 但没有一个能在不复制第三方 API 形状或引入框架假设的情况下满足完整契约.

## 可选方案

- 方案 A: 包装 `task-supervisor` crate. 提供 runtime control, status query, health interval, restart limit, backoff.
- 方案 B: 包装 `ractor-supervisor`. 提供 `OneForOne/OneForAll/RestForOne`, `Permanent/Transient/Temporary`, meltdown window.
- 方案 C: 包装 `taskvisor`. 提供 event 和 registry 架构.
- 方案 D: 包装 `tokio-graceful-shutdown`. 提供 shutdown protocol.
- 方案 E: 构建项目自有 Supervisor 模型.

## 决策

选择方案 E: 构建项目自有 Supervisor 模型.

## 理由

- `task-supervisor` 使用 clone-on-restart 任务模型, 任务内部可变状态在重启时容易丢失语义.
- `ractor-supervisor` 基于 actor framework, 项目明确排除 actor 框架.
- `taskvisor` 的 API 不符合项目需要的 tree, audit, typed error 和双层 fuse 模型.
- `tokio-graceful-shutdown` 缺少 supervisor tree 专用控制和状态.
- 自有模型可以精确满足 `ChildSpec`/`SupervisorTree`/`TaskFactory`/typed exit/audit/current state 等全部契约.

## 后果

- 正面: 完全控制 API 表面, 不受第三方 crate API 变化影响.
- 正面: 精确满足项目的认知复杂度、模块边界和命名约束.
- 负面: 需要更多开发工作量.
- 负面: 没有上游社区修补 bug.

## 关联

- 关联 ADR: ADR-004 (Tokio 原语)
- 关联 Spec: `specs/001-create-supervisor-core/`
