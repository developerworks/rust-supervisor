//! Child runner tests.
//!
//! These tests verify task attempt execution and typed exit mapping.

use rust_supervisor::child_runner::attempt::TaskExit;
use rust_supervisor::error::types::{TaskFailure, TaskFailureKind};
use rust_supervisor::task::factory::TaskResult;

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
