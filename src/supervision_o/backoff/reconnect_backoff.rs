//! 指数退避器状态定义。
//!
//! 该文件只定义重连退避器的状态槽位；
//! 具体延迟推进与重置逻辑位于配套实现文件。

use std::time::Duration;

/// 指数退避重试器。
///
/// 统一封装“基础间隔 + 连续失败次数 + 上限裁剪”三要素，
/// 用于在重复失败时逐步拉长重试等待时间。
#[derive(Debug, Clone, Copy)]
pub struct ReconnectBackoff {
    /// 指数退避的基础延迟。
    pub(super) base_delay: Duration,
    /// 退避等待的硬上限。
    pub(super) max_delay: Duration,
    /// 自上次 `reset` 以来的连续失败次数。
    pub(super) consecutive_failures: u32,
    /// 是否允许第 1 次重试直接返回 `0ms`。
    pub(super) immediate_first_retry: bool,
}
