//! Failure-window tracking for meltdown detection.
//!
//! The module tracks child and supervisor failure windows and emits simple
//! outcomes that the runtime can map to quarantine or escalation.

use crate::id::types::ChildId;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

/// Failure fuse limits for child, group, and supervisor scopes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeltdownPolicy {
    /// Maximum restarts allowed for one child inside the child window.
    pub child_max_restarts: u32,
    /// Window used to count child restarts.
    pub child_window: Duration,
    /// Maximum failures allowed for one group inside the group window.
    pub group_max_failures: u32,
    /// Window used to count group failures.
    pub group_window: Duration,
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
    /// - `group_max_failures`: Failure limit for one group.
    /// - `group_window`: Failure counting window for groups.
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
    ///     5,
    ///     Duration::from_secs(30),
    ///     10,
    ///     Duration::from_secs(60),
    ///     Duration::from_secs(120),
    /// );
    /// assert_eq!(policy.child_max_restarts, 3);
    /// ```
    pub fn new(
        child_max_restarts: u32,
        child_window: Duration,
        group_max_failures: u32,
        group_window: Duration,
        supervisor_max_failures: u32,
        supervisor_window: Duration,
        reset_after: Duration,
    ) -> Self {
        Self {
            child_max_restarts,
            child_window,
            group_max_failures,
            group_window,
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
    /// Group-level fuse fired and the group should be isolated.
    GroupFuse,
    /// Supervisor-level fuse fired and the failure should be escalated.
    SupervisorFuse,
}

/// Mutable meltdown counter state with per-scope isolation (FR-002).
#[derive(Debug, Clone)]
pub struct MeltdownTracker {
    /// Policy that defines counter windows and limits.
    pub policy: MeltdownPolicy,
    /// Per-child failure timestamps retained inside the child restart window.
    child_failures: HashMap<ChildId, VecDeque<Instant>>,
    /// Per-group failure timestamps retained inside the group window.
    group_failures: HashMap<String, VecDeque<Instant>>,
    /// Supervisor failure timestamps retained inside the supervisor window.
    supervisor_failures: VecDeque<Instant>,
    /// Latest failure timestamp used for stable-window cleanup.
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
            child_failures: HashMap::new(),
            group_failures: HashMap::new(),
            supervisor_failures: VecDeque::new(),
            last_failure: None,
        }
    }

    #[deprecated(
        since = "0.1.3",
        note = "Use record_child_restart_with_group for per-scope isolation"
    )]
    /// Records a child restart failure (legacy API, aggregates across all scopes).
    ///
    /// # Arguments
    ///
    /// - `now`: Current monotonic time supplied by the runtime or test.
    ///
    /// # Returns
    ///
    /// Returns a [`MeltdownOutcome`] for the updated counters.
    pub fn record_child_restart(&mut self, now: Instant) -> MeltdownOutcome {
        // Legacy behavior: use a synthetic child/group to maintain backward compatibility
        let synthetic_child = ChildId::new("_legacy".to_string());
        self.record_child_restart_with_group(synthetic_child, Some("_legacy".to_string()), now)
    }

    /// Records a child restart failure with explicit group assignment (FR-002).
    ///
    /// Maintains independent state per ChildId, group_id, and supervisor instance.
    /// Returns outcome based on the specific scopes involved in this operation.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Child identifier for per-child tracking.
    /// - `group_id`: Optional group identifier for per-group tracking.
    /// - `now`: Current monotonic time.
    ///
    /// # Returns
    ///
    /// Returns a [`MeltdownOutcome`] based on the most restrictive scope outcome
    /// for the specific child, group, and supervisor involved in this call.
    pub fn record_child_restart_with_group(
        &mut self,
        child_id: ChildId,
        group_id: Option<String>,
        now: Instant,
    ) -> MeltdownOutcome {
        self.prune(now);

        // Record at child level (per-ChildId isolation)
        let child_queue = self.child_failures.entry(child_id.clone()).or_default();
        child_queue.push_back(now);

        // Record at group level (per-group_id isolation) if group is specified
        if let Some(ref gid) = group_id {
            let group_queue = self.group_failures.entry(gid.clone()).or_default();
            group_queue.push_back(now);
        }

        // Record at supervisor level (single queue for all)
        self.supervisor_failures.push_back(now);
        self.last_failure = Some(now);

        // Evaluate outcomes for the specific scopes involved
        self.evaluate_outcome_for_scopes(&child_id, group_id.as_deref())
    }

    /// Evaluates outcome for specific child and group scopes (not global).
    ///
    /// This method checks only the provided child_id and group_id against their thresholds,
    /// plus the global supervisor level. It does NOT check other children or groups.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Child identifier to evaluate.
    /// - `group_id`: Optional group identifier to evaluate.
    ///
    /// # Returns
    ///
    /// Returns the most restrictive outcome for the specified scopes.
    fn evaluate_outcome_for_scopes(
        &self,
        child_id: &ChildId,
        group_id: Option<&str>,
    ) -> MeltdownOutcome {
        // Check supervisor level (global)
        if self.supervisor_failures.len() >= self.policy.supervisor_max_failures as usize {
            return MeltdownOutcome::SupervisorFuse;
        }

        // Check specific group if provided
        if let Some(gid) = group_id {
            let group_count = self.group_failures.get(gid).map_or(0, |q| q.len());
            if group_count >= self.policy.group_max_failures as usize {
                return MeltdownOutcome::GroupFuse;
            }
        }

        // Check specific child
        let child_count = self.child_failures.get(child_id).map_or(0, |q| q.len());
        if child_count >= self.policy.child_max_restarts as usize {
            return MeltdownOutcome::ChildFuse;
        }

        MeltdownOutcome::Continue
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
        self.group_failures.clear();
        self.supervisor_failures.clear();
        self.last_failure = None;
    }

    /// Returns the current child failure count for a specific child.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Child identifier to query.
    ///
    /// # Returns
    ///
    /// Returns the number of child failures inside the current window.
    pub fn child_failure_count(&self, child_id: &ChildId) -> usize {
        self.child_failures.get(child_id).map_or(0, |q| q.len())
    }

    /// Returns the current group failure count for a specific group.
    ///
    /// # Arguments
    ///
    /// - `group_id`: Group identifier to query.
    ///
    /// # Returns
    ///
    /// Returns the number of group failures inside the current window.
    pub fn group_failure_count(&self, group_id: &str) -> usize {
        self.group_failures.get(group_id).map_or(0, |q| q.len())
    }

    /// Removes expired counter entries for all scopes.
    ///
    /// # Arguments
    ///
    /// - `now`: Current monotonic time.
    ///
    /// # Returns
    ///
    /// This function returns nothing.
    fn prune(&mut self, now: Instant) {
        // Prune per-child queues
        for queue in self.child_failures.values_mut() {
            prune_window(queue, now, self.policy.child_window);
        }
        // Remove empty child queues
        self.child_failures.retain(|_, v| !v.is_empty());

        // Prune per-group queues
        for queue in self.group_failures.values_mut() {
            prune_window(queue, now, self.policy.group_window);
        }
        // Remove empty group queues
        self.group_failures.retain(|_, v| !v.is_empty());

        // Prune supervisor queue
        prune_window(
            &mut self.supervisor_failures,
            now,
            self.policy.supervisor_window,
        );
    }

    /// Evaluates outcomes across all scopes after pruning.
    ///
    /// Returns the most restrictive outcome across all tracked children, groups, and supervisor.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the most severe current outcome.
    fn evaluate_outcome(&self) -> MeltdownOutcome {
        // Check supervisor level
        if self.supervisor_failures.len() >= self.policy.supervisor_max_failures as usize {
            return MeltdownOutcome::SupervisorFuse;
        }

        // Check all groups (most restrictive wins)
        for queue in self.group_failures.values() {
            if queue.len() >= self.policy.group_max_failures as usize {
                return MeltdownOutcome::GroupFuse;
            }
        }

        // Check all children (most restrictive wins)
        for queue in self.child_failures.values() {
            if queue.len() >= self.policy.child_max_restarts as usize {
                return MeltdownOutcome::ChildFuse;
            }
        }

        MeltdownOutcome::Continue
    }

    /// Returns the current overall meltdown outcome for testing purposes.
    ///
    /// Evaluates the most restrictive outcome across all tracked scopes.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the current [`MeltdownOutcome`].
    pub fn current_outcome_for_test(&self) -> MeltdownOutcome {
        self.evaluate_outcome()
    }

    /// Returns the current outcome for a specific group (FR-002).
    ///
    /// Queries the per-group failure queue and evaluates against the group threshold.
    ///
    /// # Arguments
    ///
    /// - `group_id`: Group identifier to query.
    ///
    /// # Returns
    ///
    /// Returns the [`MeltdownOutcome`] for the specified group only.
    pub fn get_group_outcome(&self, group_id: &str) -> MeltdownOutcome {
        let count = self.group_failures.get(group_id).map_or(0, |q| q.len());
        if count >= self.policy.group_max_failures as usize {
            MeltdownOutcome::GroupFuse
        } else {
            MeltdownOutcome::Continue
        }
    }

    /// Returns the current supervisor-level outcome.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the [`MeltdownOutcome`] at supervisor level.
    pub fn get_supervisor_outcome(&self) -> MeltdownOutcome {
        if self.supervisor_failures.len() >= self.policy.supervisor_max_failures as usize {
            MeltdownOutcome::SupervisorFuse
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

/// Local verdict for a single meltdown scope layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalVerdict {
    /// Whether this scope layer has triggered its fuse.
    pub triggered: bool,
    /// The meltdown outcome for this layer.
    pub outcome: MeltdownOutcome,
}

/// Result of merging multiple layer verdicts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergedVerdict {
    /// The effective (most restrictive) meltdown verdict.
    pub effective_outcome: MeltdownOutcome,
    /// List of all scopes that triggered.
    pub scopes_triggered: Vec<crate::event::payload::MeltdownScope>,
    /// The dominant attribution scope (tie-break winner).
    pub lead_scope: Option<crate::event::payload::MeltdownScope>,
}

