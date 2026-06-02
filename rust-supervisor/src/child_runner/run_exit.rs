//! Task run exit classification.
//!
//! This module converts task results and runtime failures into a typed exit
//! model that policy code can consume without string parsing.

use crate::error::types::{TaskFailure, TaskFailureKind};
use crate::task::factory::TaskResult;
use serde::{Deserialize, Serialize};

/// Exit classification for one task run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskExit {
    /// The task returned success.
    Succeeded,
    /// The task returned cancellation.
    Cancelled,
    /// The task returned a typed failure.
    Failed(TaskFailure),
    /// The task panicked before returning a result.
    Panicked(String),
    /// The task timed out.
    TimedOut,
}

impl TaskExit {
    /// Converts a task result into an exit classification.
    ///
    /// # Arguments
    ///
    /// - `result`: Task result returned by the task future.
    ///
    /// # Returns
    ///
    /// Returns the corresponding [`TaskExit`].
    ///
    /// # Examples
    ///
    /// ```
    /// let exit = rust_supervisor::child_runner::run_exit::TaskExit::from_task_result(
    ///     rust_supervisor::task::factory::TaskResult::Succeeded,
    /// );
    /// assert!(exit.is_success());
    /// ```
    pub fn from_task_result(result: TaskResult) -> Self {
        match result {
            TaskResult::Succeeded => Self::Succeeded,
            TaskResult::Cancelled => Self::Cancelled,
            TaskResult::Failed(failure) => Self::Failed(failure),
        }
    }

    /// Returns whether this exit represents a successful task.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns `true` only for [`TaskExit::Succeeded`].
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Succeeded)
    }

    /// Returns the failure kind for policy evaluation.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns `None` for successful exits.
    pub fn failure_kind(&self) -> Option<TaskFailureKind> {
        match self {
            Self::Succeeded => None,
            Self::Cancelled => Some(TaskFailureKind::Cancelled),
            Self::Failed(failure) => Some(failure.kind.clone()),
            Self::Panicked(_message) => Some(TaskFailureKind::Panic),
            Self::TimedOut => Some(TaskFailureKind::Timeout),
        }
    }
}
