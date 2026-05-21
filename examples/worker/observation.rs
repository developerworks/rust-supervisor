//! Observation helpers for the worker role example.

// Import command result types.
use rust_supervisor::control::command::CommandResult;
// Import current state projection values.
use rust_supervisor::control::command::CurrentState;
// Import shutdown result values.
use rust_supervisor::shutdown::coordinator::ShutdownResult;
// Import broadcast receiver values.
use tokio::sync::broadcast;
// Import timeout helpers for runtime event waits.
use tokio::time::{Duration, timeout};
// Import worker lifecycle facts.
use crate::worker_task::WorkerEvent;

/// Waits for the worker initialization fact and prints it.
///
/// # Arguments
///
/// - `events`: Receiver for worker lifecycle facts.
///
/// # Returns
///
/// This function returns after the initialized event appears or the channel closes.
pub async fn wait_for_initialization(
    events: &mut tokio::sync::mpsc::UnboundedReceiver<WorkerEvent>,
) {
    // Read worker facts until initialization appears.
    while let Some(event) = events.recv().await {
        // Print the worker fact.
        print_worker_event("initialization", &event);
        // Stop waiting once initialization is observed.
        if matches!(event, WorkerEvent::Initialized { .. }) {
            break;
        }
    }
}

/// Waits for worker completion while printing worker facts.
///
/// # Arguments
///
/// - `events`: Receiver for worker lifecycle facts.
///
/// # Returns
///
/// This function returns after completion appears or the channel closes.
pub async fn wait_for_completion(events: &mut tokio::sync::mpsc::UnboundedReceiver<WorkerEvent>) {
    // Read worker facts until completion appears.
    while let Some(event) = events.recv().await {
        // Print the worker fact.
        print_worker_event("bounded-work", &event);
        // Stop waiting once completion is observed.
        if matches!(event, WorkerEvent::Completed { .. }) {
            break;
        }
    }
}

/// Waits until the runtime reports the worker child exit.
///
/// # Arguments
///
/// - `events`: Runtime event receiver.
///
/// # Returns
///
/// This function returns after the child exit event appears, the channel closes, or timeout occurs.
pub async fn wait_for_child_exit(events: &mut broadcast::Receiver<String>) {
    // Wait for the runtime to process the worker exit.
    loop {
        // Bound the wait so the example does not hang if event delivery changes.
        match timeout(Duration::from_secs(1), events.recv()).await {
            // Inspect one runtime event.
            Ok(Ok(event)) => {
                // Print worker-relevant runtime events.
                if is_worker_relevant_event(&event) {
                    // Print the runtime event.
                    println!("runtime event={event}");
                }
                // Stop once the child exit event has arrived.
                if event.starts_with("child_exit:") {
                    break;
                }
            }
            // Continue after broadcast lag.
            Ok(Err(broadcast::error::RecvError::Lagged(_count))) => {}
            // Stop when the channel closes or timeout occurs.
            Ok(Err(broadcast::error::RecvError::Closed)) | Err(_) => break,
        }
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
        // Print worker-relevant runtime events.
        if is_worker_relevant_event(&event) {
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

/// Prints one worker lifecycle fact.
///
/// # Arguments
///
/// - `label`: Output label attached to the fact.
/// - `event`: Worker lifecycle fact to print.
///
/// # Returns
///
/// This function does not return a value.
fn print_worker_event(label: &str, event: &WorkerEvent) {
    // Match the worker fact shape.
    match event {
        WorkerEvent::Initialized { child_id } => {
            // Print initialization details.
            println!("worker {label}: initialized child={child_id}");
        }
        WorkerEvent::Running { child_id, batch } => {
            // Print running details.
            println!("worker {label}: running child={child_id} batch={batch}");
        }
        WorkerEvent::Completed { child_id } => {
            // Print completion details.
            println!("worker {label}: completed child={child_id}");
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

/// Returns whether a runtime event is relevant to this worker example.
///
/// # Arguments
///
/// - `event`: Runtime event text.
///
/// # Returns
///
/// Returns `true` for command, child, shutdown, and runtime startup events.
fn is_worker_relevant_event(event: &str) -> bool {
    // Keep the event filter small and readable.
    event.starts_with("runtime_control_loop_started")
        || event.starts_with("control_command:")
        || event.starts_with("child_exit:")
        || event.starts_with("shutdown_")
}
