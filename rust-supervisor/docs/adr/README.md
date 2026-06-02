# 技术决策记录 (Architecture Decision Records)

> 最后更新: 2026-05-18

本章收录 `rust-tokio-supervisor` 项目的重要架构和技术决策。每条 ADR 记录一个决策的背景、可选方案、选型理由和后果。

## 决策列表

| 编号 | 标题                                                                                     | 日期       | 状态     |
| ---- | ---------------------------------------------------------------------------------------- | ---------- | -------- |
| 001  | [构建项目自有 Supervisor 模型, 不包装现成 crate](0001-build-own-supervisor-model.md)     | 2026-05-05 | Accepted |
| 002  | [使用 TaskFactory 而非克隆任务实例](0002-task-factory-fresh-future.md)                   | 2026-05-05 | Accepted |
| 003  | [用 Supervisor Tree 表达生命周期治理](0003-supervisor-tree-lifecycle.md)                 | 2026-05-05 | Accepted |
| 004  | [直接使用 Tokio 原语而非 Actor 框架](0004-tokio-primitives-over-actor.md)                | 2026-05-05 | Accepted |
| 005  | [分离 Current State 和 Lifecycle Event](0005-separate-state-and-event.md)                | 2026-05-05 | Accepted |
| 006  | [禁止 \*Snapshot 和 \*View 代码命名](0006-ban-snapshot-view-naming.md)                   | 2026-05-05 | Accepted |
| 007  | [使用 tracing 和 metrics 作为可观测性基础](0007-tracing-metrics-observability.md)        | 2026-05-05 | Accepted |
| 008  | [使用 Typed Error 和明确 Policy Decision](0008-typed-error-policy-decision.md)           | 2026-05-05 | Accepted |
| 009  | [三目录架构: 核心库 + Relay + UI](0009-three-directory-architecture.md)                  | 2026-05-17 | Accepted |
| 010  | [IPC 仅限 Unix 域套接字, 平台编译隔离](0010-unix-only-ipc-cfg.md)                        | 2026-05-17 | Accepted |
| 011  | [策略评估管线固定顺序: budget -> meltdown -> backoff](0011-policy-pipeline-order.md)     | 2026-05-18 | Accepted |
| 012  | [配置集中化: rust-config-tree 作为唯一入口](0012-centralized-config-rust-config-tree.md) | 2026-05-05 | Accepted |

## 模板

```markdown
# ADR-NNN: 决策标题

- **日期**: YYYY-MM-DD
- **状态**: Accepted | Proposed | Deprecated | Superseded

## 背景

描述需要做决策的问题和上下文.

## 可选方案

- 方案 A: 描述.
- 方案 B: 描述.

## 决策

选择方案 X.

## 理由

说明为什么选择该方案, 包括关键权衡.

## 后果

列出采纳该决策后的正面和负面后果.

## 关联

- 关联 ADR: ADR-NNN
- 关联 Spec: specs/XXX
```
