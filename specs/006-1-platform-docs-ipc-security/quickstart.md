# Quickstart(快速开始): IPC 安全接入阅读顺序

**Feature(功能)**: 006-1-platform-docs-ipc-security
**Phase(阶段)**: 1 (设计)
**Date(日期)**: 2026-05-17

## 谁会需要这份文档

- **集成工程师**: 接入监督库前, 需要理解平台边界和支持矩阵.
- **运维负责人**: 部署中继 (relay) 与目标进程时, 需要配置套接字权限和审计.
- **安全复核员**: 验收前需要逐项勾验 9 项 IPC 控制点.

## 阅读顺序

### 步骤 1: 理解平台边界 (5 分钟)

阅读仓库顶层 `README.md` 中的 "Platform Support(平台支持)" 表格.

关键要点:

- 本项目只支持 Unix-like(类 Unix 系统) 的 dashboard IPC(看板进程间通信).
- 非类 Unix 主机上 `#[cfg(unix)]` 会自动排除整个 dashboard 模块.
- 核心监督能力 (启动, 停止, 重启, 监控) 不依赖 dashboard IPC, 可在任意 Rust(编程语言) 支持的平台上编译.

### 步骤 2: 理解三目录架构 (5 分钟)

阅读 `README.md` 中的 "Architecture(架构)" 小节.

三件套:
| 组件 | 职责 | IPC 路径 |
|------|------|----------|
| core library(核心库) | 监督生命周期管理 | 无直接 IPC |
| relay(中继) | 聚合多目标, 翻译协议, 暴露给 UI(用户界面) | 接收目标注册, 监听 UI WebSocket(网页套接字) |
| user interface(用户界面) | 渲染看板 | 连接 relay WebSocket(网页套接字) |

### 步骤 3: 理解 9 项控制点 (15 分钟)

阅读 `contracts/ipc-control-points.md`.

每项控制点包含: 输入, 检查时机, 检查逻辑, 输出, 错误码, 放行/拒绝样本.
建议按执行顺序阅读: C6 → C5 → C2 → C4 → C3 → C9 → C8 → C7.

### 步骤 4: 配置 IPC 安全 (10 分钟)

配置文件示例 (YAML):

```yaml
dashboard:
  ipc:
    enabled: true
    path: "/run/myapp/supervisor.sock"
    permissions: "0600"
    bind_mode: "replace_stale"

ipc_security:
  peer_identity:
    enabled: true
    require_uid_match: true

  authorization:
    enabled: true
    allowed_uids: [0, 1000] # root and app user

  replay_protection:
    enabled: true
    window_size: 1024
    ttl_seconds: 60

  request_size_limit:
    enabled: true
    max_bytes: 65536

  rate_limit:
    enabled: true
    refill_rate: 100.0
    burst_capacity: 20

  audit:
    enabled: true
    backend: "file"
    file_path: "/var/log/myapp/audit.jsonl"
    failure_strategy: "fail_closed"

  idempotency:
    enabled: true
    result_cache_ttl_seconds: 60
    max_cached_results: 1024

  allowlist:
    enabled: true
    allowed_paths: [] # deny all external commands
```

### 步骤 5: 运行验收测试 (5 分钟)

```bash
cargo test --test ipc_security_integration
```

验收测试覆盖:

- 9 项控制点各一组放行样本 (prove allow path works)
- 9 项控制点各一组拒绝样本 (prove deny path works)
- 拒绝后审计记录写入验证
- 监督状态迁移不得在拒绝后发生

## 常见问题

### Q: 非 Unix 平台上能用监督库吗?

能. 核心监督能力 (启动, 停止, 重启, 监控) 在所有 Rust(编程语言) 支持的平台上编译. 但 dashboard IPC(看板进程间通信) 仅在 Unix-like(类 Unix 系统) 上可用. 在 Windows 上编译时, `#[cfg(unix)]` 会自动排除 dashboard 模块.

### Q: 如何放行非 root 用户执行控制命令?

在配置中修改 `ipc_security.authorization.allowed_uids`, 加入目标用户的 uid(用户标识):

```yaml
ipc_security:
  authorization:
    allowed_uids: [0, 1000]
```

### Q: 审计后端选 memory 还是 file?

- `memory`: 不持久化, 重启丢失. 适合开发环境或低安全要求场景.
- `file`: 追加 JSON Lines 文件. 适合生产环境审计归档.

### Q: fail_closed 和 defer_bounded 怎么选?

- `fail_closed`: 审计写失败时拒绝所有写命令. 安全优先, 可用性可能降低.
- `defer_bounded`: 审计写失败时入队延迟写入. 可用性优先, 但有队列满丢审计条目的风险.
