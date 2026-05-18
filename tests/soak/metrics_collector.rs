//! Metrics collector for soak tests.
//!
//! Collects p99 latency, RSS (platform-dependent), FD count, and
//! event gap metrics during the soak window. Platform-conditional
//! compilation handles Linux vs macOS RSS API differences.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// A single metrics snapshot.
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    /// Timestamp relative to soak start.
    pub elapsed_secs: u64,
    /// p99 emit latency in milliseconds (1s sliding window).
    pub p99_latency_ms: f64,
    /// RSS in megabytes (None if platform unsupported).
    pub rss_mb: Option<f64>,
    /// Open file descriptor count.
    pub fd_count: u64,
    /// Event gap total (journal entries vs emit count difference).
    pub event_gap_total: u64,
}

/// Collects metrics during the soak window.
#[derive(Debug)]
pub struct MetricsCollector {
    /// Collected snapshots.
    pub snapshots: Vec<MetricsSnapshot>,
    /// Latency samples for the current 1s sliding window.
    latency_window: VecDeque<f64>,
    /// Start time of the soak.
    start: Instant,
    /// Last RSS collection time.
    last_rss_collection: Instant,
    /// Last FD count collection time.
    last_fd_collection: Instant,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsCollector {
    /// Creates a new metrics collector.
    pub fn new() -> Self {
        Self {
            snapshots: Vec::new(),
            latency_window: VecDeque::with_capacity(1000),
            start: Instant::now(),
            last_rss_collection: Instant::now(),
            last_fd_collection: Instant::now(),
        }
    }

    /// Records a latency sample (called every control loop emit).
    pub fn record_latency(&mut self, latency_ms: f64) {
        self.latency_window.push_back(latency_ms);
        // Keep only the last 1000 samples (~1s at 1000 req/s).
        if self.latency_window.len() > 1000 {
            self.latency_window.pop_front();
        }
    }

    /// Takes a full metrics snapshot. Call this every second.
    pub fn snapshot(&mut self) {
        let elapsed = self.start.elapsed().as_secs();

        // Compute p99 from the sliding window.
        let p99 = self.compute_p99();

        // Collect RSS every 60s.
        let rss = if elapsed % 60 == 0 && elapsed != self.last_rss_collection.elapsed().as_secs() {
            self.last_rss_collection = Instant::now();
            Some(Self::read_rss_mb())
        } else {
            None
        };

        // Collect FD count every 60s.
        let fd = if elapsed % 60 == 0 && elapsed != self.last_fd_collection.elapsed().as_secs() {
            self.last_fd_collection = Instant::now();
            Some(Self::read_fd_count())
        } else {
            None
        };

        self.snapshots.push(MetricsSnapshot {
            elapsed_secs: elapsed,
            p99_latency_ms: p99,
            rss_mb: rss,
            fd_count: fd.unwrap_or(0),
            event_gap_total: 0,
        });
    }

    /// Computes p99 from the current latency window.
    fn compute_p99(&self) -> f64 {
        if self.latency_window.is_empty() {
            return 0.0;
        }
        let mut sorted: Vec<f64> = self.latency_window.iter().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let idx = ((sorted.len() as f64) * 0.99).ceil() as usize - 1;
        let idx = idx.min(sorted.len() - 1);
        sorted[idx]
    }

    /// Reads RSS in megabytes.
    ///
    /// Platform-specific:
    /// - Linux: reads `/proc/self/status` for VmRSS.
    /// - macOS: uses `libc::proc_pidinfo` for resident size.
    #[cfg(target_os = "linux")]
    fn read_rss_mb() -> f64 {
        let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                // Format: "VmRSS:    12345 kB"
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(kb_str) = parts.get(1) {
                    if let Ok(kb) = kb_str.parse::<f64>() {
                        return kb / 1024.0;
                    }
                }
            }
        }
        0.0
    }

    /// Reads RSS in megabytes on macOS.
    #[cfg(target_os = "macos")]
    fn read_rss_mb() -> f64 {
        // Use libc::proc_pidInfo to get resident size.
        let pid = unsafe { libc::getpid() };
        let mut task_info = std::mem::MaybeUninit::<libc::proc_taskinfo>::uninit();
        let size = unsafe {
            libc::proc_pidinfo(
                pid,
                libc::PROC_PIDTASKINFO,
                0,
                task_info.as_mut_ptr() as *mut libc::c_void,
                std::mem::size_of::<libc::proc_taskinfo>() as i32,
            )
        };
        if size > 0 {
            let info = unsafe { task_info.assume_init() };
            // pti_resident_size is in bytes, convert to MB.
            info.pti_resident_size as f64 / (1024.0 * 1024.0)
        } else {
            0.0
        }
    }

    /// Fallback for unsupported platforms.
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    fn read_rss_mb() -> f64 {
        0.0
    }

    /// Reads the current file descriptor count.
    fn read_fd_count() -> u64 {
        #[cfg(target_os = "linux")]
        {
            std::fs::read_dir("/dev/fd")
                .map(|entries| entries.count() as u64)
                .unwrap_or(0)
        }
        #[cfg(not(target_os = "linux"))]
        {
            // On macOS, count open FDs via libc.
            let mut count: u64 = 0;
            for fd in 0..4096 {
                let mut st = std::mem::MaybeUninit::<libc::stat>::uninit();
                let ret = unsafe { libc::fstat(fd, st.as_mut_ptr()) };
                if ret == 0 {
                    count += 1;
                }
            }
            count
        }
    }
}
