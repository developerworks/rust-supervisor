# Contracts(接口契约): IPC 控制点 C1-C9

**Feature(功能)**: 006-1-platform-docs-ipc-security
**Phase(阶段)**: 1 (设计)
**Date(日期)**: 2026-05-17

本文档定义 9 项 IPC 控制点的接口契约: 输入, 输出, 错误码, 以及放行/拒绝判定规则.

---

## C1: Socket Owner(套接字所有者) 校验

### 契约

- **输入**: 绑定中的 Unix Domain Socket(Unix 域套接字) 路径.
- **检查时机**: `bind()` 调用之前, 即 `prepare_socket_path()` 函数内.
- **检查逻辑**: 若套接字路径已存在且为 regular file(普通文件) 或 symlink(符号链接), 拒绝绑定. 若路径存在且为 socket 文件, 根据 `bind_mode` 决定替换或拒绝. 替换时检查文件所有者是否与当前进程 uid(用户标识) 一致.
- **输出**: `Ok(())` 放行; `Err(DashboardError)` 拒绝.
- **错误码**:
  - `ipc_socket_owner_mismatch` — 路径存在但所有者不是当前进程.
  - `ipc_symlink_rejected` — 路径是 symlink(符号链接).
  - `ipc_socket_exists_create_new` — bind_mode 为 CreateNew 但路径存在.

### 放行样本

- 套接字路径不存在且父目录可写.
- 套接字路径存在, 为 socket 文件, 所有者与当前进程 uid 一致, bind_mode 为 ReplaceStale.

### 拒绝样本

- 套接字路径为 symlink(符号链接).
- 套接字路径存在, 所有者与当前进程 uid 不一致.

---

## C2: Peer Credentials(对端身份) 校验

### 契约

- **输入**: `tokio::net::UnixStream` 已建立连接的套接字.
- **检查时机**: 每次 IPC 请求处理开始 (`handle_request()` 入口).
- **检查逻辑**: 通过 `std::os::unix::net::UnixStream::peer_cred()` 获取 `PeerIdentity { pid, uid, gid }`. 比对该快照与配置中的 `PeerIdentityConfig`.
  - `require_uid_match`: 若为 true, 要求 `peer.uid == current_process_uid`.
  - `allowed_gids`: 若非空, 要求 `peer.gid in allowed_gids`.
  - `allowed_pids`: 若非空, 要求 `peer.pid in allowed_pids`.
- **输出**: `Ok(PeerIdentity)` 放行; `Err(DashboardError)` 拒绝.
- **错误码**:
  - `peer_cred_uid_mismatch` — uid 不匹配.
  - `peer_cred_gid_not_allowed` — gid 不在白名单.
  - `peer_cred_pid_not_allowed` — pid 不在白名单.
  - `peer_cred_unavailable` — 内核不支持对端凭证.

### 放行样本

- `require_uid_match=true`, peer uid 与进程 uid 相同.
- `allowed_gids=[1000]`, peer gid 为 1000.

### 拒绝样本

- `require_uid_match=true`, peer uid 为 1001, 进程 uid 为 0.
- `allowed_gids=[1000]`, peer gid 为 2000.

---

## C3: Command Authorization(命令授权)

### 契约

- **输入**: `IpcMethod` (方法枚举) + `PeerIdentity` (对端身份).
- **检查时机**: 在 dispatch 之前, dispatch 内部对写命令做额外检查.
- **检查逻辑**:
  1. 用 `IpcRiskAction::classify(method)` 分类.
  2. `Read` 类: 通过 C1-C2 后直接放行.
  3. `WriteChild` 与 `Destructive` 类: 检查 `peer.uid in allowed_uids`.
- **输出**: `Ok(AuthorizationDecision::Allowed)` 放行; `Err(DashboardError)` 拒绝.
- **错误码**:
  - `authz_denied` — 命令未授权.
  - `authz_not_configured` — 授权配置缺失.

