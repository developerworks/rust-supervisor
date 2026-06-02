# ADR-006: 禁止 \*Snapshot 和 \*View 代码命名

- **日期**: 2026-05-05
- **状态**: Accepted

## 背景

用户要求删除所有 `*Snapshot` 和 `*View` 命名方式. 状态查询表达的是当前状态, 不是复制某个历史对象, 也不是只读视图对象.

## 可选方案

- 方案 A: 继续使用 `ConfigSnapshot`, `SupervisorSnapshot`, `ChildStateView`.
- 方案 B: 只在文档中改名而保留代码别名.
- 方案 C: 全面禁止 `*Snapshot` 和 `*View`, 使用 `*State` / `current_state`.

## 决策

选择方案 C.

## 理由

- `*Snapshot` 暗示复制, 与 `current_state` 语义 (直接返回当前状态) 不符.
- `*View` 暗示只读投影, 混淆状态边界.
- 项目禁止兼容方法和旧接口别名, 因此不能保留别名.
- 正式命名: `ConfigState`, `SupervisorState`, `ChildState`, `current_state`.

## 后果

- 正面: 命名语义清晰, 表达当前状态而非历史副本.
- 正面: 消除命名混淆, 开发者不需要猜测是快照还是实时状态.
- 负面: 需要修改已有代码和文档.

## 关联

- 关联 ADR: ADR-005
- 关联 Spec: `specs/001-create-supervisor-core/contracts/public-api.md`
