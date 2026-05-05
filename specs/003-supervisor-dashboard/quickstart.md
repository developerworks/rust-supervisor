# Quickstart(快速开始): 监督任务可视化界面

## 1. 配置目标进程 IPC(进程间通信)

在目标进程使用的 supervisor YAML(监督器配置文件) 中启用 IPC(进程间通信).

```yaml
ipc:
  enabled: true
  target_id: payments-worker-a
  path: /run/rust-supervisor/payments-worker-a.sock
  permissions: "0600"
  bind_mode: create_new
```

预期结果: 目标进程启动后打开本机 Unix domain socket(Unix 域套接字), 并且不会监听外网 TCP(传输控制协议) 端口.

## 2. 配置 sidecar(侧车进程)

创建 `examples/config/dashboard-sidecar.yaml`.

```yaml
listen:
  bind: "0.0.0.0:9443"
  public_url: "wss://localhost:9443/supervisor"
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

预期结果: 重复 `target_id` 或重复 `ipc_path` 会在启动阶段失败, 并显示冲突项.

## 3. 启动 sidecar(侧车进程)

```bash
cargo run --bin rust-supervisor-dashboard-sidecar -- --config examples/config/dashboard-sidecar.yaml
```

预期结果: sidecar(侧车进程) 监听 `wss://localhost:9443/supervisor`, 并等待已认证 dashboard session(看板会话). 在远程客户端 control session(控制会话) 建立前, sidecar(侧车进程) 不连接或绑定目标进程 IPC(进程间通信).

## 4. 启动 dashboard(看板) 前端

```bash
npm --prefix dashboard install
npm --prefix dashboard run dev
```

预期结果: 浏览器通过 `wss://` 连接 sidecar(侧车进程). client certificate(客户端证书) 由操作系统或浏览器证书库选择, 页面脚本不直接读取证书私钥.

## 5. 验证用户故事一

1. 启动两个目标进程, 分别使用不同 IPC path(进程间通信路径).
2. 使用有效 client certificate(客户端证书) 打开 dashboard(看板).
3. 确认 2 秒内显示 target process list(目标进程列表) 和至少一个 snapshot(快照).
4. 确认每个可达目标进程显示 root supervisor(根监督器), child task(子任务), dependencies(依赖), lifecycle state(生命周期状态), health(健康状态), readiness(就绪状态), restart count(重启次数), shutdown state(关闭状态) 和 generated time(生成时间).

## 6. 验证用户故事二

1. 让目标进程产生启动, 失败, 重启和关闭事件.
2. 确认 IPC(进程间通信) 建立后目标进程主动推送 event(事件) 和 log(日志).
3. 确认同一 target process(目标进程) 的 sequence(序号) 没有倒序.
4. 使用 target id(目标标识), child task(子任务), lifecycle state(生命周期状态), event type(事件类型), severity(严重程度), sequence(序号) 和 correlation id(关联标识) 过滤.
5. 人为缩小 event journal(事件日志缓冲区) 容量, 确认 dashboard(看板) 显示 dropped count(丢弃数量).

## 7. 验证用户故事三

1. 使用已授权身份建立 control session(控制会话).
2. 对一个 child task(子任务) 执行 restart child(重启子任务), pause child(暂停子任务), resume child(恢复子任务) 和 quarantine child(隔离子任务), 并填写 reason(原因).
3. 对 shutdown tree(关闭监督树), remove child(移除子任务) 和 add child(添加子任务) 验证二次确认.
4. 确认 command result(命令结果) 返回当前连接, snapshot(快照) 或 state delta(状态增量) 更新页面.
5. 确认每个 accepted(已接受), rejected(已拒绝) 和 completed(已完成) 命令都有 audit event(审计事件).
6. 使用未认证连接或未授权身份提交命令, 确认 sidecar(侧车进程) 拒绝请求, 且不转发到目标进程 IPC(进程间通信).
7. 使用 `ws://` 建立远程连接, 确认系统不得建立完整 control session(控制会话).
8. 尝试从外网直接访问目标进程 IPC(进程间通信), 确认目标进程没有外网可达入口.
9. 发送旧协议别名或历史控制命令别名, 确认系统返回明确拒绝错误, 且不得执行别名对应行为.

## 8. 运行验证命令

```bash
cargo test
npm --prefix dashboard test
npm --prefix dashboard run test:e2e
```

预期结果: Rust(编程语言) 契约测试, sidecar(侧车进程) 集成测试, dashboard(看板) 单元测试和浏览器测试全部通过.
