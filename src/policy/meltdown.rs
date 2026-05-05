//! Failure-window tracking for meltdown detection.
//!
//! The module tracks child and supervisor failure windows and emits simple
//! outcomes that the runtime can map to quarantine or escalation.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Failure fuse limits for child and supervisor scopes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeltdownPolicy {
    /// Maximum restarts allowed for one child inside the child window.
    pub child_max_restarts: u32,
    /// Window used to count child restarts.
    pub child_window: Duration,
    /// Maximum failures allowed for a supervisor inside the supervisor window.
    pub supervisor_max_failures: u32,
    /// Window used to count supervisor failures.
    pub supervisor_window: Duration,
    /// Stable duration after which recorded counters may be cleared.
    pub reset_after: Duration,
}

impl MeltdownPolicy {
    /// Creates a meltdown policy.
    ///
    /// # Arguments
    ///
    /// - `child_max_restarts`: Restart limit for one child.
    /// - `child_window`: Restart counting window.
    /// - `supervisor_max_failures`: Failure limit for a supervisor.
    /// - `supervisor_window`: Failure counting window.
    /// - `reset_after`: Stable duration that clears counters.
    ///
    /// # Returns
    ///
    /// Returns a [`MeltdownPolicy`].
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    ///
    /// let policy = rust_supervisor::policy::meltdown::MeltdownPolicy::new(
    ///     3,
    ///     Duration::from_secs(10),
    ///     10,
    ///     Duration::from_secs(60),
    ///     Duration::from_secs(120),
    /// );
    /// assert_eq!(policy.child_max_restarts, 3);
    /// ```
    pub fn new(
        child_max_restarts: u32,
        child_window: Duration,
        supervisor_max_failures: u32,
        supervisor_window: Duration,
        reset_after: Duration,
    ) -> Self {
        Self {
            child_max_restarts,
            child_window,
            supervisor_max_failures,
            supervisor_window,
            reset_after,
        }
    }
}

/// Result of recording a failure against meltdown counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MeltdownOutcome {
    /// No fuse fired.
    Continue,
    /// Child-level fuse fired and the child should be quarantined.
    ChildFuse,
    /// Supervisor-level fuse fired and the failure should be escalated.
    SupervisorFuse,
}

/// Mutable meltdown counter state.
#[derive(Debug, Clone)]
pub struct MeltdownTracker {
    /// Policy that defines counter windows and limits.
    pub policy: MeltdownPolicy,
    child_failures: VecDeque<Instant>,
    supervisor_failures: VecDeque<Instant>,
    last_failure: Option<Instant>,
}

impl MeltdownTracker {
    /// Creates an empty tracker for a policy.
    ///
    /// # Arguments
    ///
    /// - `policy`: Limits used by the tracker.
    ///
    /// # Returns
    ///
    /// Returns a [`MeltdownTracker`] with no recorded failures.
    pub fn new(policy: MeltdownPolicy) -> Self {
        Self {
            policy,
            child_failures: VecDeque::new(),
            supervisor_failures: VecDeque::new(),
            last_failure: None,
        }
    }

    /// Records a child restart failure.
    ///
    /// # Arguments
    ///
    /// - `now`: Current monotonic time supplied by the runtime or test.
    ///
    /// # Returns
    ///
    /// Returns a [`MeltdownOutcome`] for the updated counters.
    pub fn record_child_restart(&mut self, now: Instant) -> MeltdownOutcome {
        self.prune(now);
        self.child_failures.push_back(now);
        self.supervisor_failures.push_back(now);
        self.last_failure = Some(now);
        self.current_outcome()
    }

    /// Clears counters after a stable period.
    ///
    /// # Arguments
    ///
    /// - `now`: Current monotonic time supplied by the runtime or test.
    ///
    /// # Returns
    ///
    /// Returns `true` when counters were cleared.
    pub fn reset_if_stable(&mut self, now: Instant) -> bool {
        let Some(last_failure) = self.last_failure else {
            return false;
        };
        if now.duration_since(last_failure) < self.policy.reset_after {
            return false;
        }
        self.clear();
        true
    }

    /// Removes all recorded failures.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// This function returns nothing.
    pub fn clear(&mut self) {
        self.child_failures.clear();
        self.supervisor_failures.clear();
        self.last_failure = None;
    }

    /// Returns the current child failure count.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the number of child failures inside the current window.
    pub fn child_failure_count(&self) -> usize {
        self.child_failures.len()
    }

    /// Removes expired counter entries.
    ///
    /// # Arguments
    ///
    /// - `now`: Current monotonic time.
    ///
    /// # Returns
    ///
    /// This function returns nothing.
    fn prune(&mut self, now: Instant) {
        prune_window(&mut self.child_failures, now, self.policy.child_window);
        prune_window(
            &mut self.supervisor_failures,
            now,
            self.policy.supervisor_window,
        );
    }

    /// Evaluates counters after pruning.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the most severe current outcome.
    fn current_outcome(&self) -> MeltdownOutcome {
        if self.supervisor_failures.len() > self.policy.supervisor_max_failures as usize {
            MeltdownOutcome::SupervisorFuse
        } else if self.child_failures.len() > self.policy.child_max_restarts as usize {
            MeltdownOutcome::ChildFuse
        } else {
            MeltdownOutcome::Continue
        }
    }
}

/// Prunes timestamps outside a time window.
///
/// # Arguments
///
/// - `entries`: Timestamp queue to update.
/// - `now`: Current monotonic time.
/// - `window`: Maximum age to retain.
///
/// # Returns
///
/// This function returns nothing.
fn prune_window(entries: &mut VecDeque<Instant>, now: Instant, window: Duration) {
    while entries
        .front()
        .is_some_and(|entry| now.duration_since(*entry) > window)
    {
        entries.pop_front();
    }
}
