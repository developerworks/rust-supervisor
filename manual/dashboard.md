# Dashboard IPC(看板进程间通信)

## Ownership(所有权)

当前 `rust-supervisor` 仓库只实现 target process IPC(目标进程进程间通信) 和 shared contract(共享契约). relay(中继) 必须在 `/Users/0x00/Documents/rust-supervisor-relay` 实现. dashboard client(看板客户端) 必须在 `/Users/0x00/Documents/rust-supervisor-ui` 实现.

## Target config(目标配置)

target process(目标进程) 使用 `SupervisorConfig`(监督器配置) 的 optional(可选) `ipc` section(配置节) 打开本机 Unix domain socket(Unix 域套接字).

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
    authorization_scope: "payments:operate"
    lease_seconds: 30
```

`ipc.path` 必须是 absolute path(绝对路径). `registration.authorization_scope` 必须非空. `registration.lease_seconds` 必须大于 0.

## Protocol(协议)

target IPC(目标进程进程间通信) 使用 newline-delimited JSON(按行分隔的 JSON 数据). 支持的 method(方法) 是 `hello`, `state`, `events.subscribe`, `logs.tail`, `command.restart_child`, `command.pause_child`, `command.resume_child`, `command.quarantine_child`, `command.remove_child`, `command.add_child` 和 `command.shutdown_tree`.

旧协议 alias(别名) 和历史 control command alias(控制命令别名) 会返回 `unsupported_method`.

## Session gating(会话门控)

dynamic registration(动态注册) 只把 target process(目标进程) 放入 relay registry(中继注册表). 它不会触发 event(事件) 或 log(日志) 主动推送. relay(中继) 必须在 authenticated dashboard session(已认证看板会话) 建立并绑定目标后, 才能调用 `events.subscribe` 或 `logs.tail`.

## Validation(验证)

```bash
cargo fmt --check
cargo test --test dashboard_config_test --test dashboard_protocol_shape_test --test dashboard_state_test --test dashboard_stream_test --test dashboard_performance_test
cargo test
```
