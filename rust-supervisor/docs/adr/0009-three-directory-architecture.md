# ADR-009: 三目录架构 (核心库 + Relay + UI)

- **日期**: 2026-05-17
- **状态**: Accepted

## 背景

Dashboard(看板) 功能需要远程可视化. 目标进程不应暴露到网络, 需要中间层做协议翻译和会话管理.

## 可选方案

- 方案 A: 目标进程直接提供 HTTP API. 实现简单, 但暴露到网络.
- 方案 B: 目标进程只做 Unix 域套接字, 引入独立 relay 做协议翻译, 独立 UI 做渲染.
- 方案 C: 目标进程内嵌 dashboard server. 耦合度高.

## 决策

选择方案 B: 三目录架构.

## 理由

- 目标进程不暴露到网络: 安全, 仅 Unix 域套接字.
- Relay 进程聚合多个 target, 翻译协议, 管理 mTLS 会话.
- UI 进程 (Vue + shadcn-vue) 只通过 wss:// 连接 relay, 不直接连接 target.
- 三目录独立演化, 各组件可独立部署和升级.

## 后果

- 正面: 安全边界清晰: target 无网络监听.
- 正面: 组件解耦, 各仓库独立迭代.
- 正面: 非 Unix 平台通过 `#[cfg(unix)]` 编译期裁剪 dashboard 模块.
- 负面: 需要维护三个仓库.
- 负面: 部署拓扑比单体复杂.

## 关联

- 关联 ADR: ADR-010 (Unix-only IPC)
- 关联 Spec: `specs/003-supervisor-dashboard/`, `specs/006-1-platform-docs-ipc-security/`
