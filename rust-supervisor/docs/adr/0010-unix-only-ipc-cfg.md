# ADR-010: IPC 仅限 Unix 域套接字, 平台编译隔离

- **日期**: 2026-05-17
- **状态**: Accepted

## 背景

Dashboard IPC 依赖 Unix 域套接字, 在 Windows 等非 Unix 平台不可用. 需要明确平台边界.

## 可选方案

- 方案 A: 使用 Cargo feature gate 控制 dashboard/IPC 模块启用.
- 方案 B: 使用 `#[cfg(unix)]` 编译期排除. 无需 feature, 编译器保证.
- 方案 C: 在所有平台上使用 TCP socket 替代 Unix 域套接字.

## 决策

选择方案 B: `#[cfg(unix)]` 编译期排除.

## 理由

- `#[cfg(unix)]` 由 Rust 编译器底层保证, 优先级高于 feature gate.
- 核心监督能力在所有平台上可编译, 仅 dashboard/IPC 被裁剪.
- Windows 等非 Unix 平台不需要额外的 Cargo feature 开关.
- Rust 编译期保证比构建时脚本检查更可靠.

## 后果

- 正面: 平台安全由编译器保证.
- 正面: 不需要维护 feature gate 矩阵.
- 负面: Windows 等非 Unix 平台无法使用 dashboard.
- 负面: 非 Unix 用户需通过 relay 实例间接访问 dashboard.

## 关联

- 关联 ADR: ADR-009
- 关联 Spec: `specs/006-1-platform-docs-ipc-security/`
