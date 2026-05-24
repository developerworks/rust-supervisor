//! File descriptor leak monitoring.
//!
//! Reads `/proc/self/fd` to count open file descriptors. The supervisor can
//! call [`check_fd_count`] periodically to detect monotonic FD growth, which
//! may indicate orphaned tasks whose `Drop` never ran.

use std::path::Path;

/// Threshold ratio: if the current FD count exceeds
/// `baseline * (1 + FD_GROWTH_TOLERANCE)`, a warning is emitted.
const FD_GROWTH_TOLERANCE: f64 = 0.30;

/// Counts the number of file descriptors currently open by the process.
///
/// On Linux and macOS, reads the number of entries in `/proc/self/fd`
/// (or `/dev/fd` on macOS). Returns `None` when the platform does not
/// support this check.
///
/// # Arguments
///
/// This function has no arguments.
///
/// # Returns
///
/// Returns `Some(count)` when the FD count is available, or `None` when
/// the platform does not expose `/proc/self/fd`.
pub fn count_open_fds() -> Option<u64> {
    let fd_dir = if cfg!(target_os = "macos") {
        Path::new("/dev/fd")
    } else {
        Path::new("/proc/self/fd")
    };

    let entries = std::fs::read_dir(fd_dir).ok()?;
    let count = entries.filter_map(|e| e.ok()).count() as u64;
    Some(count)
}

/// Result of an FD growth check.
#[derive(Debug, Clone)]
pub struct FdWatchResult {
    /// Current number of open file descriptors.
    pub current: u64,
    /// Baseline FD count established at process start.
    pub baseline: u64,
    /// Whether the FD count has grown beyond the tolerable threshold.
    pub growth_detected: bool,
    /// Human-readable diagnostic message.
    pub message: String,
}

/// Checks the current FD count against an established baseline.
///
/// Returns `FdWatchResult::growth_detected = true` when the current count
/// exceeds `baseline * (1 + FD_GROWTH_TOLERANCE)`, which may indicate that
/// orphaned child tasks are leaking file descriptors.
///
/// # Arguments
///
/// - `baseline`: FD count captured at process start (or after a known clean
///   state). Pass `None` to skip the growth check.
///
/// # Returns
///
/// Returns an [`FdWatchResult`].
pub fn check_fd_count(baseline: Option<u64>) -> FdWatchResult {
    let current = count_open_fds().unwrap_or(0);
    let baseline = baseline.unwrap_or(current);

    let growth_ratio = if baseline > 0 {
        (current as f64 - baseline as f64) / baseline as f64
    } else {
        // When baseline is 0 (unlikely but handle gracefully), use absolute
        // check: warn above FD_GROWTH_TOLERANCE * 100 as a rough threshold.
        if current > 10 { 1.0 } else { 0.0 }
    };

    let growth_detected = growth_ratio > FD_GROWTH_TOLERANCE;
    let message = if growth_detected {
        format!(
            "FD growth detected: current={} baseline={} ratio={:.2} (>{:.0}%)",
            current,
            baseline,
            growth_ratio,
            FD_GROWTH_TOLERANCE * 100.0,
        )
    } else {
        format!("FD count normal: current={current} baseline={baseline} ratio={growth_ratio:.2}",)
    };

    FdWatchResult {
        current,
        baseline,
        growth_detected,
        message,
    }
}

#[cfg(test)]
mod tests {
    use crate::health::fd_watch::{check_fd_count, count_open_fds};

    /// Returns the current FD count on this platform.
    #[test]
    fn count_open_fds_returns_some() {
        // Every process has at least stdin (0), stdout (1), stderr (2).
        let count = count_open_fds();
        assert!(count.is_some(), "FD counting should work on this platform");
        assert!(count.unwrap() >= 3, "at least stdin/out/err");
    }

    /// Verifies that `check_fd_count` without a baseline returns the
    /// current FD count and does not flag growth.
    /// Verifies that `check_fd_count` without a baseline returns the
    /// current FD count and does not flag growth.
    #[test]
    fn check_fd_no_baseline_returns_current() {
        let result = check_fd_count(None);
        assert!(result.current >= 3);
        assert_eq!(result.baseline, result.current);
        assert!(!result.growth_detected);
    }

    /// Verifies that an artificially low baseline triggers the growth
    /// warning in `check_fd_count`.
    #[test]
    fn check_fd_growth_detected() {
        // Artificially set a very low baseline to trigger the warning.
        let result = check_fd_count(Some(1));
        // On any real system the FD count should exceed 1, so growth
        // should be detected.
        assert!(
            result.growth_detected || result.current <= 1,
            "baseline=1 should trigger growth: {:?}",
            result.message,
        );
    }

    /// Verifies that `check_fd_count` with a matching baseline does
    /// not flag growth.
    #[test]
    fn check_fd_no_growth() {
        let current = count_open_fds().unwrap_or(100);
        let result = check_fd_count(Some(current));
        assert!(!result.growth_detected, "same baseline should match");
    }
}
