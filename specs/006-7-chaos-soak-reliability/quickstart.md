# Quickstart(快速开始): 混沌与浸泡测试

## 环境要求

- Rust 1.88+(稳定版), 项目使用 Rust 2024 edition.
- macOS(Apple Silicon) 或 Linux, 16GB+ 内存.
- CI nightly runner 需要至少 30 分钟执行混沌套件, 24h 执行浸泡套件.

## 运行混沌套件

```bash
# 运行全部 11 个混沌场景(CI nightly 使用 --include-ignored)
cargo test --test chaos_suite -- --include-ignored

# 运行单个混沌场景(按名称过滤)
cargo test --test chaos_suite -- --include-ignored child_panic_storm

# 运行混沌套件但不忽略标记(开发调试时)
cargo test --test chaos_suite -- --nocapture
```

**预期产出**: 每个场景在 stdout 输出一条 JSON 判决书. 所有判决书 `passed: true` 时退出码为 0.

**JSON 判决书示例**:

```json
{
  "scenario_id": "child_panic_storm",
  "semver": "0.1.2",
  "passed": true,
  "thresholds": {
    "self_panic_count": { "value": 0, "limit": 0, "passed": true },
    "emit_latency_p99_us": { "value": 42, "limit": 100, "passed": true }
  },
  "started_at_unix_nanos": 1716000000000000000,
  "duration_ns": 60000000000,
  "error": null
}
```

## 运行浸泡套件

```bash
# 运行 24h 浸泡测试(仅在专用 CI runner 上执行)
cargo test --test soak_suite -- --ignored --nocapture

# 运行缩短版浸泡测试(开发验证用, 1h)
SOAK_DURATION_MINUTES=60 cargo test --test soak_suite -- --ignored --nocapture
```

**预期产出**: `artifacts/validation/soak-{YYYYMMDD}-{HHMMSS}.md` 格式的 SoakReport. 报告包含 5 类指标的阈值对照表.

## 故障波形列表

| 场景 ID                    | 说明                                    | 预期执行时间 |
| -------------------------- | --------------------------------------- | ------------ |
| `child_panic_storm`        | 60s 内反复 spawn 并在 1ms 后 panic      | ~65s         |
| `child_block_forever`      | spawn 一个永不返回的 blocking worker    | ~15s         |
| `child_ignore_cancel`      | spawn 后忽略 CancellationToken          | ~15s         |
| `rapid_failure_10k`        | 60s 内 10000 次快速失败 -> 重启 -> 失败 | ~65s         |
| `slow_event_subscriber`    | subscriber 限速 100ms/event             | ~30s         |
| `command_channel_full`     | 填充 mpsc channel 至满                  | ~10s         |
| `ipc_connection_storm`     | 1000 并发劣质 TCP 握手                  | ~15s         |
| `socket_path_contention`   | 占用的 socket 路径上启动 dashboard      | ~5s          |
| `relay_crash_loop`         | relay 进程被 SIGKILL 5 次               | ~20s         |
| `clock_step_backward`      | 模拟时钟回拨 10s                        | ~5s          |
| `runtime_starvation_probe` | tokio yield_now 饥饿 30s                | ~35s         |

## 解读 JSON 判决书

- `scenario_id`: 场景标识, 与上表一一对应.
- `passed`: 整体通过(true)或失败(false).
- `thresholds`: 每个阈值指标的具体数值, 包含 `value`(实际值), `limit`(限制值), `passed`(是否通过).
- `error`: 非 `null` 时表示场景执行过程中发生了意外错误(非阈值越界), 此时 `passed` 应视为 false.

## 故障注入策略概览

所有故障注入通过测试夹具实现, 不修改 `src/` 生产代码. 夹具类型:

- **FixtureChildSpawner**: 可控的 child spawn 夹具, 支持设置 panic 延迟、block 行为、cancel 响应策略.
- **FixtureClockController**: 模拟时间源, 用于时钟回拨场景.
- **FixtureEventThrottle**: 事件订阅者限速夹具, 支持设置 `slow_consumer_ms`.
- **FixtureIpcStress**: IPC 劣质连接生成器, 支持配置并发数和 payload 格式.
- **FixtureRuntimeProbe**: 运行时饥饿探针, 支持注入 `yield_now` 饥饿循环.
