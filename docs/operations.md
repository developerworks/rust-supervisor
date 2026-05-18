# 运行维护 (Operations Guide)

> 最后更新: 2026-05-18 | 对应版本: 0.1.2

## 一、概述

本文档描述 `rust-tokio-supervisor` 在生产环境中的部署、巡检和故障处理. 适用于集成工程师、SRE(站点可靠性工程师) 和值班人员.

## 二、部署

### 2.1 系统要求

| 项目      | 要求                                                              |
| --------- | ----------------------------------------------------------------- |
| Rust 版本 | MSRV 1.88 (通过 `rust-version` 在 Cargo.toml 中声明)              |
| 操作系统  | Linux / macOS / FreeBSD (Unix); Windows (核心监督能力仅 Unix IPC) |
| 内存      | 取决于子任务数量, 基准测试建议 ≥64MB                              |
| 磁盘      | 无需持久化存储 (运行时状态在内存中管理)                           |
| 依赖      | Tokio 1.52.3, 无外部数据库或消息队列                              |

### 2.2 部署方式

#### 方式一: 作为 Rust crate 依赖

```toml
[dependencies]
rust-tokio-supervisor = "0.1"
```

#### 方式二: 独立二进制 (通过 demo 示例)

```bash
cargo run --example demo -- --config examples/config/supervisor.yaml
```

### 2.3 文件布局

```text
/etc/supervisor/
└── supervisor.yaml                # 主配置

/run/rust-supervisor/              # IPC socket 目录
├── target-1.sock                  # 目标进程 IPC socket
└── dashboard-relay-registration.sock  # relay 注册 socket

/var/log/supervisor/               # 日志目录 (由 tracing subscriber 配置)
```

**Socket 权限**: `0600`, 仅进程所有者可读写. Socket 目录应由进程所有者创建.

### 2.4 环境变量

项目不使用环境变量作为配置入口. 所有可调配置通过 YAML 文件加载.

## 三、巡检

### 3.1 健康检查

通过 `SupervisorHandle::health()` 查询运行时健康状态:

```rust
let health = handle.health().await?;
```

`health()` 返回运行时控制平面状态. 配合 `is_alive()` 判断监督器是否存活:

```rust
if !handle.is_alive() {
    // 监督器已退出, 需要重启
}
```

### 3.2 状态查询

通过 `current_state()` 获取完整监督树状态:

```rust
let state = handle.current_state().await?;
// 包含: root path, child 状态, generation, attempt, restart count 等
```

### 3.3 事件流订阅

通过 `subscribe_events()` 订阅生命周期事件:

```rust
let mut rx = handle.subscribe_events();
while let Ok(event) = rx.recv().await {
    // 处理事件
}
```

### 3.4 指标观测

项目通过 `metrics` facade 发送以下指标 (如果 `metrics_enabled: true`):

| 指标名                                   | 类型      | label                    | 说明           |
| ---------------------------------------- | --------- | ------------------------ | -------------- |
| `supervisor_restart_total`               | counter   | path, child_id, decision | 重启总次数     |
| `supervisor_child_state`                 | gauge     | path, child_id, state    | 子任务当前状态 |
| `supervisor_child_uptime_seconds`        | histogram | path, child_id           | 子任务运行时长 |
| `supervisor_backoff_seconds`             | histogram | path, child_id           | 退避延迟分布   |
| `supervisor_healthcheck_latency_seconds` | histogram | path, child_id           | 健康检查延迟   |
| `supervisor_meltdown_total`              | counter   | path, scope              | 熔断触发次数   |
| `supervisor_shutdown_duration_seconds`   | histogram | path                     | 关闭耗时       |
| `supervisor_event_lag_total`             | counter   | path                     | 事件滞后计数   |
| `supervisor_config_version`              | gauge     | path                     | 配置版本号     |

### 3.5 日志字段

结构化日志字段前缀: `rust_supervisor::dashboard` (target side), `rust_supervisor_relay` (relay side), `rust_supervisor_ui` (UI side).

### 3.6 事件日志

通过 `EventJournal` 环形缓冲区查看最近生命周期事件:

```rust
let events = journal.events(); // 返回固定容量的事件列表
```

事件载荷包含: `when` (时间), `where` (位置), `what` (内容), `sequence` (序号), `correlation_id` (关联标识).

### 3.7 重启预算诊断 (Restart Budget Diagnostics)

当配置了 restart budget(重启预算) 时, 通过以下信号区分不同状态:

| 状态       | 观测指标                                                          | 典型值           |
| ---------- | ----------------------------------------------------------------- | ---------------- |
| 预算正常   | `BudgetExhausted` 事件率为 0, `supervisor_restart_total` 正常递增 | 事件率 = 0       |
| 预算限流中 | `BudgetExhausted` 事件频繁出现, `retry_after_ns` 指明等待时长     | 事件率 > 0       |
| 预算过紧   | `BudgetExhausted` 事件率 > 10 次/分钟, 合法 child 重启也被拒      | 触发告警阈值     |
| 预算恢复中 | 故障停止后 `BudgetExhausted` 事件率逐渐归零, 令牌逐步恢复         | 事件率递减       |
| 公平性饥饿 | `StarvationAlert` 事件出现, 某 child 连续被跳过调度               | probe 窗口 > 10s |

诊断步骤:

