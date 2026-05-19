# 安全说明 (Security Documentation)

> 最后更新: 2026-05-19 | 对应版本: 0.1.2

## 一、概述

`rust-tokio-supervisor` 的安全设计遵循**最小暴露、编译期保证、逐层检查**原则. 核心监督库在所有 Rust 支持平台上可编译, dashboard IPC(进程间通信) 仅限 Unix 域套接字, 通过 `#[cfg(unix)]` 在非 Unix 平台编译期排除.

## 二、平台安全边界

| 维度           | 策略                              | 说明                                                  |
| -------------- | --------------------------------- | ----------------------------------------------------- |
| 网络暴露       | 目标进程不监听任何 TCP 端口       | 仅 Unix 域套接字 IPC, `#[cfg(unix)]` 编译期保证       |
| 非 Unix 平台   | Dashboard/IPC 模块编译期排除      | `ipc` 和 `dashboard` 模块被剥离, 核心监督能力保持可用 |
| 跨平台替代方案 | 通过 relay 实例间接访问 dashboard | 在 Unix 主机部署 relay, 目标进程通过注册协议连接      |

## 三、IPC 安全控制点 (C1-C9)

看板 IPC 配置了 9 项安全控制点, 经 `IpcSecurityPipeline` 统一编排.

### 3.1 执行顺序

```text
连接建立
    │
    ▼
C1: Socket Owner (bind 时校验)
    │
    ▼ (每个请求)
C6: Rate Limit ──→ C5: Size Limit ──→ C2: Peer Credentials
    │
    ▼
C4: Replay Protection ──→ C3: Authorization ──→ C8: Idempotency
    │
    ▼
[Dispatch 执行]
    │
    ▼
C7: Audit Persistence

C9: Allowlist (在扩展点上执行, 不属于主线)
```

### 3.2 控制点详情

| 编号 | 控制点                        | 触发时机 | 默认配置                           | 绕过后果           |
| ---- | ----------------------------- | -------- | ---------------------------------- | ------------------ |
| C1   | Socket Owner(套接字所有者)    | bind()   | 进程 UID 与 socket 文件 owner 一致 | 未授权进程可连接   |
| C2   | Peer Credentials(对端身份)    | 每个请求 | `PeerIdentity` 验证                | 身份伪造           |
| C3   | Authorization(命令授权)       | 分派前   | `IpcMethod` + `PeerIdentity`       | 命令越权执行       |
| C4   | Replay Protection(重放防护)   | 每个请求 | window=1024, TTL=60s               | 重放攻击           |
| C5   | Size Limit(请求大小)          | 读取后   | max_bytes=65536                    | 缓冲区溢出         |
| C6   | Rate Limit(速率限制)          | 每个请求 | 100/s, burst=20                    | DoS 攻击           |
| C7   | Audit Persistence(审计持久化) | 分派后   | ring buffer=4096 (内存)            | 审计缺失           |
| C8   | Idempotency(幂等键)           | 分派前   | cache TTL=60s, max=1024            | 命令重复执行       |
| C9   | Allowlist(白名单)             | 扩展点   | 默认空 (禁用)                      | 未授权外部命令执行 |

### 3.3 失效策略

任一控制点不达标时, 高风险写指令一律拒绝并写入带流水号的审计条目. 审计卷满载时的策略:

- 默认: **fail closed**(拒绝高风险写动作)
- 可配置: **defer with bounded queue**(延迟落盘且有界队列)
- 禁止: 静默丢弃审计条目

## 四、供应链安全

### 4.1 依赖审计

| 检查项     | 工具                          | 策略                                                   |
| ---------- | ----------------------------- | ------------------------------------------------------ |
| 已知漏洞   | `cargo audit`                 | deny (阻断)                                            |
| 许可证合规 | `cargo deny check licenses`   | allow MIT/Apache-2.0/BSD-3-Clause/ISC/Unicode-3.0/Zlib |
| 安全公告   | `cargo deny check advisories` | vulnerability=deny, unsound=deny, yanked=deny          |
| 许可证类型 | `cargo deny`                  | copyleft=warn, unlicensed=deny                         |

### 4.2 软件物料清单 (SBOM)

发布时生成两份 SBOM:

- `artifacts/sbom/rust-supervisor.cdx.json` — CycloneDX 1.5
- `artifacts/sbom/rust-supervisor.spdx.json` — SPDX 2.3

SBOM 包含所有直接依赖和传递依赖, 每条依赖记录版本、许可证、校验和、注册表来源. 拒绝 secret、token、本地绝对路径和构建临时目录进入发布产物.

### 4.3 供应链证明

发布时生成 `artifacts/attestation.json`, 包含:

