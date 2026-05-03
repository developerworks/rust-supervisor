//! 任务监督模块固定参数。
//!
//! 这些常量只服务于 [`super::task_supervision`] 的重启策略构造，
//! 不直接对外暴露，避免外部依赖文件布局。

use std::time::Duration;

/// 关键任务默认最大重启次数。
pub(super) const TASK_CRITICAL_MAX_RESTARTS: u32 = 5;
/// 降级任务默认最大重启次数。
pub(super) const TASK_DEGRADED_MAX_RESTARTS: u32 = 2;
/// 统一的重启基础退避。
pub(super) const TASK_RESTART_BASE_DELAY: Duration = Duration::from_secs(1);
/// 指数退避的最大指数上限。
pub(super) const TASK_RESTART_EXPONENT_CAP: u32 = 8;
