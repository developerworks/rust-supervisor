//! Child runner tests.
//!
//! These tests verify task child_start_count execution and typed exit mapping.

use rust_supervisor::child_runner::run_exit::TaskExit;
use rust_supervisor::error::types::{TaskFailure, TaskFailureKind};
use rust_supervisor::task::factory::TaskResult;

/// Verifies success and failure task result classification.
/// Verifies task result mapping into typed task exits.
#[test]
fn task_exit_classifies_success_and_failure() {
    let success = TaskExit::from_task_result(TaskResult::Succeeded);
    let failure = TaskExit::from_task_result(TaskResult::Failed(TaskFailure::new(
        TaskFailureKind::Timeout,
        "timeout",
        "deadline elapsed",
    )));

    assert!(success.is_success());
    assert_eq!(failure.failure_kind(), Some(TaskFailureKind::Timeout));
}
