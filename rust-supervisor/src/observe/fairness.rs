//! Fairness probe module.
//!
//! Lightweight probe inserted on the control loop main path
//! that detects scheduling starvation (US1: fairness probe).
//!
//! [`FairnessProbe`] records per-child scheduling opportunities
//! via [`FairnessProbe::record_opportunity`] and periodically checks via
//! [`FairnessProbe::check`] whether any child has been starved (received fewer than
//! `min_ops_per_window` scheduling opportunities within the
//! `probe_interval_ns` window). When starvation is detected,
//! a [`StarvationAlert`] is emitted for diagnostics.

use crate::id::types::ChildId;
use std::collections::HashMap;

/// Lightweight fairness probe inserted on the control loop main path.
///
/// Records scheduling opportunities per child and detects starvation
/// when some children are consistently skipped over a probe window.
#[derive(Debug)]
pub struct FairnessProbe {
    /// Cumulative scheduling opportunity counter.
    scheduling_opportunities: u64,
    /// Per-child scheduling counts.
    per_child_ops: HashMap<ChildId, u64>,
    /// Timestamp of the last probe check (Unix nanos).
    last_probe_unix_nanos: u128,
    /// Probe interval in nanoseconds (default 10 s).
    probe_interval_ns: u128,
    /// Minimum scheduling opportunities each ready child should
    /// receive within a probe window (default 1).
    min_ops_per_window: u64,
}

/// Alert emitted when scheduling starvation is detected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StarvationAlert {
    /// The child that has been starved.
    pub starved_child_id: ChildId,
    /// How many times this child was skipped.
    pub skip_count: u64,
    /// Start of the probe window (Unix nanos).
    pub probe_start_unix_nanos: u128,
    /// End of the probe window (Unix nanos).
    pub probe_end_unix_nanos: u128,
}

impl FairnessProbe {
    /// Creates a fairness probe with default settings.
    ///
    /// # Arguments
    ///
    /// - `now_unix_nanos`: Current Unix timestamp in nanoseconds.
    ///
    /// # Returns
    ///
    /// Returns a [`FairnessProbe`] with `probe_interval_ns = 10 s`
    /// and `min_ops_per_window = 1`.
    pub fn new(now_unix_nanos: u128) -> Self {
        Self {
            scheduling_opportunities: 0,
            per_child_ops: HashMap::new(),
            last_probe_unix_nanos: now_unix_nanos,
            probe_interval_ns: 10_000_000_000,
            min_ops_per_window: 1,
        }
    }

    /// Records one scheduling opportunity for a child.
    pub fn record_opportunity(&mut self, child_id: &ChildId) {
        self.scheduling_opportunities += 1;
        *self.per_child_ops.entry(child_id.clone()).or_insert(0) += 1;
    }

    /// Checks for scheduling starvation across all known children.
    ///
    /// # Arguments
    ///
    /// - `now_unix_nanos`: Current Unix timestamp in nanoseconds.
    /// - `all_child_ids`: All currently ready child identifiers.
    ///
    /// # Returns
    ///
    /// Returns `Some(StarvationAlert)` if any child received fewer than
    /// `min_ops_per_window` opportunities since the last probe,
    /// or `None` if all children received sufficient scheduling.
    pub fn check(
        &mut self,
        now_unix_nanos: u128,
        all_child_ids: &[ChildId],
    ) -> Option<StarvationAlert> {
        let elapsed = now_unix_nanos.saturating_sub(self.last_probe_unix_nanos);
        if elapsed < self.probe_interval_ns {
            return None;
        }

        let probe_start = self.last_probe_unix_nanos;
        self.last_probe_unix_nanos = now_unix_nanos;

        for child_id in all_child_ids {
            let ops = self.per_child_ops.get(child_id).copied().unwrap_or(0);
            if ops < self.min_ops_per_window {
                let alert = StarvationAlert {
                    starved_child_id: child_id.clone(),
                    skip_count: self.min_ops_per_window.saturating_sub(ops),
                    probe_start_unix_nanos: probe_start,
                    probe_end_unix_nanos: now_unix_nanos,
                };
                // Reset counters for the next window.
                self.per_child_ops.clear();
                return Some(alert);
            }
        }

        // Reset counters for the next window.
        self.per_child_ops.clear();
        None
    }
}
