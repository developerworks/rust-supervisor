//! Supervisor role child construction for the supervisor example.

// Import supervisor error values.
use rust_supervisor::error::types::SupervisorError;
// Import child identifiers.
use rust_supervisor::id::types::ChildId;
// Import supervisor task role defaults.
use rust_supervisor::policy::task_role_defaults::TaskRole;
// Import child specification values.
use rust_supervisor::spec::child::{ChildSpec, Criticality, TaskKind};
use rust_supervisor::spec::shutdown::ShutdownBudget;
// Import task context values.
use rust_supervisor::task::context::TaskContext;
// Import task factory helpers.
use rust_supervisor::task::factory::{TaskResult, service_fn};
// Import shared ownership for task factories.
use std::sync::Arc;
// Import time values for supervisor role work.
use std::time::{Duration, SystemTime, UNIX_EPOCH};
// Import asynchronous channel helpers.
use tokio::sync::mpsc;

/// Lifecycle fact emitted by the supervisor role example.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SupervisorEvent {
    /// The supervisor role unit has initialized.
    Initialized {
        /// Child identifier.
        child_id: String,
        /// Supervisor tree path for this attempt.
        path: String,
    },
    /// The supervisor role unit emitted one running tick.
    Running {
        /// Child identifier.
        child_id: String,
        /// Monotonic example tick number.
        tick: u64,
    },
    /// The supervisor role unit observed cancellation and is stopping cooperatively.
    Stopping {
        /// Child identifier.
        child_id: String,
    },
}

/// Builds the supervisor role child used by the example.
///
/// # Arguments
///
/// - `events`: Channel used to report supervisor role lifecycle facts to the example.
///
/// # Returns
///
/// Returns a [`ChildSpec`] whose `task_role` is [`TaskRole::Supervisor`].
pub fn supervisor_role_child(
    events: mpsc::UnboundedSender<SupervisorEvent>,
) -> Result<ChildSpec, SupervisorError> {
    // Build a task factory from the supervisor role function.
    let factory = service_fn(move |ctx: TaskContext| {
        // Clone the event sender for this attempt.
        let events = events.clone();
        // Run one supervisor role attempt.
        async move { run_supervisor_role_unit(ctx, events).await }
    });
    // Build a runnable child that is classified by TaskRole::Supervisor.
    let mut child = ChildSpec::worker(
        // Set the Child identifier.
        ChildId::new("nested-supervisor-unit"),
        // Set the display name.
        "Nested Supervisor Unit",
        // Select async worker execution so the role unit has a runnable body.
        TaskKind::AsyncWorker,
        // Store the factory behind shared ownership.
        Arc::new(factory),
    )?;
    // Classify the task as a supervisor role unit.
    child.task_role = Some(TaskRole::Supervisor);
    // Keep the supervisor role unit in the critical path.
    child.criticality = Criticality::Critical;
    // Add stable diagnostic tags.
    child.tags = vec!["supervisor".to_owned(), "nested".to_owned()];
    // Use short child shutdown budgets for a fast example.
    child.shutdown_budget =
        ShutdownBudget::new(Duration::from_millis(150), Duration::from_millis(50));
    // Return the supervisor role child declaration.
    Ok(child)
}

/// Runs one supervisor role attempt until cancellation arrives.
///
/// # Arguments
///
/// - `ctx`: Runtime context for the current child attempt.
/// - `events`: Channel used to report supervisor role lifecycle facts.
///
/// # Returns
///
/// Returns [`TaskResult::Cancelled`] after a cooperative stop.
async fn run_supervisor_role_unit(
    ctx: TaskContext,
    events: mpsc::UnboundedSender<SupervisorEvent>,
) -> TaskResult {
    // Mark readiness after initialization work finishes.
    ctx.mark_ready();
    // Emit an initial heartbeat for observers.
    ctx.heartbeat();
    // Publish the initialization fact.
    let _ignored = events.send(SupervisorEvent::Initialized {
        child_id: ctx.child_id.value.clone(),
        path: ctx.path.to_string(),
    });
    // Build a periodic running loop.
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    // Keep the cancellation token alive across select waits.
    let cancellation_token = ctx.cancellation_token();
    // Track running ticks for observable output.
    let mut tick = 0_u64;
    // Keep the role unit alive until cancellation.
    loop {
        // Wait for either cancellation or the next supervisor role tick.
        tokio::select! {
            // Stop cooperatively when the runtime cancels this attempt.
            _ = cancellation_token.cancelled() => {
                // Publish the stopping fact.
                let _ignored = events.send(SupervisorEvent::Stopping {
                    child_id: ctx.child_id.value.clone(),
                });
                // Report cooperative cancellation to the runtime.
                return TaskResult::Cancelled;
            }
            // Emit one running tick.
            _ = interval.tick() => {
                // Advance the tick counter.
                tick += 1;
                // Run one business tick for the supervisor role unit.
                run_supervisor_business(&ctx, &events, tick);
            }
        }
    }
}

/// Runs one supervisor role business tick.
///
/// # Arguments
///
/// - `ctx`: Runtime context for the current child attempt.
/// - `events`: Channel used to report supervisor role lifecycle facts.
/// - `tick`: Monotonic example tick number.
///
/// # Returns
///
/// This function does not return a value.
fn run_supervisor_business(
    ctx: &TaskContext,
    events: &mpsc::UnboundedSender<SupervisorEvent>,
    tick: u64,
) {
    // Read the current system time for the business output.
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    // Print the current time once for this business tick.
    println!(
        "supervisor business: child={} tick={} now_unix={}.{:09}",
        ctx.child_id,
        tick,
        now.as_secs(),
        now.subsec_nanos()
    );
    // Emit heartbeat for liveness observation.
    ctx.heartbeat();
    // Publish the running fact.
    let _ignored = events.send(SupervisorEvent::Running {
        child_id: ctx.child_id.value.clone(),
        tick,
    });
}
