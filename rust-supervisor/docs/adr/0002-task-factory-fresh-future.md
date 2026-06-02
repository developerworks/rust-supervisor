# ADR-002: 使用 TaskFactory 而非克隆任务实例

- **日期**: 2026-05-05
- **状态**: Accepted

## 背景

重启语义必须诚实: 每次重启都应构造新的异步任务, 而不是克隆旧的失败实例. 需要跨重启保留的状态必须显式表达.

## 可选方案

- 方案 A: 每次重启克隆旧任务实例. 实现更简单.
- 方案 B: 每次重启通过 `TaskFactory` 构造 fresh future.
- 方案 C: 把可变任务实例存入 supervisor, 使用锁保护.

## 决策

选择方案 B: 每次重启构造 fresh future.

## 理由

- 克隆任务实例会隐藏状态重置行为, 让开发者误以为状态在重启后保持.
- 把可变任务实例存入 supervisor 会增加锁和所有权复杂度, 混入运行时治理.
- `TaskFactory::build` 接收 `TaskContext`, 每次返回新 `BoxTaskFuture`, 语义清晰.

## 后果

- 正面: 重启语义诚实, 不隐藏状态丢失.
- 正面: `TaskFactory` 可测试.
- 负面: 需要跨重启保持的状态必须通过 `Arc`、存储或调用者拥有的 state repository 显式管理.

## 关联

- 关联 ADR: ADR-001
- 关联 Spec: `specs/001-create-supervisor-core/contracts/public-api.md`