### 放行样本

- Read 方法 (`hello`, `state`) + 任意通过 C2 的身份.
- `command.restart_child` + peer uid 0 (root, 默认白名单).

### 拒绝样本

- `command.shutdown_tree` + peer uid 1000 (非 root, 默认配置).

---

## C4: Replay Protection(重放保护)

### 契约

- **输入**: `request_id: String`.
- **检查时机**: 每次请求处理开始, 在业务逻辑前.
- **检查逻辑**: 在 `ReplayWindow` 中查询 `request_id`. 若存在且未过期 → 拒绝为 replay(重放). 若不存在或已过期 → 记录并放行.
- **窗口参数**: `window_size=1024`, `ttl_seconds=60` (可配置).
- **输出**: `Ok(())` 放行; `Err(DashboardError)` 拒绝.
- **错误码**: `replay_detected` — request_id 已存在于窗口内.

### 放行样本

- 单次提交全新的 UUID(通用唯一标识符) request_id.
- 两次相同 request_id 但间隔 61 秒 (TTL(存活时间) 过期).

### 拒绝样本

- 第二次提交相同 request_id, 距第一次 5 秒.

---

## C5: Request Size Limit(请求大小限制)

### 契约

- **输入**: 请求体原始字节数 `len: usize`.
- **检查时机**: 从套接字读取完整帧后, JSON 反序列化前.
- **检查逻辑**: 若 `len > max_bytes`, 拒绝.
- **输出**: `Ok(())` 放行; `Err(DashboardError)` 拒绝.
- **错误码**: `request_too_large` — 超过 max_bytes.

### 放行样本

- 请求体 500 字节, `max_bytes=65536`.

### 拒绝样本

- 请求体 100000 字节, `max_bytes=65536`.

---

## C6: Rate Limit(速率限制)

### 契约

- **输入**: 连接标识 (per-connection `TokenBucket`).
- **检查时机**: 每次请求处理开始.
- **检查逻辑**: `token_bucket.try_consume()` → 若令牌不足, 拒绝.
- **参数**: `refill_rate=100.0/s`, `burst_capacity=20` (可配置).
- **输出**: `Ok(())` 放行; `Err(DashboardError)` 拒绝.
- **错误码**: `rate_limit_exceeded` — 速率超限.

### 放行样本

- 第 1 至第 20 个请求在 1 秒内到达 (突发容量 20).
- 匀速每 10ms 一个请求 (每秒 100 个).

### 拒绝样本

- 第 21 个请求在 0.5 秒内到达 (突发已耗尽, 补充不足).

---

## C7: Audit Persistence(审计持久化)

### 契约

- **输入**: `AuditRecord`.
- **检查时机**: 每次 IPC 写请求处理完成后.
- **检查逻辑**:
  - `backend="memory"`: 写入内存 ring buffer(环形缓冲), 容量 4096.
  - `backend="file"`: 追加一行 JSON 至 `file_path`.
  - 若 file 写入失败且 `failure_strategy="fail_closed"`: 拒绝当前请求.
  - 若 `failure_strategy="defer_bounded"`: 入队延迟写入, 队列满时丢弃最旧条目.
- **输出**: `Ok(())` 记录成功; `Err(DashboardError)` 记录失败.
- **错误码**:
  - `audit_write_failed` — 审计写入失败.
  - `audit_queue_full` — 延迟队列满.

### 放行样本

- `backend="memory"`: 始终成功 (内存操作).
- `backend="file"`: 磁盘可写且路径存在.

### 拒绝样本

- `backend="file"`, `failure_strategy="fail_closed"`, 磁盘写满.

---

## C8: Command Idempotency Key(命令幂等键)

### 契约

