# ADR-007: 使用 tracing 和 metrics 作为可观测性基础

- **日期**: 2026-05-05
- **状态**: Accepted

## 背景

项目需要结构化生命周期回放、字段化诊断和可采集指标, 但不希望核心绑定到具体 exporter.

## 可选方案

- 方案 A: 只使用普通日志.
- 方案 B: 在核心中嵌入具体 metrics backend (如 Prometheus).
- 方案 C: 使用 `tracing` 和 `metrics` facade.

## 决策

选择方案 C.

## 理由

- `tracing` span 表达 child attempt, tracing event 表达状态迁移.
- `metrics` facade 允许 core 发送 counter/gauge/histogram, 不绑定单一 exporter.
- Prometheus exporter 可以放在示例或可选集成中.
- 普通日志不足以支持结构化生命周期回放和字段化诊断.

## 后果

- 正面: 核心不绑定具体 exporter, 可插拔.
- 正面: tracing span 提供原生结构化上下文.
- 负面: 需要额外学习 `tracing` 和 `metrics` API.

## 关联

- 关联 Spec: `specs/001-create-supervisor-core/contracts/public-api.md`
