//! Observation helpers for the sidecar role example.

// Import command result types.
use rust_supervisor::control::command::CommandResult;
// Import current state projection values.
use rust_supervisor::control::command::CurrentState;
// Import shutdown result values.
use rust_supervisor::shutdown::coordinator::ShutdownResult;
// Import broadcast receiver values.
use tokio::sync::broadcast;
// Import sidecar lifecycle facts.
use crate::sidecar_task::SidecarEvent;

/// Waits for a number of initialization facts and prints them.
///
/// # Arguments
///
/// - `events`: Receiver for sidecar example lifecycle facts.
/// - `expected`: Number of initialized children to observe.
///
/// # Returns
///
/// This function returns after the expected initialized events appear or the channel closes.
pub async fn wait_for_initializations(
    events: &mut tokio::sync::mpsc::UnboundedReceiver<SidecarEvent>,
    expected: usize,
) {
    // Count initialized children.
    let mut initialized = 0_usize;
    // Read facts until all expected children initialize.
    while let Some(event) = events.recv().await {
        // Print the child fact.
        print_role_event("initialization", &event);
        // Count initialization events.
        if matches!(event, SidecarEvent::Initialized { .. }) {
            initialized += 1;
        }
        // Stop waiting once all children are initialized.
        if initialized >= expected {
            break;
        }
    }
}

/// Drains and prints all currently available sidecar example lifecycle facts.
///
/// # Arguments
///
/// - `label`: Output label attached to each printed fact.
/// - `events`: Receiver for sidecar example lifecycle facts.
///
/// # Returns
///
/// This function does not return a value.
pub fn drain_role_events(
    label: &str,
    events: &mut tokio::sync::mpsc::UnboundedReceiver<SidecarEvent>,
) {
    // Drain events that have already arrived.
    while let Ok(event) = events.try_recv() {
        // Print one child fact.
        print_role_event(label, &event);
    }
}

/// Drains and prints runtime event text from the supervisor.
pub fn drain_runtime_events(events: &mut broadcast::Receiver<String>) {
    // Drain events that have already arrived.
    while let Ok(event) = events.try_recv() {
        // Print sidecar-relevant runtime events.
        if is_sidecar_relevant_event(&event) {
            // Print the runtime event.
            println!("runtime event={event}");
        }
    }
}

/// Prints a labeled current state projection.
pub fn print_current_state(label: &str, result: CommandResult) {
    // Continue only for current state results.
    if let CommandResult::CurrentState { state } = result {
        // Print the state header.
        print_state_summary(label, &state);
        // Print each child runtime record.
        for record in state.child_runtime_records {
            // Print compact child state.
            println!(
                "state {label}: child={} status={:?} readiness={:?} heartbeat_stale={}",
                record.child_id,
                record.status,
                record.liveness.readiness,
                record.liveness.heartbeat_stale
            );
        }
    }
}

/// Prints a completed shutdown result.
pub fn print_shutdown_result(result: ShutdownResult) {
    // Print the final shutdown phase.
    println!(
        "shutdown phase={:?} idempotent={}",
        result.phase, result.idempotent
    );
    // Print the report when it is available.
    if let Some(report) = result.report {
        // Print the report phase and outcome count.
        println!(
            "shutdown report: phase={:?} outcomes={}",
            report.phase,
            report.outcomes.len()
        );
        // Print each child shutdown outcome.
        for outcome in report.outcomes {
            // Print compact shutdown outcome details.
            println!(
                "shutdown outcome: child={} status={:?} phase={:?} cancel_delivered={}",
                outcome.child_id, outcome.status, outcome.phase, outcome.cancel_delivered
            );
        }
    }
}

/// Prints one sidecar example lifecycle fact.
fn print_role_event(label: &str, event: &SidecarEvent) {
    // Match the sidecar example fact shape.
    match event {
        SidecarEvent::Initialized { child_id, role } => {
            // Print initialization details.
            println!("sidecar {label}: initialized role={role} child={child_id}");
        }
        SidecarEvent::Running {
            child_id,
            role,
            tick,
        } => {
            // Print running details.
            println!("sidecar {label}: running role={role} child={child_id} tick={tick}");
        }
        SidecarEvent::Stopping { child_id, role } => {
            // Print cooperative stop details.
            println!("sidecar {label}: stopping role={role} child={child_id}");
        }
    }
}

/// Prints a compact current state summary.
fn print_state_summary(label: &str, state: &CurrentState) {
    // Print summary fields.
    println!(
        "state {label}: child_count={} shutdown_completed={}",
        state.child_count, state.shutdown_completed
    );
}

/// Returns whether a runtime event is relevant to this sidecar example.
fn is_sidecar_relevant_event(event: &str) -> bool {
    // Keep the event filter small and readable.
    event.starts_with("runtime_control_loop_started")
        || event.starts_with("control_command:")
        || event.starts_with("shutdown_")
        || event.starts_with("child_shutdown_cancel_delivered")
        || event.starts_with("child_shutdown_graceful")
        || event.starts_with("child_shutdown_aborted")
}
