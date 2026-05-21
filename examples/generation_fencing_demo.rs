//! Demonstrates generation fencing mechanics during manual restart.
//!
//! When a restart request arrives while a child still has an active attempt,
//! the runtime enters a fencing sequence:
//!   1. Open -> WaitingForOldStop (cancel delivered)
//!   2. WaitingForOldStop -> AbortingOld (grace deadline expired)
//!   3. AbortingOld -> ReadyToStart (old attempt confirmed finished)
//!   4. ReadyToStart -> Open (new generation started)
//!
//! This example constructs fence states, decisions, and outcomes to illustrate
//! the lifecycle without requiring an actual running supervisor.

use rust_supervisor::child_runner::run_exit::TaskExit;
use rust_supervisor::control::outcome::{
    ChildControlFailure, ChildControlFailurePhase, GenerationFenceDecision, GenerationFenceOutcome,
    GenerationFencePhase, GenerationFenceState, PendingRestart, StaleAttemptReport,
    StaleReportHandling,
};
use rust_supervisor::id::types::{ChildId, ChildStartCount, Generation};
use uuid::Uuid;

/// Runs the generation fencing demonstration.
fn main() {
    println!("=== Generation Fencing Demo ===");
    println!();

    let child_id = ChildId::new("order_processor");
    let command_id = Uuid::nil();

    // Phase 0: Open — no active attempt, no fence.
    println!("--- Phase: Open ---");
    let fence_open = GenerationFencePhase::Open;
    println!("  {fence_open:?} - No active attempt, restart allowed immediately");
    println!();

    // Phase 1: WaitingForOldStop — restart queued behind an active attempt.
    println!("--- Phase: WaitingForOldStop ---");
    let fence_waiting = GenerationFencePhase::WaitingForOldStop;
    let pending = PendingRestart::new(
        command_id,
        "operator",
        "manual restart for deployment",
        Generation { value: 2 },
        ChildStartCount { value: 3 },
        Generation { value: 3 },
        1000,  // requested_at_unix_nanos
        5000,  // stop_deadline_at_unix_nanos
        false, // abort_requested
        0,     // duplicate_request_count
    );
    println!("  {fence_waiting:?} - Restart queued, waiting for old attempt to stop");
    println!(
        "  pending: old_gen={:?} old_attempt={:?} target_gen={:?}",
        pending.old_generation.value, pending.old_attempt.value, pending.target_generation.value,
    );
    println!();

    // Phase 2: AbortingOld — graceful deadline exceeded, abort sent.
    println!("--- Phase: AbortingOld ---");
    let fence_aborting = GenerationFencePhase::AbortingOld;
    println!("  {fence_aborting:?} - Graceful stop deadline exceeded, abort requested");
    println!();

    // Phase 3: ReadyToStart — old attempt confirmed exited.
    println!("--- Phase: ReadyToStart ---");
    let fence_ready = GenerationFencePhase::ReadyToStart;
    println!("  {fence_ready:?} - Old attempt confirmed finished, new generation may start");
    println!();

    // Phase 4: Closed — supervisor shutting down.
    println!("--- Phase: Closed ---");
    let fence_closed = GenerationFencePhase::Closed;
    println!("  {fence_closed:?} - Supervisor shutting down, restart blocked");
    println!();

    // Demonstrate fence decisions.
    println!("=== Fence Decisions ===");
    println!();

    let decisions: [(GenerationFenceDecision, &str); 5] = [
        (
            GenerationFenceDecision::StartedImmediately,
            "No active attempt; new generation started at once",
        ),
        (
            GenerationFenceDecision::QueuedAfterStop,
            "Active attempt present; restart queued behind it",
        ),
        (
            GenerationFenceDecision::AlreadyPending,
            "Duplicate restart merged into existing pending request",
        ),
        (
            GenerationFenceDecision::BlockedByShutdown,
            "Supervisor is shutting down; restart rejected",
        ),
        (
            GenerationFenceDecision::Rejected,
            "Request rejected; see conflict field for reason",
        ),
    ];

    for (decision, description) in &decisions {
        println!("  {decision:?}");
        println!("    -> {description}");
    }
    println!();

    // Build a complete fence outcome.
    println!("=== Complete Fence Outcome ===");
    println!();

    let outcome = GenerationFenceOutcome::new(
        GenerationFenceDecision::QueuedAfterStop,
        Some(Generation { value: 2 }),
        Some(ChildStartCount { value: 3 }),
        Some(Generation { value: 3 }),
        true,  // cancel_delivered
        false, // abort_requested
        None,  // no conflict
    );

    println!("  decision          = {:?}", outcome.decision);
    println!(
        "  old_generation    = {:?}",
        outcome.old_generation.map(|g| g.value)
    );
    println!(
        "  old_attempt       = {:?}",
        outcome.old_attempt.map(|a| a.value)
    );
    println!(
        "  target_generation = {:?}",
        outcome.target_generation.map(|g| g.value)
    );
    println!("  cancel_delivered  = {}", outcome.cancel_delivered);
    println!("  abort_requested   = {}", outcome.abort_requested);
    println!("  conflict          = {:?}", outcome.conflict);
    println!();

    // Build a rejected outcome with conflict detail.
    println!("=== Rejected Outcome with Conflict ===");
    println!();

    let conflict = ChildControlFailure::new(
        ChildControlFailurePhase::WaitCompletion,
        "child is already being stopped by another command",
        true, // recoverable
    );

    let rejected = GenerationFenceOutcome::new(
        GenerationFenceDecision::Rejected,
        Some(Generation { value: 2 }),
        Some(ChildStartCount { value: 3 }),
        None,
        false,
        false,
        Some(conflict),
    );

    println!("  decision          = {:?}", rejected.decision);
    println!(
        "  conflict.phase    = {:?}",
        rejected.conflict.as_ref().map(|c| c.phase)
    );
    println!(
        "  conflict.reason   = {:?}",
        rejected.conflict.as_ref().map(|c| &c.reason)
    );
    println!(
        "  conflict.recoverable = {:?}",
        rejected.conflict.as_ref().map(|c| c.recoverable)
    );
    println!();

    // Demonstrate stale report handling.
    println!("=== Stale Attempt Report ===");
    println!();

    let stale = StaleAttemptReport::new(
        child_id,
        Generation { value: 1 },
        ChildStartCount { value: 2 },
        Some(Generation { value: 3 }),
        Some(ChildStartCount { value: 1 }),
        TaskExit::Succeeded,
        StaleReportHandling::IgnoredForState,
        1000,
    );

    println!(
        "  A late completion report from gen={} attempt={} arrived after",
        stale.reported_generation.value, stale.reported_attempt.value,
    );
    println!(
        "  the runtime had moved to gen={:?} attempt={:?}.",
        stale.current_generation.map(|g| g.value),
        stale.current_attempt.map(|a| a.value),
    );
    println!("  handled_as = {:?}", stale.handled_as);
    println!();

    // Demonstrate GenerationFenceState.
    println!("=== GenerationFenceState Timeline ===");
    println!();

    let states: [(GenerationFencePhase, &str); 5] = [
        (GenerationFencePhase::Open, "initial state, no fence"),
        (
            GenerationFencePhase::WaitingForOldStop,
            "restart accepted, waiting for old attempt",
        ),
        (
            GenerationFencePhase::AbortingOld,
            "grace deadline expired, aborting",
        ),
        (
            GenerationFencePhase::ReadyToStart,
            "old attempt done, starting new generation",
        ),
        (
            GenerationFencePhase::Closed,
            "supervisor shutting down, no restarts",
        ),
    ];

    for (phase, desc) in &states {
        let state = GenerationFenceState {
            phase: *phase,
            active_generation: None,
            active_attempt: None,
            pending_restart: None,
            last_stale_report: None,
        };
        println!("  {:20} - {}", format!("{:?}", state.phase), desc);
    }

    println!();
    println!("=== Summary ===");
    println!("Generation fencing ensures at-most-one active attempt per child.");
    println!("Restart requests queue behind the active attempt and wait for it to stop.");
    println!(
        "Late (stale) reports from old generations are ignored for state but recorded for audit."
    );
}
