# Research(研究): 平台条件编译与 IPC 安全默认值策略

**Feature(功能)**: 006-1-platform-docs-ipc-security
**Phase(阶段)**: 0 (研究)
**Date(日期)**: 2026-05-17

## 研究问题清单

1. 平台条件编译策略: `#[cfg(unix)]` 还是 Cargo feature gate(功能开关)?
2. Unix Domain Socket(Unix 域套接字) 对端凭证 (peer credentials) 在各目标平台的可获得性.
3. 9 项 IPC 控制点的合理出厂默认值.

## 1. 平台条件编译策略

### 现状

当前代码库在 `src/dashboard/ipc_server.rs` 和 `src/dashboard/registration.rs` 中直接使用:

- `std::os::unix::fs::FileTypeExt`
- `std::os::unix::net::UnixStream as StdUnixStream`
- `tokio::net::UnixListener`
- `tokio::net::UnixStream`

所有文件均无 `#[cfg(unix)]` 保护. `Cargo.toml` 无任何 feature gate.

### 候选方案

| 方案                  | 描述                                                                     | 优点                                                | 缺点                                                                          |
| --------------------- | ------------------------------------------------------------------------ | --------------------------------------------------- | ----------------------------------------------------------------------------- |
| A: `#[cfg(unix)]`     | 条件编译属性, 非 Unix 平台直接省略整个 dashboard IPC 模块                | 零配置, 编译器自动处理; Rust 内置, 无需维护 feature | 非 Unix 平台无法编译 dashboard IPC 相关代码; 调用方需自行 `#[cfg(unix)]` 判断 |
| B: Cargo feature gate | 在 `Cargo.toml` 定义 `dashboard-ipc` feature, 默认开启, 非 Unix 用户关闭 | 用户显式控制; 可在非 Unix 编译但无 IPC 硬化         | 需维护 feature 组合; 与 `#[cfg(unix)]` 存在冗余                               |
| C: 混合 (A + B)       | `#[cfg(unix)]` 保证编译安全, feature gate 让用户显式裁剪                 | 最安全, 双重避免误用                                | 维护成本略高                                                                  |

### 建议: 方案 A (`#[cfg(unix)]`)

理由:

1. Rust 编译器自动为 `#[cfg(unix)]` 提供平台检测, 无需用户手动开关 feature. 非类 Unix 平台编译时, dashboard IPC 模块直接被省略, 调用点获得编译错误而非运行时 panic(恐慌).
2. 当前仓库无多平台支持负担. `tokio` 自身已对 Unix/Windows 做充分条件编译. 引入 feature gate 只会增加 CI(持续集成) 组合爆炸风险.
3. 方案 C 中 feature gate 的显式裁剪价值在本仓库场景有限: 如果用户的目标平台不支持 Unix Domain Socket(Unix 域套接字), 编译器已通过 `#[cfg(unix)]` 阻止编译. 再加 feature gate 属于冗余.

### 实施步骤

1. 在 `src/dashboard/mod.rs` 顶部添加 `#[cfg(unix)]` 使得整个模块仅在 Unix 平台编译.
2. 在 `src/lib.rs` 中为 `pub mod dashboard;` 添加 `#[cfg(unix)]`.
3. 所有 `src/dashboard/` 下的 `use std::os::unix::*` 和 `use tokio::net::Unix*` 自动受保护.
4. `src/main.rs` 中引用 dashboard 的代码需要用 `#[cfg(unix)]` 保护或以 `cfg_if!` 宏处理.

### 对支持矩阵的影响

- Unix-like(类 Unix): dashboard IPC 完整可用.
- 非 Unix: `#[cfg(unix)]` 排除整个 dashboard 模块. 支持矩阵标记为 "dashboard IPC: 不支持".

## 2. 对端凭证 (Peer Credentials) 可获得性

### 各平台 syscall(系统调用) 覆盖

| 平台    | 凭证机制                  | 获取方式                                                                   |
| ------- | ------------------------- | -------------------------------------------------------------------------- |
| Linux   | `SO_PEERCRED`             | `getsockopt` 返回 `ucred { pid, uid, gid }`                                |
| macOS   | `LOCAL_PEERCRED`          | `getsockopt` 返回 `xucred { cr_version, cr_uid, cr_ngroups, cr_groups[] }` |
| FreeBSD | `LOCAL_PEERCRED`          | 同 macOS                                                                   |
| Windows | 不支持 Unix Domain Socket | N/A(不适用)                                                                |

### 建议

- 不引入第三方 crate(库). 标准库 `std::os::unix::net::UnixStream` 已提供 `.peer_cred()` 方法 (Rust 1.80+ 稳定).
- C1 (socket owner 校验) 和 C2 (peer credentials 校验) 通过 `std::os::unix::net::socket_addr` 的 `as_abstract_namespace_addr` 以及 `peer_cred` 实现.
- 若标准库能力不足 (例如需要 macOS 上的 gid group list), 再评估 `libc` crate(库) 直接调用.

## 3. 9 项 IPC 控制点默认值

以下默认值按"安全优先, 可调的保守值"原则设定:

| 控制点                                  | 出厂默认值                                                   | 可配置                 | 说明                                                    |
| --------------------------------------- | ------------------------------------------------------------ | ---------------------- | ------------------------------------------------------- |
| C1: socket owner(套接字所有者)          | 仅允许与监听进程相同 uid(用户标识)                           | 是                     | 通过 `peer_cred()` 返回的 uid 比对 `std::process::id()` |
| C2: peer credentials(对端身份)          | 要求 uid 匹配, gid 不校验                                    | 是 (可配置 gid 白名单) | Linux 返回 `pid, uid, gid`; macOS 返回 `cr_uid`         |
| C3: command authorization(命令授权)     | 读写分离: hello/state 可匿名; 命令类需认证                   | 是                     | 授权矩阵: `IpcMethod × PeerIdentity → allow/deny`       |
| C4: replay protection(重放保护)         | 基于 request_id 的滑动窗口, 窗口大小 1024, TTL(存活时间) 60s | 是                     | 使用 `HashSet<String>` + 定时清理                       |
| C5: request size limit(请求大小限制)    | 64 KiB (65536 字节)                                          | 是                     | 在 JSON 反序列化前检查                                  |
| C6: rate limit(速率限制)                | 100 请求/秒 每连接                                           | 是                     | token bucket(令牌桶) 算法, 突发容量 20                  |
| C7: audit persistence(审计持久化)       | 默认仅内存环形缓冲 (ring buffer), 可配置文件路径启用持久化   | 是                     | fail closed(默认拒绝高风险写动作) 当磁盘不可写          |
| C8: command idempotency key(命令幂等键) | 基于 request_id 去重, 去重窗口与 C4 共用                     | 是                     | 重复 request_id 返回首次执行缓存结果                    |
| C9: external command allowlist(白名单)  | 默认为空数组 (全部拒绝)                                      | 是                     | 仅当配置文件显式列出可执行绝对路径时才放行              |

### 默认值推导公式

- C4 窗口: 1024 个请求 ID, 对应每秒 100 请求下约 10 秒的去重窗口.
- C5 上限: 64 KiB 足以容纳典型 JSON RPC(远程过程调用) 请求体, 同时阻止内存炸弹式攻击.
- C6 令牌桶: 容量 20 允许短时突发, 补充速率 100/s 限制平均速率.

## 结论

平台策略: 采用 `#[cfg(unix)]` 条件编译, 不引入额外 Cargo feature gate.
IPC 控制点默认值: 按上表设定, 所有值可配置.
不引入新 crate(库) 依赖: `peer_cred()` 使用标准库; token bucket 自行实现约 40 行.