/// Merges local verdicts from child, group, and supervisor layers.
///
/// Takes the most restrictive outcome and applies tie-breaking rules.
///
/// The lead scope is selected only from scopes whose local verdict equals the
/// effective outcome. If multiple matching scopes remain, priority order is
/// child, then group, then supervisor.
///
/// # Arguments
///
/// - `child_verdict`: Local verdict from child layer.
/// - `group_verdict`: Local verdict from group layer.
/// - `supervisor_verdict`: Local verdict from supervisor layer.
///
/// # Returns
///
/// Returns a [`MergedVerdict`] with effective outcome, triggered scopes, and lead scope.
///
/// # Examples
///
/// ```
/// use rust_supervisor::policy::meltdown::{LocalVerdict, MeltdownOutcome, merge_meltdown_verdicts};
/// use rust_supervisor::event::payload::MeltdownScope;
///
/// let child = LocalVerdict { triggered: true, outcome: MeltdownOutcome::ChildFuse };
/// let group = LocalVerdict { triggered: false, outcome: MeltdownOutcome::Continue };
/// let supervisor = LocalVerdict { triggered: false, outcome: MeltdownOutcome::Continue };
///
/// let merged = merge_meltdown_verdicts(child, group, supervisor);
/// assert_eq!(merged.effective_outcome, MeltdownOutcome::ChildFuse);
/// assert_eq!(merged.lead_scope, Some(MeltdownScope::Child));
/// ```
pub fn merge_meltdown_verdicts(
    child_verdict: LocalVerdict,
    group_verdict: LocalVerdict,
    supervisor_verdict: LocalVerdict,
) -> MergedVerdict {
    use crate::event::payload::MeltdownScope;

    // Collect triggered scopes
    let mut scopes_triggered = Vec::new();
    if child_verdict.triggered {
        scopes_triggered.push(MeltdownScope::Child);
    }
    if group_verdict.triggered {
        scopes_triggered.push(MeltdownScope::Group);
    }
    if supervisor_verdict.triggered {
        scopes_triggered.push(MeltdownScope::Supervisor);
    }

    // Determine effective outcome (most restrictive wins)
    // Priority: SupervisorFuse > GroupFuse > ChildFuse > Continue
    let effective_outcome = [
        supervisor_verdict.outcome,
        group_verdict.outcome,
        child_verdict.outcome,
    ]
    .iter()
    .max_by(|a, b| outcome_severity(**a).cmp(&outcome_severity(**b)))
    .copied()
    .unwrap_or(MeltdownOutcome::Continue);

    // Determine lead_scope only among scopes tied with the effective outcome.
    let lead_scope = if child_verdict.triggered && child_verdict.outcome == effective_outcome {
        Some(MeltdownScope::Child)
    } else if group_verdict.triggered && group_verdict.outcome == effective_outcome {
        Some(MeltdownScope::Group)
    } else if supervisor_verdict.triggered && supervisor_verdict.outcome == effective_outcome {
        Some(MeltdownScope::Supervisor)
    } else {
        None
    };

    MergedVerdict {
        effective_outcome,
        scopes_triggered,
        lead_scope,
    }
}

