use std::time::Duration;

use super::ReconnectBackoff;
use tokio::task::yield_now;
use tokio::time::sleep;

impl ReconnectBackoff {
    /// 指数增长时允许使用的最大指数。
    ///
    /// 业务作用：限制等待时间的增长速度，避免极端故障下出现过长尾部延迟。
    const MAX_EXPONENT: u32 = 8;

    /// 创建包含“首次即时重试”策略的退避器。
    ///
    /// 用于流式任务重连场景：
    /// - 第一次返回 `0ms`，尽快恢复。
    /// - 后续按 `base_delay * 2^n` 退避（`n` 从 1 开始）。
    ///
    /// # 示例
    /// ```rust,ignore
    /// use std::time::Duration;
    /// use crate::helpers::backoff::ReconnectBackoff;
    ///
    /// let mut backoff = ReconnectBackoff::new(
    ///     Duration::from_millis(50),
    ///     Duration::from_millis(500),
    /// );
    ///
    /// assert_eq!(backoff.next_delay(), Duration::ZERO);
    /// assert_eq!(backoff.next_delay(), Duration::from_millis(50));
    /// ```
    pub const fn new(base_delay: Duration, max_delay: Duration) -> Self {
        Self {
            base_delay,
            max_delay,
            consecutive_failures: 0,
            immediate_first_retry: true,
        }
    }

    /// 创建不包含“首次即时重试”的退避器。
    ///
    /// 用于任务监督场景：每次重启都有明确等待时间，第一等待为 `base_delay * 2`。
    ///
    /// # 示例
    /// ```rust,ignore
    /// use std::time::Duration;
    /// use crate::helpers::backoff::ReconnectBackoff;
    ///
    /// let mut backoff = ReconnectBackoff::new_without_immediate_retry(
    ///     Duration::from_millis(100),
    ///     Duration::from_secs(1),
    /// );
    ///
    /// assert_eq!(backoff.next_delay(), Duration::from_millis(200));
    /// ```
    pub const fn new_without_immediate_retry(base_delay: Duration, max_delay: Duration) -> Self {
        Self {
            base_delay,
            max_delay,
            consecutive_failures: 0,
            immediate_first_retry: false,
        }
    }

    /// 重置退避计数器。
    ///
    /// 业务作用：当连接或任务已经恢复稳定后，下一次失败应重新按“首轮重试”处理，
    /// 而不是沿用此前累积的长退避。
    pub fn reset(&mut self) {
        self.consecutive_failures = 0;
    }

    /// 读取下一次重试延迟并推进计数器。
    ///
    /// 返回规则：
    /// - 即时重试模式下，第 1 次返回 `0ms`；
    /// - 非即时重试模式下，第 1 次返回 `base_delay * 2`；
    /// - 之后继续按指数增长，直到被 `max_delay` 裁剪。
    ///
    /// # 示例
    /// ```rust,ignore
    /// use std::time::Duration;
    /// use crate::helpers::backoff::ReconnectBackoff;
    ///
    /// let mut backoff = ReconnectBackoff::new(
    ///     Duration::from_millis(100),
    ///     Duration::from_millis(350),
    /// );
    ///
    /// assert_eq!(backoff.next_delay(), Duration::ZERO);
    /// assert_eq!(backoff.next_delay(), Duration::from_millis(100));
    /// assert_eq!(backoff.next_delay(), Duration::from_millis(200));
    /// assert_eq!(backoff.next_delay(), Duration::from_millis(350));
    /// ```
    pub fn next_delay(&mut self) -> Duration {
        let failures = self.consecutive_failures;
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);

        if self.immediate_first_retry && failures == 0 {
            return Duration::ZERO;
        }

        let exponent = if self.immediate_first_retry {
            failures.saturating_sub(1)
        } else {
            failures.saturating_add(1)
        }
        .min(Self::MAX_EXPONENT);

        self.base_delay
            .saturating_mul(1_u32 << exponent)
            .min(self.max_delay)
    }

    /// 预览下一次等待时长，但不推进内部计数器。
    pub fn preview_next_delay(&self) -> Duration {
        let mut copy = *self;
        copy.next_delay()
    }

    /// 返回自上次 `reset` 以来已经累计的连续失败次数。
    pub const fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures
    }

    /// 按 `next_delay` 等待对应时间。
    ///
    /// 若本次延迟为 `0ms`，内部会调用 `yield_now` 让出调度权，
    /// 既保留“立即重试”的语义，也避免长时间占用当前执行器。
    pub async fn wait(&mut self) -> Duration {
        let delay = self.next_delay();
        if delay.is_zero() {
            yield_now().await;
        } else {
            sleep(delay).await;
        }
        delay
    }
}

#[cfg(test)]
#[path = "../tests/backoff_tests.rs"]
mod tests;
