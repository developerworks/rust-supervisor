# Contract(契约): Target process IPC(目标进程进程间通信)

## Transport(传输)

- 传输使用 Unix domain socket(Unix 域套接字).
- 编码使用 newline-delimited JSON(按行分隔的 JSON 数据).
- 每一行必须是一个完整 JSON object(JSON 对象).
- 目标进程 IPC(进程间通信) 只监听本机 path(路径), 不监听 TCP(传输控制协议).

## Request(请求)

```json
{
  "request_id": "01HV0000000000000000000001",
  "method": "snapshot",
  "params": {
    "target_id": "payments-worker-a"
  }
}
```

### Methods(方法)

- `hello`: sidecar(侧车进程) 建立 IPC(进程间通信) 后声明协议版本和目标身份.
- `snapshot`: 读取 DashboardSnapshot(看板快照).
- `events.subscribe`: 订阅目标进程主动推送的 EventRecord(事件记录).
- `logs.tail`: 订阅 LogRecord(日志记录) 和最近日志.
- `command.restart_child`: 重启 child task(子任务).
- `command.pause_child`: 暂停 child task(子任务).
- `command.resume_child`: 恢复 child task(子任务).
- `command.quarantine_child`: 隔离 child task(子任务).
- `command.remove_child`: 移除 child task(子任务).
- `command.add_child`: 添加 child task(子任务).
- `command.shutdown_tree`: 关闭监督树.

未知 method(方法), 旧协议别名和历史控制命令别名必须返回 `unsupported_method` 错误, 不得映射到上述有效 method(方法).

## Response(响应)

```json
{
  "request_id": "01HV0000000000000000000001",
  "ok": true,
  "result": {
    "type": "snapshot",
    "target_id": "payments-worker-a",
    "snapshot_generation": 42
  }
}
```

## Error(错误)

```json
{
  "request_id": "01HV0000000000000000000001",
  "ok": false,
  "error": {
    "code": "target_unavailable",
    "stage": "ipc_connect",
    "target_id": "payments-worker-a",
    "message": "target process IPC is unavailable",
    "retryable": true
  }
}
```

## Server push(服务端主动推送)

目标进程和 sidecar(侧车进程) 建立 IPC(进程间通信) 后, 目标进程必须主动发送事件, 日志和可用状态变化.

```json
{
  "type": "event",
  "target_id": "payments-worker-a",
  "sequence": 1024,
  "correlation_id": "restart-7",
  "event_type": "child_restarted",
  "severity": "warning",
  "target_path": "/root/payment_loop",
  "child_id": "payment_loop",
  "occurred_at": "2026-05-05T12:00:00Z",
  "config_version": "supervisor-OneForAll-policy-10-30-shutdown-5000-observe-256",
  "payload": {}
}
```

## Command request params(命令请求参数)

```json
{
  "command_id": "01HV0000000000000000000100",
  "target_id": "payments-worker-a",
  "target": {
    "child_path": "/root/payment_loop"
  },
  "reason": "operator requested restart after upstream recovery",
  "requested_by": "subject-from-mtls",
  "confirmed": true
}
```

### Command rules(命令规则)

- `requested_by` 必须由 sidecar(侧车进程) 从 RemoteIdentity(远程身份) 派生.
- 目标进程不得信任客户端直接提供的 `requested_by`.
- `reason` 必须非空.
- `command.shutdown_tree`, `command.remove_child` 和 `command.add_child` 必须要求 `confirmed=true`.
- 每个 accepted(已接受), rejected(已拒绝) 和 completed(已完成) 命令都必须产生 audit event(审计事件).
- 任何历史控制命令别名都必须返回结构化拒绝错误, 不得执行别名对应行为.
