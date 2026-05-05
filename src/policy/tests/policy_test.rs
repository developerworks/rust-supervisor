//! Restart decision tests.
//!
//! These tests verify typed policy decisions for task exits.

use rust_supervisor::policy::backoff::BackoffPolicy;
use rust_supervisor::policy::decision::{
    PolicyEngine, PolicyFailureKind, RestartDecision, RestartPolicy, TaskExit,
};
use std::time::Duration;

#[test]
fn transient_failure_restarts_after_backoff() {
    let engine = PolicyEngine::new();
    let backoff = BackoffPolicy::new(
        Duration::from_millis(10),
        Duration::from_millis(100),
        0,
        Duration::from_secs(1),
    );

    let decision = engine.decide(
        RestartPolicy::Transient,
        TaskExit::Failed {
            kind: PolicyFailureKind::Recoverable,
        },
        2,
        &backoff,
    );

    assert_eq!(
        decision,
        RestartDecision::RestartAfter {
            delay: Duration::from_millis(20)
        }
    );
}

#[test]
fn fatal_config_shuts_down_tree() {
    let engine = PolicyEngine::new();
    let backoff = BackoffPolicy::new(
        Duration::from_millis(10),
        Duration::from_millis(100),
        0,
        Duration::from_secs(1),
    );

    let decision = engine.decide(
        RestartPolicy::Permanent,
        TaskExit::Failed {
            kind: PolicyFailureKind::FatalConfig,
        },
        1,
        &backoff,
    );

    assert_eq!(decision, RestartDecision::ShutdownTree);
}
