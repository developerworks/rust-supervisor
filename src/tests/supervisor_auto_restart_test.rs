//! Supervisor automatic restart integration tests.
//!
//! These tests verify that child exits reach the runtime control loop and that
//! supervision strategy scopes are executed automatically.

use rust_supervisor::error::types::{TaskFailure, TaskFailureKind};
use rust_supervisor::id::types::ChildId;
use rust_supervisor::runtime::supervisor::Supervisor;
use rust_supervisor::spec::child::{ChildSpec, TaskKind};
use rust_supervisor::spec::supervisor::{GroupStrategy, SupervisionStrategy, SupervisorSpec};
use rust_supervisor::task::factory::{TaskResult, service_fn};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

/// Verifies that `OneForOne` restarts only the failed child after one failure.
#[tokio::test]
async fn one_for_one_restarts_only_failed_child_after_failure() {
    let gate = Arc::new(AtomicBool::new(false));
    let first_start_counts = Arc::new(AtomicUsize::new(0));
    let second_start_counts = Arc::new(AtomicUsize::new(0));
    let first = counted_worker("first", true, first_start_counts.clone(), gate.clone());
    let second = counted_worker("second", false, second_start_counts.clone(), gate.clone());
    let mut spec = SupervisorSpec::root(vec![first, second]);
    spec.strategy = SupervisionStrategy::OneForOne;
    let handle = Supervisor::start(spec).await.expect("start supervisor");
    assert_eq!(current_child_count(&handle).await, 2);

    gate.store(true, Ordering::SeqCst);
    wait_for_count(&first_start_counts, 2).await;
    assert_eq!(second_start_counts.load(Ordering::SeqCst), 1);

    handle
        .shutdown_tree("test", "one_for_one complete")
        .await
        .expect("shutdown supervisor");
}

/// Verifies that `OneForAll` restarts every declared child after one failure.
#[tokio::test]
async fn one_for_all_restarts_every_child_after_failure() {
    let gate = Arc::new(AtomicBool::new(false));
    let first_start_counts = Arc::new(AtomicUsize::new(0));
    let second_start_counts = Arc::new(AtomicUsize::new(0));
    let first = counted_worker("first", true, first_start_counts.clone(), gate.clone());
    let second = counted_worker("second", false, second_start_counts.clone(), gate.clone());
    let mut spec = SupervisorSpec::root(vec![first, second]);
    spec.strategy = SupervisionStrategy::OneForAll;
    let handle = Supervisor::start(spec).await.expect("start supervisor");
    assert_eq!(current_child_count(&handle).await, 2);

    gate.store(true, Ordering::SeqCst);
    wait_for_count(&first_start_counts, 2).await;
    wait_for_count(&second_start_counts, 2).await;

    handle
        .shutdown_tree("test", "one_for_all complete")
        .await
        .expect("shutdown supervisor");
}

/// Verifies that `RestForOne` restarts the failed child and following children.
#[tokio::test]
async fn rest_for_one_restarts_failed_child_and_following_children() {
    let gate = Arc::new(AtomicBool::new(false));
    let first_start_counts = Arc::new(AtomicUsize::new(0));
    let second_start_counts = Arc::new(AtomicUsize::new(0));
    let third_start_counts = Arc::new(AtomicUsize::new(0));
    let first = counted_worker("first", false, first_start_counts.clone(), gate.clone());
    let second = counted_worker("second", true, second_start_counts.clone(), gate.clone());
    let third = counted_worker("third", false, third_start_counts.clone(), gate.clone());
    let mut spec = SupervisorSpec::root(vec![first, second, third]);
    spec.strategy = SupervisionStrategy::RestForOne;
    let handle = Supervisor::start(spec).await.expect("start supervisor");
    assert_eq!(current_child_count(&handle).await, 3);

    gate.store(true, Ordering::SeqCst);
    wait_for_count(&second_start_counts, 2).await;
    wait_for_count(&third_start_counts, 2).await;
    assert_eq!(first_start_counts.load(Ordering::SeqCst), 1);

    handle
        .shutdown_tree("test", "rest_for_one complete")
        .await
        .expect("shutdown supervisor");
}

