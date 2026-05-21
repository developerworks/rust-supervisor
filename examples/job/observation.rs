//! Observation helpers for the job role example.

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
// Import job lifecycle facts.
use crate::job_task::JobEvent;

/// Waits for the job initialization fact and prints it.
///
/// # Arguments
///
/// - `events`: Receiver for job lifecycle facts.
///
/// # Returns
///
/// This function returns after the initialized event appears or the channel closes.
pub async fn wait_for_initialization(events: &mut tokio::sync::mpsc::UnboundedReceiver<JobEvent>) {
    // Read job facts until initialization appears.
    while let Some(event) = events.recv().await {
        // Print the job fact.
        print_job_event("initialization", &event);
        // Stop waiting once initialization is observed.
        if matches!(event, JobEvent::Initialized { .. }) {
            break;
        }
    }
}

/// Waits for job completion while printing job facts.
///
/// # Arguments
///
/// - `events`: Receiver for job lifecycle facts.
///
/// # Returns
///
/// This function returns after completion appears or the channel closes.
pub async fn wait_for_completion(events: &mut tokio::sync::mpsc::UnboundedReceiver<JobEvent>) {
    // Read job facts until completion appears.
    while let Some(event) = events.recv().await {
        // Print the job fact.
        print_job_event("one-shot-work", &event);
        // Stop waiting once completion is observed.
        if matches!(event, JobEvent::Completed { .. }) {
            break;
        }
    }
}

/// Waits until the runtime reports the job child exit.
///
/// # Arguments
///
/// - `events`: Runtime event receiver.
///
/// # Returns
///
/// This function returns after the child exit event appears, the channel closes, or timeout occurs.
pub async fn wait_for_child_exit(events: &mut broadcast::Receiver<String>) {
    // Wait for the runtime to process the job exit.
    loop {
        // Bound the wait so the example does not hang if event delivery changes.
        match timeout(Duration::from_secs(1), events.recv()).await {
            // Inspect one runtime event.
            Ok(Ok(event)) => {
                // Print job-relevant runtime events.
                if is_job_relevant_event(&event) {
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
        // Print job-relevant runtime events.
        if is_job_relevant_event(&event) {
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

/// Prints one job lifecycle fact.
fn print_job_event(label: &str, event: &JobEvent) {
    // Match the job fact shape.
    match event {
        JobEvent::Initialized { child_id } => {
            // Print initialization details.
            println!("job {label}: initialized child={child_id}");
        }
        JobEvent::Running { child_id } => {
            // Print running details.
            println!("job {label}: running child={child_id}");
        }
        JobEvent::Completed { child_id } => {
            // Print completion details.
            println!("job {label}: completed child={child_id}");
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

/// Returns whether a runtime event is relevant to this job example.
fn is_job_relevant_event(event: &str) -> bool {
    // Keep the event filter small and readable.
    event.starts_with("runtime_control_loop_started")
        || event.starts_with("control_command:")
        || event.starts_with("child_exit:")
        || event.starts_with("shutdown_")
}
