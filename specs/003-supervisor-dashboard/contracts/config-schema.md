# Contract(契约): 配置结构

## Target process config(目标进程配置)

目标进程通过 `SupervisorConfig`(监督器配置) 的 optional(可选) `ipc` 配置节打开本机 IPC(进程间通信).

```yaml
ipc:
  enabled: true
  target_id: payments-worker-a
  path: /run/rust-supervisor/payments-worker-a.sock
  permissions: "0600"
  bind_mode: create_new
```

### Rules(规则)

- `ipc.enabled=false` 时, 目标进程不得打开 IPC(进程间通信).
- `ipc.enabled=true` 时, `ipc.path` 必须是绝对 path(路径).
- `ipc.target_id` 必须非空, 并且应该在 sidecar(侧车进程) 配置中有对应 target(目标).
- `ipc.permissions` 默认是 `0600`.
- `bind_mode=create_new` 时, path(路径) 已存在必须失败并返回结构化配置错误.

## Sidecar config(侧车进程配置)

sidecar(侧车进程) 使用独立 YAML(配置文件格式) 配置 `wss://` 监听地址, mTLS(双向传输层安全协议认证), trusted proxy(可信代理) 和多个目标进程.

```yaml
listen:
  bind: "0.0.0.0:9443"
  public_url: "wss://dashboard.example.test/supervisor"
tls:
  certificate_path: "./certs/sidecar.crt"
  private_key_path: "./certs/sidecar.key"
  client_ca_path: "./certs/operators-ca.crt"
trusted_proxy:
  enabled: false
  allowed_remote_addrs: []
  identity_header: "x-verified-client-subject"
targets:
  - target_id: payments-worker-a
    display_name: "payments worker a"
    ipc_path: /run/rust-supervisor/payments-worker-a.sock
    authorization_scope: "payments:operate"
  - target_id: billing-worker-a
    display_name: "billing worker a"
    ipc_path: /run/rust-supervisor/billing-worker-a.sock
    authorization_scope: "billing:observe"
```

### Rules(规则)

- `listen.public_url` 必须使用 `wss://`.
- `tls.client_ca_path` 必须存在, 除非 `trusted_proxy.enabled=true` 且 sidecar(侧车进程) 只接受可信代理地址.
- `targets[].target_id` 必须唯一.
- `targets[].ipc_path` 必须唯一.
- `targets[].ipc_path` 必须是绝对 path(路径).
- 配置冲突必须在 sidecar(侧车进程) 启动阶段失败, 并指出冲突 target id(目标标识) 或 IPC path(进程间通信路径).