/// Verifies that a group strategy limits runtime restarts to group members.
#[tokio::test]
async fn group_strategy_restarts_only_group_members_after_failure() {
    let gate = Arc::new(AtomicBool::new(false));
    let first_start_counts = Arc::new(AtomicUsize::new(0));
    let second_start_counts = Arc::new(AtomicUsize::new(0));
    let third_start_counts = Arc::new(AtomicUsize::new(0));
    let fourth_start_counts = Arc::new(AtomicUsize::new(0));
    let first = counted_worker("first", false, first_start_counts.clone(), gate.clone());
    let mut second = counted_worker("second", true, second_start_counts.clone(), gate.clone());
    let mut third = counted_worker("third", false, third_start_counts.clone(), gate.clone());
    let fourth = counted_worker("fourth", false, fourth_start_counts.clone(), gate.clone());
    second.tags.push("pipeline".to_owned());
    third.tags.push("pipeline".to_owned());
    let mut spec = SupervisorSpec::root(vec![first, second, third, fourth]);
    spec.strategy = SupervisionStrategy::OneForAll;
    spec.group_strategies = vec![GroupStrategy::new(
        "pipeline",
        SupervisionStrategy::OneForAll,
    )];
    let handle = Supervisor::start(spec).await.expect("start supervisor");
    assert_eq!(current_child_count(&handle).await, 4);

    gate.store(true, Ordering::SeqCst);
    wait_for_count(&second_start_counts, 2).await;
    wait_for_count(&third_start_counts, 2).await;
    assert_eq!(first_start_counts.load(Ordering::SeqCst), 1);
    assert_eq!(fourth_start_counts.load(Ordering::SeqCst), 1);

    handle
        .shutdown_tree("test", "group_strategy complete")
        .await
        .expect("shutdown supervisor");
}

/// Builds a worker that can fail only its first child_start_count.
///
/// # Arguments
///
/// - `id`: Stable child identifier.
/// - `fail_first_child_start_count`: Whether the first child_start_count should return failure.
/// - `start_counts`: Shared counter updated when the factory future runs.
/// - `gate`: Gate used to make the first child_start_count observable by the test.
///
/// # Returns
///
/// Returns a child specification with a fresh task future factory.
fn counted_worker(
    id: &str,
    fail_first_child_start_count: bool,
    start_counts: Arc<AtomicUsize>,
    gate: Arc<AtomicBool>,
) -> ChildSpec {
    let name = id.to_owned();
    let factory = service_fn(move |_context| {
        let start_counts = start_counts.clone();
        let gate = gate.clone();
        async move {
            let child_start_count = start_counts
                .fetch_add(1, Ordering::SeqCst)
                .saturating_add(1);
            if child_start_count == 1 {
                wait_for_open_gate(&gate).await;
            }
            if fail_first_child_start_count && child_start_count == 1 {
                return TaskResult::Failed(TaskFailure::new(
                    TaskFailureKind::Error,
                    "test_failure",
                    "first child_start_count failed",
                ));
            }
            TaskResult::Succeeded
        }
    });
    ChildSpec::worker(
        ChildId::new(id),
        name,
        TaskKind::AsyncWorker,
        Arc::new(factory),
    )
}

/// Waits until the shared test gate is open.
///
/// # Arguments
///
/// - `gate`: Shared gate state.
///
/// # Returns
///
/// This function returns after the gate value becomes `true`.
async fn wait_for_open_gate(gate: &AtomicBool) {
    while !gate.load(Ordering::SeqCst) {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

/// Waits until an atomic child_start_count counter reaches the expected value.
///
/// # Arguments
///
/// - `start_counts`: ChildStartCount counter to inspect.
/// - `expected`: Minimum child_start_count count expected by the test.
///
/// # Returns
///
/// This function returns after the expected count is reached.
async fn wait_for_count(start_counts: &AtomicUsize, expected: usize) {
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if start_counts.load(Ordering::SeqCst) >= expected {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap_or_else(|_| {
        panic!(
            "child_start_count count timeout, current={}, expected={expected}",
            start_counts.load(Ordering::SeqCst)
        )
    });
}

/// Reads the current child count from the runtime handle.
///
/// # Arguments
///
/// - `handle`: Supervisor handle used for the state query.
///
/// # Returns
///
/// Returns the current runtime child count.
async fn current_child_count(handle: &rust_supervisor::control::handle::SupervisorHandle) -> usize {
    let result = handle.current_state().await.expect("current state");
    match result {
        rust_supervisor::control::command::CommandResult::CurrentState { state } => {
            state.child_count
        }
        other => panic!("unexpected current state result: {other:?}"),
    }
}
