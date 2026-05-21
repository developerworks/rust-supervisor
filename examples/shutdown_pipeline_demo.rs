//! Demonstrates the four-stage shutdown pipeline with phase transitions,
//! timeout escalation, child shutdown outcomes, and the final reconcile report.
//!
//! The shutdown pipeline progresses through these phases:
//!   1. RequestStop  — cancellation propagates to all children
//!   2. GracefulDrain — runtime waits for cooperative child completion
//!   3. AbortStragglers — runtime escalates stragglers to abort
//!   4. Reconcile   — final state reconciliation and cleanup
//!
//! This example constructs shutdown phases, coordinator transitions, and a
//! sample pipeline report to illustrate the lifecycle.

use rust_supervisor::id::types::{ChildId, ChildStartCount, Generation, SupervisorPath};
use rust_supervisor::shutdown::coordinator::ShutdownCoordinator;
use rust_supervisor::shutdown::report::{
    ChildShutdownOutcome, ChildShutdownOutcomeInput, ChildShutdownStatus, ResourceReconcileStatus,
    ShutdownPipelineReport, ShutdownReconcileReport,
};
use rust_supervisor::shutdown::stage::{ShutdownCause, ShutdownPhase, ShutdownPolicy};
use std::time::Duration;

/// Runs the shutdown pipeline demonstration.
fn main() {
    println!("=== Shutdown Pipeline Demo ===");
    println!();

    // --- Shutdown Policy ---
    println!("--- Shutdown Policy ---");
    println!();

    let policy = ShutdownPolicy::new(
        Duration::from_secs(5), // graceful_timeout
        Duration::from_secs(1), // abort_wait
        true,                   // abort_after_timeout
    );

    println!("  graceful_timeout = {:?}", policy.graceful_timeout);
    println!("  abort_wait       = {:?}", policy.abort_wait);
    println!(
        "  interpretation: wait {:?} for cooperative stop, then {:?} for abort",
        policy.graceful_timeout, policy.abort_wait,
    );

    // --- Shutdown Phases ---
    println!();
    println!("--- Shutdown Phases ---");
    println!();

    let mut phase = ShutdownPhase::Idle;
    println!("  Phase 0: {:?} - supervisor is running normally", phase);

    phase = ShutdownPhase::RequestStop;
    println!(
        "  Phase 1: {:?} - shutdown requested, cancellation sent to all children",
        phase
    );

    phase = ShutdownPhase::GracefulDrain;
    println!(
        "  Phase 2: {:?} - waiting for children to exit cooperatively",
        phase
    );

    phase = ShutdownPhase::AbortStragglers;
    println!(
        "  Phase 3: {:?} - graceful timeout expired, stragglers aborted",
        phase
    );

    phase = ShutdownPhase::Reconcile;
    println!(
        "  Phase 4: {:?} - reconciling final state, cleaning up resources",
        phase
    );

    phase = ShutdownPhase::Completed;
    println!("  Phase 5: {:?} - shutdown complete", phase);

    // --- Phase transitions ---
    println!();
    println!("--- Phase Transitions ---");
    println!();

    let mut current = ShutdownPhase::Idle;
    while let Some(next) = current.next() {
        println!("  {:?} -> {:?}", current, next);
        current = next;
    }

    // --- ShutdownCoordinator ---
    println!();
    println!("--- ShutdownCoordinator ---");
    println!();

    let coord_policy = ShutdownPolicy::new(Duration::from_secs(5), Duration::from_secs(1), true);
    let mut coordinator = ShutdownCoordinator::new(coord_policy);

    let cause = ShutdownCause::new("operator", "scheduled maintenance");
    let result = coordinator.request_stop(cause);

    println!(
        "  after request_stop: phase={:?} idempotent={}",
        result.phase, result.idempotent
    );
    println!(
        "  cause: requested_by={} reason={}",
        result.cause.requested_by, result.cause.reason
    );

    // Idempotent shutdown request (same cause).
    let result2 = coordinator.request_stop(ShutdownCause::new("operator", "scheduled maintenance"));
    println!(
        "  idempotent request: phase={:?} idempotent={}",
        result2.phase, result2.idempotent
    );

    // --- Child Shutdown Outcomes ---
    println!();
    println!("--- Child Shutdown Outcomes ---");
    println!();

    let make_outcome = |name: &str, status: ChildShutdownStatus, phase: ShutdownPhase| {
        ChildShutdownOutcome::new(ChildShutdownOutcomeInput {
            child_id: ChildId::new(name),
            path: SupervisorPath::root().join(name),
            generation: Generation::initial(),
            child_start_count: ChildStartCount::first(),
            status,
            cancel_delivered: true,
            exit: None,
            phase,
            reason: format!("child completed during {:?}", phase),
        })
    };

    let outcomes = vec![
        make_outcome(
            "feed_handler",
            ChildShutdownStatus::Graceful,
            ShutdownPhase::GracefulDrain,
        ),
        make_outcome(
            "risk_engine",
            ChildShutdownStatus::Aborted,
            ShutdownPhase::AbortStragglers,
        ),
        make_outcome(
            "audit_sink",
            ChildShutdownStatus::LateReport,
            ShutdownPhase::Reconcile,
        ),
    ];

    for outcome in &outcomes {
        println!(
            "  child={:12} status={:?} phase={:?}",
            outcome.child_id.value, outcome.status, outcome.phase,
        );
    }

    // --- Reconcile Report ---
    println!();
    println!("--- Reconcile Report ---");
    println!();

    let reconcile = ShutdownReconcileReport {
        registry_status: ResourceReconcileStatus::Cleaned,
        runtime_handle_status: ResourceReconcileStatus::Cleaned,
        journal_status: ResourceReconcileStatus::Recorded,
        metrics_status: ResourceReconcileStatus::Recorded,
        socket_status: ResourceReconcileStatus::NotOwned,
        orphan_slots: vec![],
        total_slots_checked: 5,
        verified_clean: true,
        warnings: vec![],
    };

    println!("  registry_status       = {:?}", reconcile.registry_status);
    println!(
        "  runtime_handle_status = {:?}",
        reconcile.runtime_handle_status
    );
    println!("  journal_status        = {:?}", reconcile.journal_status);
    println!("  metrics_status        = {:?}", reconcile.metrics_status);
    println!("  socket_status         = {:?}", reconcile.socket_status);
    println!("  orphan_slots          = {:?}", reconcile.orphan_slots);
    println!(
        "  total_slots_checked   = {}",
        reconcile.total_slots_checked
    );
    println!("  verified_clean        = {}", reconcile.verified_clean);

    // --- Full Pipeline Report ---
    println!();
    println!("--- Full Pipeline Report ---");
    println!();

    let report = ShutdownPipelineReport {
        cause: ShutdownCause::new("operator", "scheduled maintenance"),
        started_at_unix_nanos: 1000,
        completed_at_unix_nanos: 6123,
        phase: ShutdownPhase::Completed,
        outcomes,
        reconcile,
        idempotent: false,
    };

    println!("  cause.requested_by = {}", report.cause.requested_by);
    println!("  cause.reason       = {}", report.cause.reason);
    println!(
        "  duration_ms        = {}ms",
        report.completed_at_unix_nanos - report.started_at_unix_nanos
    );
    println!("  final_phase        = {:?}", report.phase);
    println!("  child_outcomes     = {}", report.outcomes.len());

    println!();
    println!("=== Summary ===");
    println!("Shutdown pipeline: 5 phases (Idle -> RequestStop -> GracefulDrain");
    println!("                   -> AbortStragglers -> Reconcile -> Completed).");
    println!("ShutdownCoordinator provides idempotent phase transitions.");
    println!("Children exit with Graceful, Aborted, or LateReport status.");
    println!("Reconcile verifies all runtime slots are clean before completion.");
}