1. 观察 `BudgetExhausted` 事件率: 若 > 10 次/分钟, 检查 `max_burst` 和 `recovery_rate_per_sec` 配置是否过严
2. 检查 `retry_after_ns` 字段: 确认等待时长是否可接受
3. 对比事件中的 `correlation_id` 与后续 `RestartAfter` 事件: 确认预算链路完整
4. 配合 `supervisor_backoff_seconds` 直方图: 区分"budget 限流中"与"backoff 等待中"——budget 限流时事件率为正且无 backoff 延迟, backoff 等待时有 backoff 延迟指标

## 四、故障处理

### 4.1 重启风暴

**现象**: 子任务在短时间内反复快速崩溃, 重启次数激增.

**处理步骤**:

1. 检查 `supervisor_meltdown_total` 指标是否触发熔断
2. 检查 `supervisor_restart_total` 与 budget 阈值对比
3. 查看 `RestartBudgetTracker` 是否已耗尽
4. 如熔断触发, 等待 `retry_after_ns` 到期后自动恢复
5. 如需紧急止血, 调用 `quarantine_child` 隔离故障子任务

```rust
handle.quarantine_child("child-1", "operator", "restart storm").await?;
```

### 4.2 关闭卡住

**现象**: `shutdown_tree` 调用超时未返回.

**处理步骤**:

1. 检查 `shutdown_duration_seconds` 指标
2. 检查每个 child 的 `graceful_timeout` 配置
3. 确认 `BlockingWorker` 的不可立即中止性质
4. 检查日志中的 abort stragglers 阶段
5. 如进程级卡住, 使用系统信号 SIGTERM 发送给宿主进程

**关闭阶段**:

| 阶段             | 超时配置              | 说明                                |
| ---------------- | --------------------- | ----------------------------------- |
| request stop     | —                     | 立即, 传播 CancellationToken        |
| graceful drain   | `graceful_timeout_ms` | 等待 child 自行退出                 |
| abort stragglers | `abort_wait_ms`       | 强制中止 AsyncWorker                |
| reconcile        | —                     | 更新 registry/state/metrics/journal |

### 4.3 子任务失败

**现象**: 子任务不断失败并在 backoff 后重启.

**处理步骤**:

1. 查看 `supervisor_child_state` gauge 确认子任务当前状态
2. 订阅事件流, 分析失败原因 (`TaskFailureKind`)
3. 检查 `BackoffPolicy` 的退避序列
4. 如为 `FatalConfig` 或 `FatalBug`, 检查配置或代码
5. 如为 `ExternalDependency`, 检查外部服务健康

### 4.4 IPC 通信故障

**现象**: 目标进程无法通过 IPC socket 连接.

**处理步骤**:

1. 确认 socket 文件存在: `ls -la /run/rust-supervisor/*.sock`
2. 确认权限: `stat -c %a /run/rust-supervisor/*.sock` (应为 600)
3. 确认进程 UID 与 socket owner 一致
4. 查看 `IpcSecurityPipeline` 日志 (target: `rust_supervisor::ipc::security`)
5. 检查 rate limit 是否触发 (C6)
6. 检查 relay 注册心跳是否正常

### 4.5 P1 事故响应速查

| 现象       | 初始诊断                 | 止血操作                | 恢复操作              | 参考文档 |
| ---------- | ------------------------ | ----------------------- | --------------------- | -------- |
| 重启风暴   | `meltdown_total` 激增    | `quarantine_child` 隔离 | 等待 retry_after 到期 | 4.1 节   |
| 关闭卡住   | `shutdown_duration` 超时 | 进程级 SIGTERM          | 检查 ChildSlot 状态   | 4.2 节   |
| 子任务崩溃 | `restart_total` 上升     | `remove_child`          | 修复业务代码          | 4.3 节   |
| IPC 不可达 | socket 文件缺失          | 检查进程存活            | 重启 supervisor 进程  | 4.4 节   |
| 预算耗尽   | `BudgetExhausted` 事件   | 可选: 调整 budget 配置  | 等待 recovery 窗口    | 4.1 节   |

## 五、备份与恢复

- 运行时配置: 通过 YAML 文件版本控制管理, 不依赖运行时备份
- 事件日志: `EventJournal` 是内存环形缓冲区, 进程重启后丢失
- 审计记录: 默认内存存储, 如需持久化配置 `audit_persistence=file`
- 状态恢复: 所有 child 在 supervisor 重启后按 `SupervisorSpec` 重新声明和启动

## 六、性能调优

### 6.1 关键配置

| 参数                      | 推荐值 | 说明                             |
| ------------------------- | ------ | -------------------------------- |
| `event_journal_capacity`  | 256    | 事件日志容量, 根据诊断需求调整   |
| `child_restart_window_ms` | 60000  | 重启统计窗口, 敏感任务可调大     |
| `initial_backoff_ms`      | 100    | 首次退避, 频繁重启任务可适当增加 |
| `jitter_ratio`            | 0.10   | 抖动比, 避免惊群                 |
| `graceful_timeout_ms`     | 5000   | 关闭超时, 根据业务清理时间调整   |

### 6.2 性能指标

- 单次 `try_consume()` 调用延迟: p99 < 10µs
- 完整 `evaluate_budget` 阶段: p99 < 100µs
- 不影响控制循环主路径延迟

## 七、相关文档

- [配置模板](../examples/config/supervisor.template.yaml)
- [架构 - 数据流](architecture.md#二核心数据流)
- [安全说明](security.md)
- [质量门禁](en/quality-gates.md)
- [产品路线图](product-roadmap.md)
