//! Shutdown budget and tree policy declarations.
//!
//! [`ShutdownBudget`] describes cooperative stop windows for one child.
//! [`TreeShutdownPolicy`] extends that budget with supervisor-tree execution
//! knobs used by the four-stage shutdown pipeline.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Recommended extra grace before the tree shutdown global hard deadline.
pub const DEFAULT_FORCE_KILL_MARGIN_SECS: u64 = 5;

/// Cooperative stop timing budget for one child.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ShutdownBudget {
    /// Graceful stop budget for cooperative shutdown.
    pub graceful_timeout: Duration,
    /// Wait budget after an abort request.
    pub abort_wait: Duration,
}

impl ShutdownBudget {
    /// Creates a shutdown budget.
    ///
    /// # Arguments
    ///
    /// - `graceful_timeout`: Cooperative shutdown budget.
    /// - `abort_wait`: Wait budget after abort escalation.
    ///
    /// # Returns
    ///
    /// Returns a [`ShutdownBudget`] value.
    ///
    /// # Examples
    ///
    /// ```
    /// let budget = rust_supervisor::spec::shutdown::ShutdownBudget::new(
    ///     std::time::Duration::from_secs(1),
    ///     std::time::Duration::from_millis(100),
    /// );
    /// assert_eq!(budget.graceful_timeout.as_secs(), 1);
    /// ```
    pub fn new(graceful_timeout: Duration, abort_wait: Duration) -> Self {
        Self {
            graceful_timeout,
            abort_wait,
        }
    }
}

impl Default for ShutdownBudget {
    /// Returns the recommended child shutdown budget.
    fn default() -> Self {
        Self::new(Duration::from_secs(5), Duration::from_secs(1))
    }
}

/// Shutdown timing policy for a supervisor tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TreeShutdownPolicy {
    /// Default cooperative stop budget inherited by children without an override.
    pub budget: ShutdownBudget,
    /// Whether asynchronous stragglers may be aborted after the timeout.
    pub abort_after_timeout: bool,
    /// Extra grace beyond the tree budget before the global hard deadline.
    pub force_kill_margin: Duration,
    /// Maximum orphaned child tasks before controlled process exit.
    ///
    /// When set to 0, the threshold is computed automatically from worker count.
    pub max_orphan_threshold: u32,
}

impl TreeShutdownPolicy {
    /// Creates a tree shutdown policy.
    ///
    /// # Arguments
    ///
    /// - `budget`: Default cooperative stop budget for the tree.
    /// - `abort_after_timeout`: Whether async stragglers may be aborted.
    /// - `force_kill_margin`: Extra grace before the global hard deadline.
    /// - `max_orphan_threshold`: Max orphan count before process exit.
    ///   Pass 0 to compute automatically.
    ///
    /// # Returns
    ///
    /// Returns a [`TreeShutdownPolicy`].
    pub fn new(
        budget: ShutdownBudget,
        abort_after_timeout: bool,
        force_kill_margin: Duration,
        max_orphan_threshold: u32,
    ) -> Self {
        Self {
            budget,
            abort_after_timeout,
            force_kill_margin,
            max_orphan_threshold,
        }
    }

    /// Creates a tree policy from one budget and recommended tree defaults.
    ///
    /// # Arguments
    ///
    /// - `budget`: Default cooperative stop budget for the tree.
    ///
    /// # Returns
    ///
    /// Returns a [`TreeShutdownPolicy`] with abort enabled and automatic orphan threshold.
    pub fn with_budget(budget: ShutdownBudget) -> Self {
        Self::new(
            budget,
            true,
            Duration::from_secs(DEFAULT_FORCE_KILL_MARGIN_SECS),
            0,
        )
    }
}

impl Default for TreeShutdownPolicy {
    /// Returns the recommended tree shutdown policy.
    fn default() -> Self {
        Self::with_budget(ShutdownBudget::default())
    }
}
