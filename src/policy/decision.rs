//! Restart policy decisions for typed task exits.
//!
//! The module converts typed exits into explicit restart decisions. It does not
//! inspect string messages and it does not own runtime state.

use crate::error::types::TaskFailureKind;
use crate::policy::backoff::BackoffPolicy;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Restart scope selected after a child failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SupervisionStrategy {
    /// Restart only the failed child.
    OneForOne,
    /// Restart every child in the same supervisor scope.
    OneForAll,
    /// Restart the failed child and all later declared siblings.
    RestForOne,
}

/// Rule that decides whether a task exit is restartable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RestartPolicy {
    /// Restart after both successful and failed exits.
    Permanent,
    /// Restart after failed exits only.
    Transient,
    /// Never restart automatically.
    Temporary,
}

/// Failure category consumed by the policy engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyFailureKind {
    /// A failure that may succeed on a later attempt.
    Recoverable,
    /// A configuration error that should stop the tree.
    FatalConfig,
    /// A code defect that should be escalated.
    FatalBug,
    /// A dependency failure that may be recoverable.
    ExternalDependency,
    /// The task exceeded its runtime budget.
    Timeout,
    /// The task panicked.
    Panic,
    /// The task was cancelled intentionally.
    Cancelled,
    /// The task missed its heartbeat budget.
    Unhealthy,
}

impl From<TaskFailureKind> for PolicyFailureKind {
    fn from(value: TaskFailureKind) -> Self {
        match value {
            TaskFailureKind::Error => Self::Recoverable,
            TaskFailureKind::Panic => Self::Panic,
            TaskFailureKind::Timeout => Self::Timeout,
            TaskFailureKind::Unhealthy => Self::Unhealthy,
            TaskFailureKind::Cancelled => Self::Cancelled,
        }
    }
}

/// Typed exit information supplied to restart policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskExit {
    /// The task completed successfully.
    Succeeded,
    /// The task failed with a typed category.
    Failed {
        /// Failure category used for policy decisions.
        kind: PolicyFailureKind,
    },
}

/// Explicit decision returned by the policy engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RestartDecision {
    /// Do not restart the child.
    DoNotRestart,
    /// Restart after the supplied delay.
    RestartAfter {
        /// Delay before the next restart attempt.
        delay: Duration,
    },
    /// Stop automatic restart and place the child in quarantine.
    Quarantine,
    /// Escalate the failure to the parent supervisor.
    EscalateToParent,
    /// Shut down the whole supervisor tree.
    ShutdownTree,
}

/// Stateless restart policy engine.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyEngine;

impl PolicyEngine {
    /// Creates a policy engine.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a [`PolicyEngine`].
    ///
    /// # Examples
    ///
    /// ```
    /// let engine = rust_supervisor::policy::decision::PolicyEngine::new();
    /// let _ = engine;
    /// ```
    pub fn new() -> Self {
        Self
    }

    /// Decides the restart action for a typed exit.
    ///
    /// # Arguments
    ///
    /// - `policy`: Restart policy configured for the child.
    /// - `exit`: Typed task exit.
    /// - `attempt`: One-based restart attempt used for backoff.
    /// - `backoff`: Backoff policy used when a restart is allowed.
    ///
    /// # Returns
    ///
    /// Returns a [`RestartDecision`] that the runtime can execute.
    pub fn decide(
        &self,
        policy: RestartPolicy,
        exit: TaskExit,
        attempt: u64,
        backoff: &BackoffPolicy,
    ) -> RestartDecision {
        match exit {
            TaskExit::Succeeded => self.decide_success(policy, attempt, backoff),
            TaskExit::Failed { kind } => self.decide_failure(policy, kind, attempt, backoff),
        }
    }

    /// Decides behavior after successful completion.
    ///
    /// # Arguments
    ///
    /// - `policy`: Restart policy configured for the child.
    /// - `attempt`: One-based restart attempt used for backoff.
    /// - `backoff`: Backoff policy used when a restart is allowed.
    ///
    /// # Returns
    ///
    /// Returns a restart decision for a successful exit.
    fn decide_success(
        &self,
        policy: RestartPolicy,
        attempt: u64,
        backoff: &BackoffPolicy,
    ) -> RestartDecision {
        match policy {
            RestartPolicy::Permanent => RestartDecision::RestartAfter {
                delay: backoff.delay_for_attempt(attempt),
            },
            RestartPolicy::Transient | RestartPolicy::Temporary => RestartDecision::DoNotRestart,
        }
    }

    /// Decides behavior after a typed failure.
    ///
    /// # Arguments
    ///
    /// - `policy`: Restart policy configured for the child.
    /// - `kind`: Failure kind supplied by the runner.
    /// - `attempt`: One-based restart attempt used for backoff.
    /// - `backoff`: Backoff policy used when a restart is allowed.
    ///
    /// # Returns
    ///
    /// Returns a restart decision for a failed exit.
    fn decide_failure(
        &self,
        policy: RestartPolicy,
        kind: PolicyFailureKind,
        attempt: u64,
        backoff: &BackoffPolicy,
    ) -> RestartDecision {
        match kind {
            PolicyFailureKind::FatalConfig => RestartDecision::ShutdownTree,
            PolicyFailureKind::FatalBug => RestartDecision::EscalateToParent,
            PolicyFailureKind::Cancelled => RestartDecision::DoNotRestart,
            _ => self.restartable_failure(policy, attempt, backoff),
        }
    }

    /// Applies restart policy to a restartable failure.
    ///
    /// # Arguments
    ///
    /// - `policy`: Restart policy configured for the child.
    /// - `attempt`: One-based restart attempt used for backoff.
    /// - `backoff`: Backoff policy used when a restart is allowed.
    ///
    /// # Returns
    ///
    /// Returns a restart decision for a restartable failure.
    fn restartable_failure(
        &self,
        policy: RestartPolicy,
        attempt: u64,
        backoff: &BackoffPolicy,
    ) -> RestartDecision {
        match policy {
            RestartPolicy::Permanent | RestartPolicy::Transient => RestartDecision::RestartAfter {
                delay: backoff.delay_for_attempt(attempt),
            },
            RestartPolicy::Temporary => RestartDecision::DoNotRestart,
        }
    }
}