/// Returns severity level for outcome comparison (higher = more restrictive).
fn outcome_severity(outcome: MeltdownOutcome) -> u8 {
    match outcome {
        MeltdownOutcome::Continue => 0,
        MeltdownOutcome::ChildFuse => 1,
        MeltdownOutcome::GroupFuse => 2,
        MeltdownOutcome::SupervisorFuse => 3,
    }
}

#[cfg(test)]
mod merge_tests {
    use crate::event::payload::MeltdownScope;
    use crate::policy::meltdown::{LocalVerdict, MeltdownOutcome, merge_meltdown_verdicts};

    /// Tests merging verdicts when only child-level meltdown is triggered.
    #[test]
    fn test_merge_child_only() {
        let child = LocalVerdict {
            triggered: true,
            outcome: MeltdownOutcome::ChildFuse,
        };
        let group = LocalVerdict {
            triggered: false,
            outcome: MeltdownOutcome::Continue,
        };
        let supervisor = LocalVerdict {
            triggered: false,
            outcome: MeltdownOutcome::Continue,
        };

        let merged = merge_meltdown_verdicts(child, group, supervisor);
        assert_eq!(merged.effective_outcome, MeltdownOutcome::ChildFuse);
        assert_eq!(merged.scopes_triggered, vec![MeltdownScope::Child]);
        assert_eq!(merged.lead_scope, Some(MeltdownScope::Child));
    }

