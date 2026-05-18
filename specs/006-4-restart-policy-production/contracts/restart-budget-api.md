# Contract(契约): RestartBudget(重启预算) API

**Feature(功能)**: `006-4-restart-policy-production`

## RestartBudgetConfig

````rust
/// Configuration for restart budget tracking.
pub struct RestartBudgetConfig {
    /// Sliding window duration for failure counting.
    pub window: Duration,
    /// Maximum burst failures allowed within the window.
    pub max_burst: u32,
    /// Token recovery rate per second (0.0 = no recovery).
    pub recovery_rate_per_sec: f64,
}

impl RestartBudgetConfig {
    /// Creates a restart budget configuration.
    ///
    /// # Arguments
    ///
    /// - `window`: Sliding window for failure counting.
    /// - `max_burst`: Maximum burst failures in the window.
    /// - `recovery_rate_per_sec`: Tokens recovered per second.
    ///
    /// # Returns
    ///
    /// Returns a [`RestartBudgetConfig`].
    pub fn new(window: Duration, max_burst: u32, recovery_rate_per_sec: f64) -> Self;
}

/// # Budget Curve(预算曲线) 计算公式
///
/// Effective restart attempts per minute 上界由令牌桶容量和恢复速率共同决定:
///
/// ```text
/// max_effective_rpm = (max_tokens / window_seconds + recovery_rate_per_sec) * 60
/// ```
///
/// 示例: `window = 60s`, `max_burst = 10`, `recovery_rate_per_sec = 0.5`
/// → max_effective_rpm = (10/60 + 0.5) × 60 = 10 + 30 = 40 RPM
///
/// SC-001 要求的 105% 上界即 `max_effective_rpm * 1.05`.
````

## RestartBudgetTracker

```rust
/// Mutable restart budget tracker with sliding window + token bucket.
pub struct RestartBudgetTracker { /* fields private */ }

impl RestartBudgetTracker {
    /// Creates a tracker with full token capacity.
    pub fn new(config: RestartBudgetConfig, now_unix_nanos: u128) -> Self;

    /// Attempts to consume one token for a restart.
    ///
    /// Refills tokens based on elapsed time before checking availability.
    ///
    /// # Arguments
    ///
    /// - `now_unix_nanos`: Current Unix timestamp in nanoseconds.
    ///
    /// # Returns
    ///
    /// Returns [`BudgetVerdict::Granted`] when a token is available,
    /// or [`BudgetVerdict::Exhausted`] with the retry‑after duration.
    pub fn try_consume(&mut self, now_unix_nanos: u128) -> BudgetVerdict;

    /// Returns the current token count (for diagnostics).
    pub fn current_tokens(&self, now_unix_nanos: u128) -> f64;

    /// Returns the number of failures currently in the sliding window.
    pub fn window_failures(&self, now_unix_nanos: u128) -> u32;
}
```

## BudgetVerdict

```rust
/// Outcome of a budget consumption attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetVerdict {
    /// Budget granted, restart may proceed.
    Granted,
    /// Budget exhausted, restart must wait.
    Exhausted {
        /// Nanoseconds to wait before retrying.
        retry_after_ns: u128,
    },
}
```

## 不变式

1. `tokens` 不得低于 `0.0`, 不得高于 `max_burst as f64`
2. 每次 `try_consume` 调用必须先驱逐过期故障再检查令牌
3. `failures` 队列中的时间戳必须在 `[now - window, now]` 范围内
4. 成功重启不计入 `failures` 队列, 仅故障计入

## 与其他组件的交互

```
control_loop.rs
  └─ SupervisionPipeline::evaluate_budget()
       ├─ RestartBudgetTracker::try_consume(now_unix_nanos)
       │    ├─ Granted → 继续 backoff 计算
       │    └─ Exhausted { retry_after } → 拒绝重启, 发射 BudgetExhausted 事件
       └─ MeltdownTracker::track() → 若 budget 耗尽 → 递增熔断计数器
```
