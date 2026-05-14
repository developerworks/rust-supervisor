# Dashboard(看板) 三端使用流程

语言: [English](../en/dashboard.html)

dashboard(看板) 功能由三个仓库共同完成. `rust-supervisor` 只负责 target process(目标进程) 本机 IPC(进程间通信) 和 shared contract(共享契约). `~/rust-supervisor-relay` 负责 relay(中继) 和外部 `wss://` session(会话). `~/rust-supervisor-ui` 负责 browser dashboard client(浏览器看板客户端).

## 三端职责

- `rust-supervisor`: target process(目标进程) 读取 `SupervisorConfig`(监督器配置), 在 `ipc.enabled=true` 时打开 Unix domain socket(Unix 域套接字), 并生成 snapshot(快照), event record(事件记录), log record(日志记录), command result(命令结果) 和 registration heartbeat(注册心跳).
- `rust-supervisor-relay`: relay(中继) 监听 registration socket(注册套接字), 保存 target registry(目标注册表), 对外提供 `wss://` dashboard session(看板会话), 校验 mTLS(双向传输层安全协议认证) 和 allowed IPC path prefix(允许的进程间通信路径前缀), 并把会话命令转发到 target process(目标进程).
- `rust-supervisor-ui`: dashboard client(看板客户端) 通过 `wss://` 连接 relay(中继), 显示 target list(目标列表), topology(拓扑), state(状态), event stream(事件流), log tail(日志尾部) 和 command audit(命令审计).

## 本地演示流程

1. 先启动 relay(中继). 它必须先监听 registration socket(注册套接字), target process(目标进程) 才能注册自己.

```bash
cd ~/rust-supervisor-relay
cargo run -- --config examples/config/dashboard-relay.local.yaml
```

2. 再启动 target process(目标进程). 它会打开本机 IPC(进程间通信) socket(套接字), 并向 relay(中继) 发送 registration heartbeat(注册心跳).

```bash
cd ~/rust-supervisor
cargo run --example demo -- --config examples/config/supervisor.local.yaml
```

3. 最后启动 dashboard client(看板客户端). browser script(浏览器脚本) 只连接 relay(中继), 不直接读取 target process(目标进程) 的本机 IPC(进程间通信) socket(套接字).

```bash
cd ~/rust-supervisor-ui
VITE_SUPERVISOR_RELAY_URL=wss://localhost:9443/supervisor npm run dev
```

## 运行顺序

relay(中继) 接收到 registration heartbeat(注册心跳) 后, 只把 target process(目标进程) 放入 target registry(目标注册表). 这个注册动作不会触发 event(事件) 或 log(日志) 主动推送. dashboard client(看板客户端) 建立 authenticated dashboard session(已认证看板会话) 并选择目标后, relay(中继) 才连接 target process(目标进程) IPC(进程间通信) socket(套接字), 读取 state(状态), 并按会话请求订阅 `events.subscribe` 或 `logs.tail`.

控制命令必须从 dashboard client(看板客户端) 发起, 经过 relay(中继) session validation(会话校验), 再发送到 target process(目标进程). 每个命令必须带上 operator identity(操作者身份), target identity(目标身份) 和 reason(原因). dangerous command(危险命令) 还必须在 client(客户端) 中完成确认.

## 验证命令

```bash
cd ~/rust-supervisor
cargo test --test dashboard_config_test --test dashboard_protocol_shape_test --test dashboard_state_test --test dashboard_stream_test --test dashboard_performance_test

cargo test --manifest-path ~/rust-supervisor-relay/Cargo.toml
npm --prefix ~/rust-supervisor-ui run test
npm --prefix ~/rust-supervisor-ui run build
npm --prefix ~/rust-supervisor-ui run test:e2e:three-end
```

## 生产接入注意事项

target process(目标进程) 只能暴露本机 Unix domain socket(Unix 域套接字), 不能直接把 IPC(进程间通信) 暴露到外网. relay(中继) 对外只能使用 `wss://`. mTLS(双向传输层安全协议认证) client certificate(客户端证书) 由 browser(浏览器) 或 operating system certificate store(操作系统证书库) 选择, page script(页面脚本) 不能读取证书私钥. `ipc.path`, `registration.relay_registration_path` 和 relay(中继) 的 allowed IPC path prefix(允许的进程间通信路径前缀) 必须同时匹配, 否则目标会注册失败或被 relay(中继) 拒绝连接.