    /// Tests merging verdicts when all three scopes trigger with different severity.
    #[test]
    fn test_merge_all_three_uses_strictest_scope() {
        let child = LocalVerdict {
            triggered: true,
            outcome: MeltdownOutcome::ChildFuse,
        };
        let group = LocalVerdict {
            triggered: true,
            outcome: MeltdownOutcome::GroupFuse,
        };
        let supervisor = LocalVerdict {
            triggered: true,
            outcome: MeltdownOutcome::SupervisorFuse,
        };

        let merged = merge_meltdown_verdicts(child, group, supervisor);
        // Most restrictive outcome
        assert_eq!(merged.effective_outcome, MeltdownOutcome::SupervisorFuse);
        // All scopes triggered
        assert_eq!(merged.scopes_triggered.len(), 3);
        // Lead scope is selected from scopes matching the effective outcome.
        assert_eq!(merged.lead_scope, Some(MeltdownScope::Supervisor));
    }

    /// Tests tie-breaking when multiple scopes share the effective outcome.
    #[test]
    fn test_merge_tie_breaks_matching_effective_outcome() {
        let child = LocalVerdict {
            triggered: true,
            outcome: MeltdownOutcome::GroupFuse,
        };
        let group = LocalVerdict {
            triggered: true,
            outcome: MeltdownOutcome::GroupFuse,
        };
        let supervisor = LocalVerdict {
            triggered: false,
            outcome: MeltdownOutcome::Continue,
        };

        let merged = merge_meltdown_verdicts(child, group, supervisor);
        assert_eq!(merged.effective_outcome, MeltdownOutcome::GroupFuse);
        assert_eq!(merged.lead_scope, Some(MeltdownScope::Child));
    }

    /// Tests merging verdicts when group and supervisor trigger but child does not.
    #[test]
    fn test_merge_group_and_supervisor() {
        let child = LocalVerdict {
            triggered: false,
            outcome: MeltdownOutcome::Continue,
        };
        let group = LocalVerdict {
            triggered: true,
            outcome: MeltdownOutcome::GroupFuse,
        };
        let supervisor = LocalVerdict {
            triggered: true,
            outcome: MeltdownOutcome::SupervisorFuse,
        };

        let merged = merge_meltdown_verdicts(child, group, supervisor);
        assert_eq!(merged.effective_outcome, MeltdownOutcome::SupervisorFuse);
        assert_eq!(merged.scopes_triggered.len(), 2);
        // Lead scope is selected from scopes matching the effective outcome.
        assert_eq!(merged.lead_scope, Some(MeltdownScope::Supervisor));
    }

    /// Tests merging verdicts when no scopes are triggered.
    #[test]
    fn test_merge_none_triggered() {
        let child = LocalVerdict {
            triggered: false,
            outcome: MeltdownOutcome::Continue,
        };
        let group = LocalVerdict {
            triggered: false,
            outcome: MeltdownOutcome::Continue,
        };
        let supervisor = LocalVerdict {
            triggered: false,
            outcome: MeltdownOutcome::Continue,
        };

        let merged = merge_meltdown_verdicts(child, group, supervisor);
        assert_eq!(merged.effective_outcome, MeltdownOutcome::Continue);
        assert!(merged.scopes_triggered.is_empty());
        assert_eq!(merged.lead_scope, None);
    }
}
