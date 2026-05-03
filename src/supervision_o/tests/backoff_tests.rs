//! 测试说明：
//! - 验证两种初始化模式的首轮时序；
//! - 验证指数退避增长与上限裁剪；
//! - 验证恢复成功后的 `reset` 语义。

use super::ReconnectBackoff;
use std::time::Duration;

/// 包含即时重试时，第一轮应立即返回 `0ms`。
#[test]
fn backoff_starts_with_immediate_retry() {
    let mut backoff = ReconnectBackoff::new(Duration::from_millis(100), Duration::from_secs(1));
    assert_eq!(backoff.next_delay(), Duration::ZERO);
    assert_eq!(backoff.next_delay(), Duration::from_millis(100));
}

/// 不包含即时重试时，第一轮应按 `base * 2` 开始。
#[test]
fn backoff_starts_without_immediate_retry() {
    let mut backoff = ReconnectBackoff::new_without_immediate_retry(
        Duration::from_millis(100),
        Duration::from_secs(1),
    );
    assert_eq!(backoff.next_delay(), Duration::from_millis(200));
    assert_eq!(backoff.next_delay(), Duration::from_millis(400));
}

/// 指数增长并会被 `max_delay` 裁剪。
#[test]
fn backoff_grows_and_caps() {
    let mut backoff = ReconnectBackoff::new(Duration::from_millis(100), Duration::from_millis(350));
    assert_eq!(backoff.next_delay(), Duration::ZERO);
    assert_eq!(backoff.next_delay(), Duration::from_millis(100));
    assert_eq!(backoff.next_delay(), Duration::from_millis(200));
    assert_eq!(backoff.next_delay(), Duration::from_millis(350));
    assert_eq!(backoff.next_delay(), Duration::from_millis(350));
}

/// `reset` 后应恢复为初始状态。
#[test]
fn backoff_reset_restores_initial_state() {
    let mut backoff = ReconnectBackoff::new(Duration::from_millis(100), Duration::from_secs(1));
    let _ = backoff.next_delay();
    let _ = backoff.next_delay();
    let _ = backoff.next_delay();
    backoff.reset();
    assert_eq!(backoff.next_delay(), Duration::ZERO);
}

/// `preview_next_delay` 只预览，不应推进失败计数器。
#[test]
fn preview_next_delay_does_not_advance_failures() {
    let mut backoff = ReconnectBackoff::new(Duration::from_millis(100), Duration::from_secs(1));

    assert_eq!(backoff.consecutive_failures(), 0);
    assert_eq!(backoff.preview_next_delay(), Duration::ZERO);
    assert_eq!(backoff.consecutive_failures(), 0);

    assert_eq!(backoff.next_delay(), Duration::ZERO);
    assert_eq!(backoff.consecutive_failures(), 1);

    assert_eq!(backoff.preview_next_delay(), Duration::from_millis(100));
    assert_eq!(backoff.consecutive_failures(), 1);
    assert_eq!(backoff.next_delay(), Duration::from_millis(100));
}
