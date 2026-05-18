# ADR-005: 分离 Current State 和 Lifecycle Event

- **日期**: 2026-05-05
- **状态**: Accepted

## 背景

当前状态和历史事件回答不同问题. 混用会导致职责不清.

## 可选方案

- 方案 A: 只提供 event stream. 消费者必须回放历史才能知道当前状态.
- 方案 B: 只提供 current state. 系统会丢失顺序、命令审计、重启决策和事件滞后信息.
- 方案 C: 分离 current state 和 event plane.

## 决策

选择方案 C: 分离 current state 和 event plane.

## 理由

- Current state 回答"现在是什么", event plane 回答"发生过什么".
- Watch-style state plane 只保存最新 `SupervisorState`.
- Event plane 保存有序生命周期事件, 供 subscriber/audit/replay/test 使用.
- 遵循 CQRS 模式: 查询 (current_state) 和事件 (event stream) 分离.

## 后果

- 正面: 职责清晰, state 和 event 各自优化.
- 正面: 事件流可回答审计问题, 状态快照可回答当前问题.
- 负面: 需要维护两个平面的一致性.
- 负面: 实现复杂度比单一模型高.

## 关联

- 关联 ADR: ADR-006 (命名约束)
- 关联 Spec: `specs/001-create-supervisor-core/`
