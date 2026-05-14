# Dashboard Three-End Workflow

Language: [中文](../zh/dashboard.html)

The dashboard feature is delivered by three repositories. `rust-supervisor` owns only target-process local IPC and shared contracts. `~/rust-supervisor-relay` owns the relay and external `wss://` sessions. `~/rust-supervisor-ui` owns the browser dashboard client.

## Three-End Responsibilities

- `rust-supervisor`: The target process reads `SupervisorConfig`, opens a Unix domain socket when `ipc.enabled=true`, and produces snapshots, event records, log records, command results, and registration heartbeats.
- `rust-supervisor-relay`: The relay listens on the registration socket, stores the target registry, exposes external `wss://` dashboard sessions, validates mTLS and allowed IPC path prefixes, and forwards session commands to the target process.
- `rust-supervisor-ui`: The dashboard client connects to the relay through `wss://` and displays the target list, topology, state, event stream, log tail, and command audit.

## Local Demo Flow

1. Start the relay first. It must listen on the registration socket before the target process can register itself.

```bash
cd ~/rust-supervisor-relay
cargo run -- --config examples/config/dashboard-relay.local.yaml
```

2. Start the target process next. It opens the local IPC socket and sends registration heartbeats to the relay.

```bash
cd ~/rust-supervisor
cargo run --example demo -- --config examples/config/supervisor.local.yaml
```

3. Start the dashboard client last. Browser code connects only to the relay and never reads the target-process local IPC socket directly.

```bash
cd ~/rust-supervisor-ui
VITE_SUPERVISOR_RELAY_URL=wss://localhost:9443/supervisor npm run dev
```

## Runtime Order

After receiving a registration heartbeat, the relay only stores the target process in the target registry. Registration does not trigger proactive event or log push. After the dashboard client establishes an authenticated dashboard session and selects a target, the relay connects to the target-process IPC socket, reads state, and subscribes to `events.subscribe` or `logs.tail` only when the session requests those streams.

Control commands must start from the dashboard client, pass relay session validation, and then reach the target process. Each command must carry operator identity, target identity, and reason. Dangerous commands must also be confirmed in the client.

## Verification Commands

```bash
cd ~/rust-supervisor
cargo test --test dashboard_config_test --test dashboard_protocol_shape_test --test dashboard_state_test --test dashboard_stream_test --test dashboard_performance_test

cargo test --manifest-path ~/rust-supervisor-relay/Cargo.toml
npm --prefix ~/rust-supervisor-ui run test
npm --prefix ~/rust-supervisor-ui run build
npm --prefix ~/rust-supervisor-ui run test:e2e:three-end
```

## Production Notes

The target process may expose only a local Unix domain socket and must not expose IPC directly to the network. The relay must use `wss://` for external access. The browser or operating-system certificate store selects the mTLS client certificate, and page scripts must not read the certificate private key. `ipc.path`, `registration.relay_registration_path`, and the relay allowed IPC path prefix must match, otherwise the target will fail to register or the relay will reject the connection.
