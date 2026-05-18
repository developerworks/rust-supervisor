# Quickstart(快速开始): 类型化事件与端到端可追溯闭环

**Branch(分支)**: `006-5-typed-events-observability` | **Date(日期)**: 2026-05-18

## Overview(概述)

本切片将控制循环中的字符串事件替换为类型化的 `SupervisorEvent` 枚举, 接入 correlation id(关联标识) 端到端追踪, 并实现 event subscriber(事件订阅者) 慢消费时的背压处理.

## 现有代码入口

| 文件 | 说明 |
|------|------|
| `src/event/payload.rs` | `What` 枚举(30+ 变体) 和 `SupervisorEvent` 结构体 |
| `src/event/time.rs` | `CorrelationId`, `EventSequence`, `EventTime` |
| `src/observe/pipeline.rs` | `ObservabilityPipeline` 事件扇出 |
| `src/journal/ring.rs` | 环形缓冲区事件日志 |
| `src/runtime/control_loop.rs` | 控制循环主逻辑 |

## 新增代码预期位置

| 文件 | 说明 |
|------|------|
| `src/event/correlation.rs` | `CorrelationHandle` 关联句柄 |
| `src/event/payload.rs` | 新增 10 个 `What` 变体 |
| `src/observe/pipeline.rs` | 背压检测与降级采样 |
| `src/spec/supervisor.rs` | `BackpressureConfig`, `BackpressureStrategy` |
| `tests/typed_event_coverage_test.rs` | 穷尽 What 枚举变体冒烟 |
| `tests/correlation_tracking_test.rs` | correlation id 5 段覆盖 |
| `tests/backpressure_strategy_test.rs` | 背压告警与采样降级 |

## 配置

在 `SupervisorSpec` 中新增事件相关配置:

```yaml
event:
  backpressure_strategy: alert_and_block  # 或 sample_and_audit
  backpressure_warn_threshold_pct: 80
  backpressure_critical_threshold_pct: 95
  backpressure_window_secs: 30
  audit_channel_capacity: 1024
```

## 验证

```bash
# 全量测试
cargo test

# 独立测试
cargo test --test typed_event_coverage_test
cargo test --test correlation_tracking_test
cargo test --test backpressure_strategy_test

# 文档检查
cargo doc --no-deps --document-private-items
```
