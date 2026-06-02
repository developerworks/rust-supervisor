//! Observation helpers for the service role example.

// Import command result types.
use rust_supervisor::control::command::CommandResult;
// Import current state projection values.
use rust_supervisor::control::command::CurrentState;
// Import shutdown result values.
use rust_supervisor::shutdown::coordinator::ShutdownResult;
// Import broadcast receiver values.
use tokio::sync::broadcast;
// Import service lifecycle facts.
use crate::service_task::ServiceEvent;

/// Waits for the service initialization fact and prints it.
///
/// # Arguments
///
/// - `events`: Receiver for service lifecycle facts.
///
/// # Returns
///
/// This function returns after the initialized event appears or the channel closes.
pub async fn wait_for_initialization(
    events: &mut tokio::sync::mpsc::UnboundedReceiver<ServiceEvent>,
) {
    // Read service facts until initialization appears.
    while let Some(event) = events.recv().await {
        // Print the service fact.
        print_service_event("initialization", &event);
        // Stop waiting once initialization is observed.
        if matches!(event, ServiceEvent::Initialized { .. }) {
            break;
        }
    }
}

/// Drains and prints all currently available service lifecycle facts.
///
/// # Arguments
///
/// - `label`: Output label attached to each printed fact.
/// - `events`: Receiver for service lifecycle facts.
///
/// # Returns
///
/// This function does not return a value.
pub fn drain_service_events(
    label: &str,
    events: &mut tokio::sync::mpsc::UnboundedReceiver<ServiceEvent>,
) {
    // Drain events that have already arrived.
    while let Ok(event) = events.try_recv() {
        // Print one service fact.
        print_service_event(label, &event);
    }
}

/// Drains and prints runtime event text from the supervisor.
///
/// # Arguments
///
/// - `events`: Runtime event receiver.
///
/// # Returns
///
/// This function does not return a value.
pub fn drain_runtime_events(events: &mut broadcast::Receiver<String>) {
    // Drain events that have already arrived.
    while let Ok(event) = events.try_recv() {
        // Print service-relevant runtime events.
        if is_service_relevant_event(&event) {
            // Print the runtime event.
            println!("runtime event={event}");
        }
    }
}

/// Prints a labeled current state projection.
///
/// # Arguments
///
/// - `label`: Output label for this state snapshot.
/// - `result`: Command result returned by `current_state`.
///
/// # Returns
///
/// This function does not return a value.
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
///
/// # Arguments
///
/// - `result`: Shutdown result returned by `shutdown_tree`.
///
/// # Returns
///
/// This function does not return a value.
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

/// Prints one service lifecycle fact.
///
/// # Arguments
///
/// - `label`: Output label attached to the fact.
/// - `event`: Service lifecycle fact to print.
///
/// # Returns
///
/// This function does not return a value.
fn print_service_event(label: &str, event: &ServiceEvent) {
    // Match the service fact shape.
    match event {
        ServiceEvent::Initialized { child_id, path } => {
            // Print initialization details.
            println!("service {label}: initialized child={child_id} path={path}");
        }
        ServiceEvent::Running { child_id, tick } => {
            // Print running details.
            println!("service {label}: running child={child_id} tick={tick}");
        }
        ServiceEvent::Stopping { child_id } => {
            // Print cooperative stop details.
            println!("service {label}: stopping child={child_id}");
        }
    }
}

/// Prints a compact current state summary.
///
/// # Arguments
///
/// - `label`: Output label for this state snapshot.
/// - `state`: Current runtime state to print.
///
/// # Returns
///
/// This function does not return a value.
fn print_state_summary(label: &str, state: &CurrentState) {
    // Print summary fields.
    println!(
        "state {label}: child_count={} shutdown_completed={}",
        state.child_count, state.shutdown_completed
    );
}

/// Returns whether a runtime event is relevant to this service example.
///
/// # Arguments
///
/// - `event`: Runtime event text.
///
/// # Returns
///
/// Returns `true` for command, child, shutdown, and runtime startup events.
fn is_service_relevant_event(event: &str) -> bool {
    // Keep the event filter small and readable.
    event.starts_with("runtime_control_loop_started")
        || event.starts_with("control_command:")
        || event.starts_with("shutdown_")
        || event.starts_with("child_shutdown_cancel_delivered")
        || event.starts_with("child_shutdown_graceful")
        || event.starts_with("child_shutdown_aborted")
}