- 版本号和 Git commit hash
- 质量门禁结果摘要
- 所有产物的 SHA256 校验和

验证方式: `bash scripts/verify-attestation.sh`. 比对 `artifacts/release-record.json` 中登记的哈希值.

## 五、代码安全实践

### 5.1 类型安全

- 使用 `TaskFailureKind` 枚举而非字符串表达失败类别, 消除字符串注入风险
- 策略管线产出结构化的 `RestartDecision`, 不依赖字符串推断
- `SupervisorEvent` 是类型化结构体, 非自由格式 JSON

### 5.2 并发安全

- `CancellationToken` 提供单向关闭传播: 父令牌可取消子令牌, 子令牌不可反向取消父令牌
- `mpsc` 通道承载控制命令, `broadcast` 通道分发事件
- `Mutex` 保护共享状态, `Arc` 管理所有权
- Loom 测试在夜间 CI 中检测并发正确性

### 5.3 编译期安全

- `#[cfg(unix)]` 保证 dashboard/IPC 模块不在非 Unix 平台编译
- 禁止 `unsafe` 代码 (未发现 `unsafe` 使用)
- 禁止 inline unit test 注入生产代码

### 5.4 审计追踪

每个控制命令生成 audit event, 包含:

- `command_id`: 命令唯一标识
- `requested_by`: 请求者标识 (非空)
- `reason`: 操作原因 (非空)
- `target_path`: 目标路径
- `accepted_at`: 接受时间
- `result`: 执行结果

## 六、安全配置建议

### 6.1 IPC 配置 (Unix only)

```yaml
ipc:
  enabled: true
  path: /run/rust-supervisor/target.sock
  permissions: "0600" # 仅所有者可读写
  bind_mode: create_new # 拒绝覆盖现有 socket (防符号链接攻击)
```

### 6.2 安全清单

- [ ] IPC socket 路径使用绝对路径
- [ ] socket 权限设置为 `0600` (仅进程所有者)
- [ ] `bind_mode` 使用 `create_new` 拒绝符号链接覆盖
- [ ] IPC 注册不使用网络中继
- [ ] 审计功能开启 (`audit_enabled: true`)
- [ ] 依赖审计通过 (`cargo audit --deny warnings`)
- [ ] SBOM 校验通过
- [ ] 签名标签已验证

## 七、已知安全限制

| 限制               | 说明                                          | 缓解措施                                              |
| ------------------ | --------------------------------------------- | ----------------------------------------------------- |
| IPC 仅 Unix        | Windows 等非 Unix 平台无法使用 dashboard      | 通过 Unix relay 间接访问                              |
| 审计持久化默认内存 | `AuditBackend` 默认使用 ring buffer, 重启丢失 | 配置 `audit_persistence=file` 落盘                    |
| 配置不支持热更新   | 修改安全配置需重启 supervisor                 | 使用配置管理工具自动化重启                            |
| 无内置 mTLS        | target 侧不处理 mTLS                          | mTLS 由 relay 侧管理, 参考 rust-supervisor-relay 文档 |

## 八、混沌测试安全约束

混沌测试套件遵循以下安全约束，确保不会影响生产环境安全:

- **不修改生产代码**: 所有混沌场景代码位于 `tests/chaos/`，仅通过 `[dev-dependencies]` 引用，`cargo build --release` 不包含混沌代码。
- **进程级隔离**: 混沌 harness 仅通过测试夹具注入故障，不允许修改默认发布二进制行为。宪章要求"混沌 harness 只允许通过测试夹具注入故障"。
- **panic 隔离**: 子任务 panic 通过 `std::panic::catch_unwind` 捕获。控制循环已配置 `std::panic::set_hook` 记录结构化错误并继续执行，不会因子任务 panic 而终止。Spec 中"panic"指 Rust 语言级 panic；进程级崩溃使用"crash"表述。
- **时间隔离**: 时钟回拨场景仅模拟 wall clock 回退，不修改系统 `CLOCK_MONOTONIC`。滑动窗口和熔断器使用 `std::time::Instant` (monotonic clock)，不受 wall clock 回退影响。
- **网络隔离**: IPC 连接风暴场景使用独立临时 socket 路径，不影响生产 IPC 端点。

## 九、相关文档

- [IPC 安全控制点契约](../specs/006-1-platform-docs-ipc-security/contracts/ipc-control-points.md)
- [平台支持矩阵](architecture.md#45-平台编译隔离)
- [架构 - IPC 安全控制点](architecture.md#七ipc-安全控制点)
- [发布门禁与供应链](en/quality-gates.md)
- [发布记录](../artifacts/release-record.json)
