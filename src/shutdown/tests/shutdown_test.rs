//! Shutdown coordinator tests.
//!
//! These tests verify idempotent four-stage shutdown transitions.

use rust_supervisor::shutdown::coordinator::ShutdownCoordinator;
use rust_supervisor::shutdown::stage::{ShutdownCause, ShutdownPhase, ShutdownPolicy};
use std::time::Duration;

#[test]
fn shutdown_request_is_idempotent() {
    let policy = ShutdownPolicy::new(Duration::from_secs(5), Duration::from_secs(1), true);
    let mut coordinator = ShutdownCoordinator::new(policy);

    let first = coordinator.request_stop(ShutdownCause::new("operator", "deploy"));
    let second = coordinator.request_stop(ShutdownCause::new("operator", "repeat"));

    assert_eq!(first.phase, ShutdownPhase::RequestStop);
    assert!(second.idempotent);
    assert_eq!(second.cause.reason, "deploy");
}

#[test]
fn shutdown_phase_advances_to_completed() {
    let policy = ShutdownPolicy::new(Duration::from_secs(5), Duration::from_secs(1), true);
    let mut coordinator = ShutdownCoordinator::new(policy);

    coordinator.request_stop(ShutdownCause::new("operator", "deploy"));
    coordinator.advance();
    coordinator.advance();
    coordinator.advance();
    coordinator.advance();

    assert_eq!(coordinator.phase(), ShutdownPhase::Completed);
}
