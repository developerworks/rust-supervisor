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
  registration:
    enabled: true
    relay_registration_path: /run/rust-supervisor/dashboard-relay-registration.sock
    display_name: "payments worker a"
    authorization_scope: "payments:operate"
    lease_seconds: 30
```

预期结果: 目标进程启动后打开本机 Unix domain socket(Unix 域套接字), 并在 IPC(进程间通信) 就绪后向 relay(中继) 提交 dynamic registration(动态注册). 目标进程不会监听外网 TCP(传输控制协议) 端口.

## 2. 配置 relay(中继)

在 `/Users/0x00/Documents/rust-supervisor-relay` 创建 `examples/config/dashboard-relay.yaml`.

```yaml
listen:
  bind: "0.0.0.0:9443"
  public_url: "wss://localhost:9443/supervisor"
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
authorization_defaults:
  unknown_scope_policy: reject
```

预期结果: relay(中继) 配置留在 `/Users/0x00/Documents/rust-supervisor-relay`, 并等待目标进程提交 dynamic registration(动态注册). 重复 `target_id`, 重复 `ipc_path`, 空授权范围或无效租约会在注册阶段失败, 并显示冲突项.

## 3. 启动 relay(中继)

```bash
cargo run --manifest-path /Users/0x00/Documents/rust-supervisor-relay/Cargo.toml -- --config /Users/0x00/Documents/rust-supervisor-relay/examples/config/dashboard-relay.yaml
```

预期结果: relay(中继) 监听 `wss://localhost:9443/supervisor`, 并打开本机 registration(注册) 入口. 目标进程注册后进入 target process list(目标进程列表), 但在远程客户端 control session(控制会话) 建立前, relay(中继) 不连接或绑定目标进程 IPC(进程间通信), 也不触发事件日志主动推送.

## 4. 启动 dashboard client(看板客户端)

```bash
npm --prefix /Users/0x00/Documents/rust-supervisor-ui install
npm --prefix /Users/0x00/Documents/rust-supervisor-ui run dev
```

预期结果: dashboard client(看板客户端) 从 `/Users/0x00/Documents/rust-supervisor-ui` 启动, 浏览器通过 `wss://` 连接 relay(中继). client certificate(客户端证书) 由操作系统或浏览器证书库选择, 页面脚本不直接读取证书私钥.
前端实现必须使用 Vue(网页界面框架), shadcn-vue(组件库) 和 Tailwind(样式框架). `package.json`, `components.json`, `tailwind.config.ts` 和 `src/assets/main.css` 必须位于 `/Users/0x00/Documents/rust-supervisor-ui`.

## 5. 验证用户故事一

1. 启动两个目标进程, 分别使用不同 IPC path(进程间通信路径), 并确认它们向 relay(中继) 完成 dynamic registration(动态注册).
2. 使用有效 client certificate(客户端证书) 打开 dashboard(看板).
3. 确认 2 秒内显示 target process list(目标进程列表) 和至少一个 snapshot(快照).
4. 确认每个可达目标进程显示 root supervisor(根监督器), child task(子任务), dependencies(依赖), lifecycle state(生命周期状态), health(健康状态), readiness(就绪状态), restart count(重启次数), shutdown state(关闭状态) 和 generated time(生成时间).

## 6. 验证用户故事二

1. 让目标进程产生启动, 失败, 重启和关闭事件.
2. 确认只有 dashboard session(看板会话) 建立并绑定目标后, relay(中继) 才建立 IPC(进程间通信) 订阅.
3. 确认 IPC(进程间通信) 订阅建立后目标进程主动推送 event(事件) 和 log(日志).
4. 确认同一 target process(目标进程) 的 sequence(序号) 没有倒序.
5. 使用 target id(目标标识), child task(子任务), lifecycle state(生命周期状态), event type(事件类型), severity(严重程度), sequence(序号) 和 correlation id(关联标识) 过滤.
6. 人为缩小 event journal(事件日志缓冲区) 容量, 确认 dashboard(看板) 显示 dropped count(丢弃数量).

## 7. 验证用户故事三

1. 使用已授权身份建立 control session(控制会话).
2. 对一个 child task(子任务) 执行 restart child(重启子任务), pause child(暂停子任务), resume child(恢复子任务) 和 quarantine child(隔离子任务), 并填写 reason(原因).
3. 对 shutdown tree(关闭监督树), remove child(移除子任务) 和 add child(添加子任务) 验证二次确认.
4. 确认 command result(命令结果) 返回当前连接, snapshot(快照) 或 state delta(状态增量) 更新页面.
5. 确认每个 accepted(已接受), rejected(已拒绝) 和 completed(已完成) 命令都有 audit event(审计事件).
6. 使用未认证连接或未授权身份提交命令, 确认 relay(中继) 拒绝请求, 且不转发到目标进程 IPC(进程间通信).
7. 使用 `ws://` 建立远程连接, 确认系统不得建立完整 control session(控制会话).
8. 尝试从外网直接访问目标进程 IPC(进程间通信), 确认目标进程没有外网可达入口.
9. 发送旧协议别名或历史控制命令别名, 确认系统返回明确拒绝错误, 且不得执行别名对应行为.

## 8. 运行验证命令

```bash
cargo test
cargo test --manifest-path /Users/0x00/Documents/rust-supervisor-relay/Cargo.toml
npm --prefix /Users/0x00/Documents/rust-supervisor-ui test
npm --prefix /Users/0x00/Documents/rust-supervisor-ui run test:e2e
```

预期结果: 当前仓库 Rust(编程语言) 契约测试, relay(中继) 集成测试, dashboard client(看板客户端) 单元测试和浏览器测试全部通过, 并且前端基线显示为 Vue(网页界面框架), shadcn-vue(组件库) 和 Tailwind(样式框架).