- **输入**: `request_id: String` + `request: IpcRequest`.
- **检查时机**: 命令类请求 dispatch 前.
- **检查逻辑**:
  1. 查询 `IdempotencyCache` 中是否存在 `request_id`.
  2. 若存在且未过期 → 返回缓存 `IpcResponse` (不重新执行).
  3. 若不存在 → 执行命令, 缓存结果.
- **缓存参数**: `result_cache_ttl_seconds=60`, `max_cached_results=1024`.
- **输出**: `Ok(IpcResponse)` — 放行或返回缓存.
- **与 C4 的关系**: C4 检查 replay, C8 检查幂等. C4 在 C8 之前执行. 若 C4 拒绝 replay, C8 不会被触发. 若 C4 窗口已清 (TTL(存活时间) 过期), 但 C8 缓存仍有效, 则 C4 放行但 C8 返回缓存结果.

### 放行样本

- 首次 `command.restart_child` → 执行并缓存.
- 60 秒后相同 request_id → C4 已清理, 重新执行 (C8 缓存也过期).

### 拒绝样本

- 首次 `command.restart_child` → 执行成功. 5 秒后相同 request_id → C4 拒绝 (replay).

---

## C9: External Command Allowlist(白名单)

### 契约

- **输入**: 可执行文件绝对路径 `path: &str`.
- **检查时机**: 控制面扩展点尝试执行外部命令时.
- **检查逻辑**: 若 `allowed_paths` 为空, 拒绝所有. 若 `allowed_paths` 非空, 检查 `path in allowed_paths`.
- **输出**: `Ok(())` 放行; `Err(DashboardError)` 拒绝.
- **错误码**:
  - `allowlist_denied` — 路径不在白名单.
  - `allowlist_empty` — 白名单为空, 所有外部命令被拒绝.

### 放行样本

- `allowed_paths=["/usr/bin/systemctl"]`, 请求 `/usr/bin/systemctl`.

### 拒绝样本

- `allowed_paths=[]`, 请求任意路径.
- `allowed_paths=["/usr/bin/systemctl"]`, 请求 `/usr/local/bin/custom`.

---

## 控制点执行顺序

每次 IPC 请求按以下顺序经过控制点:

```
Request arrives
  │
  ▼
[C6] Rate Limit ──── deny → 429 rate_limit_exceeded
  │ allow
  ▼
[C5] Size Limit ──── deny → 413 request_too_large
  │ allow
  ▼
[C1] Socket Owner ──── deny → (bind time only, not per-request)
  │ allow (bind time)
  ▼
[C2] Peer Credentials ──── deny → 403 peer_cred_*
  │ allow
  ▼
[C4] Replay Protection ──── deny → 409 replay_detected
  │ allow
  ▼
[C3] Authorization ──── deny → 403 authz_denied
  │ allow
  ▼
[C9] Allowlist ──── (only for external command extension points)
  │ allow/deny
  ▼
[C8] Idempotency ──── cache hit → 200 (cached result)
  │ cache miss
  ▼
  Dispatch (execute command)
  │
  ▼
[C7] Audit ──── (always after dispatch)
```

## Tracing(结构化追踪) Target 前缀

所有拒绝路径使用统一的 tracing(结构化追踪) target 前缀:

```
rust_supervisor::ipc::security::
```

每个控制点的子 target:

| 控制点 | tracing target                                     |
| ------ | -------------------------------------------------- |
| C1     | `rust_supervisor::ipc::security::socket_owner`     |
| C2     | `rust_supervisor::ipc::security::peer_credentials` |
| C3     | `rust_supervisor::ipc::security::authorization`    |
| C4     | `rust_supervisor::ipc::security::replay`           |
| C5     | `rust_supervisor::ipc::security::size_limit`       |
| C6     | `rust_supervisor::ipc::security::rate_limit`       |
| C7     | `rust_supervisor::ipc::security::audit`            |
| C8     | `rust_supervisor::ipc::security::idempotency`      |
| C9     | `rust_supervisor::ipc::security::allowlist`        |
