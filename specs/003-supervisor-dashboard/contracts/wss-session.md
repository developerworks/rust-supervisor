# Contract(契约): Remote `wss://` session(远程安全会话)

**Owner(所有者)**: `wss://` session(会话) 服务端由 `/Users/0x00/Documents/rust-supervisor-relay` 实现. dashboard client(看板客户端) 由 `/Users/0x00/Documents/rust-supervisor-ui` 实现. 当前 `rust-supervisor` 仓库不实现该远程会话服务端或前端客户端.

## Transport(传输)

- 外部 dashboard(看板) 连接必须使用 `wss://`.
- TLS(传输层安全协议) 握手必须在 HTTP Upgrade(HTTP 升级) 到 WebSocket(网络套接字协议) 前完成.
- 默认模式下 relay(中继) 必须验证 client certificate(客户端证书).
- `ws://` 连接不得建立完整 control session(控制会话).
- trusted proxy(可信代理) 模式必须只接受配置内代理地址传入的已验证身份字段.
- 外部客户端不得绕过 relay(中继) 直接访问目标进程 IPC(进程间通信).

## Session startup(会话启动)

服务端在 control session(控制会话) 建立后必须先发送由 active registration(活动注册) 形成的 target process list(目标进程列表) 和授权范围. 目标进程完成 dynamic registration(动态注册) 只会进入该列表, 不会因为注册本身触发事件日志主动推送.

```json
{
  "type": "session_established",
  "session_id": "01HV0000000000000000000200",
  "identity": {
    "principal": "operator@example.test",
    "source": "mtls"
  },
  "targets": [
    {
      "target_id": "payments-worker-a",
      "display_name": "payments worker a",
      "registration_state": "active",
      "connection_state": "registered",
      "authorization_scope": "payments:operate"
    }
  ]
}
```

## Server messages(服务端消息)

```json
{ "type": "state", "target_id": "payments-worker-a", "state": {} }
{ "type": "event", "target_id": "payments-worker-a", "event": {} }
{ "type": "log", "target_id": "payments-worker-a", "log": {} }
{ "type": "state_delta", "target_id": "payments-worker-a", "delta": {} }
{ "type": "command_result", "target_id": "payments-worker-a", "result": {} }
{ "type": "error", "error": {} }
```

### Ordering rules(顺序规则)

- `session_established` 必须早于 `state`, `event`, `log`, `state_delta`, `command_result` 和 `error`.
- `session_established` 发送后, relay(中继) 才能按授权目标触发 IPC(进程间通信) 绑定和 event/log subscription(事件日志订阅).
- 同一 target process(目标进程) 内的 `event.sequence` 必须按单调顺序发送给 dashboard(看板).
- IPC(进程间通信) 重连成功后, relay(中继) 必须先发送新的 `state`, 再继续发送新的 event(事件) 和 log(日志).

## Client command(客户端命令)

```json
{
  "type": "command",
  "command_id": "01HV0000000000000000000300",
  "target_id": "payments-worker-a",
  "command": "pause_child",
  "target": {
    "child_path": "/root/payment_loop"
  },
  "reason": "investigating downstream duplicate processing",
  "confirmed": false
}
```

### Authorization rules(授权规则)

- relay(中继) 必须根据 RemoteIdentity(远程身份) 派生 `requested_by`.
- 客户端消息中的 `requested_by` 必须被忽略或拒绝.
- 未认证, 未授权, 证书身份不可解析或 control session(控制会话) 未建立时, relay(中继) 不得连接 IPC(进程间通信), 不得绑定 IPC(进程间通信), 不得转发命令.
- `shutdown_tree`, `remove_child` 和 `add_child` 必须要求 `confirmed=true` 和非空 `reason`.
- 客户端发送旧协议别名或历史控制命令别名时, relay(中继) 必须返回结构化拒绝错误, 不得执行别名对应行为.

## Filters(过滤)

dashboard client(看板客户端) 可以本地过滤, 也可以向 relay(中继) 发送订阅偏好.

```json
{
  "type": "filter_update",
  "target_ids": ["payments-worker-a"],
  "child_paths": ["/root/payment_loop"],
  "lifecycle_states": ["failed", "restarting"],
  "event_types": ["child_failed", "child_restarted"],
  "severities": ["warning", "error"],
  "sequence_from": 1000,
  "correlation_id": "restart-7"
}
```

过滤不得改变目标进程内 event sequence(事件序号) 的真实顺序.
