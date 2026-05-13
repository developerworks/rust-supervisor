# Contract(契约): 配置结构

**Owner(所有者)**: 目标进程配置由 `/Users/0x00/Documents/rust-supervisor` 实现. relay(中继) 配置由 `/Users/0x00/Documents/rust-supervisor-relay` 实现. UI(用户界面) 不直接读取目标进程 IPC(进程间通信) 配置.

## Target process config(目标进程配置)

目标进程通过 `SupervisorConfig`(监督器配置) 的 optional(可选) `ipc` 配置节打开本机 IPC(进程间通信).

```yaml
ipc:
  enabled: true
  target_id: payments-worker-a
  path: /run/rust-supervisor/payments-worker-a.sock
  permissions: "0600"
  bind_mode: create_new
  registration:
    enabled: true
    relay_registration_path: /run/rust-supervisor/dashboard-relay-registration.sock
    display_name: "payments worker a"
    lease_seconds: 30
    registration_heartbeat_interval_seconds: 15
```

### Rules(规则)

- `ipc.enabled=false` 时, 目标进程不得打开 IPC(进程间通信).
- `ipc.enabled=true` 时, `ipc.path` 必须是绝对 path(路径).
- `ipc.target_id` 必须非空.
- `ipc.permissions` 默认是 `0600`.
- `bind_mode=create_new` 时, path(路径) 已存在必须失败并返回结构化配置错误.
- `ipc.registration.enabled=true` 时, 目标进程必须在 IPC(进程间通信) 就绪后向 relay(中继) 提交 dynamic registration(动态注册).
- `ipc.registration.relay_registration_path` 必须是本机绝对 path(路径).
- `ipc.registration.lease_seconds` 必须大于 0.
- `ipc.registration.registration_heartbeat_interval_seconds` 必须大于 0, 并且必须小于 `ipc.registration.lease_seconds`.

## Relay config(中继配置)

relay(中继) 使用独立 YAML(配置文件格式) 配置 `wss://` 监听地址, mTLS(双向传输层安全协议认证), trusted proxy(可信代理), registration(注册) 入口和租约规则. 目标进程列表不得写死在 relay(中继) 配置中. 该配置文件必须放在 `/Users/0x00/Documents/rust-supervisor-relay`.

```yaml
listen:
  bind: "0.0.0.0:9443"
  public_url: "wss://dashboard.example.test/supervisor"
tls:
  certificate_path: "./certs/relay.crt"
  private_key_path: "./certs/relay.key"
  client_ca_path: "./certs/operators-ca.crt"
trusted_proxy:
  enabled: false
  allowed_remote_addrs: []
  identity_header: "x-verified-client-subject"
registration:
  listen_path: /run/rust-supervisor/dashboard-relay-registration.sock
  permissions: "0600"
  allowed_ipc_path_prefixes:
    - /run/rust-supervisor/
  default_lease_seconds: 30
  max_lease_seconds: 120
```

### Rules(规则)

- `listen.public_url` 必须使用 `wss://`.
- `tls.client_ca_path` 必须存在, 除非 `trusted_proxy.enabled=true` 且 relay(中继) 只接受可信代理地址.
- `registration.listen_path` 必须是本机绝对 path(路径), 且不得暴露到外网.
- `registration.allowed_ipc_path_prefixes` 为空时必须拒绝目标进程注册.
- relay(中继) 必须在运行时拒绝不同 owner identity(所有者身份) 覆盖相同 target id(目标标识), 重复 IPC path(进程间通信路径), 非绝对 IPC path(进程间通信路径), supported_commands(支持的命令) 结构无效和无效租约.
- 注册冲突必须返回结构化错误, 并指出冲突 target id(目标标识) 或 IPC path(进程间通信路径).
- 目标进程只完成注册时不得触发事件日志主动推送. 已认证客户端会话建立并绑定目标后, relay(中继) 才能连接目标进程 IPC(进程间通信) 并建立 subscription(订阅).
