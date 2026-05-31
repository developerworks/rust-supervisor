//! Service role child construction for the service example.

// Import supervisor error values.
use rust_supervisor::error::types::SupervisorError;
// Import child identifiers.
use rust_supervisor::id::types::ChildId;
// Import service task role defaults.
use rust_supervisor::policy::task_role_defaults::TaskRole;
// Import child specification values.
use rust_supervisor::spec::child::{ChildSpec, Criticality, ShutdownPolicy, TaskKind};
// Import task context values.
use rust_supervisor::task::context::TaskContext;
// Import task factory helpers.
use rust_supervisor::task::factory::{TaskResult, service_fn};
// Import shared ownership for task factories.
use std::sync::Arc;
// Import time values for service work.
use std::time::{Duration, SystemTime, UNIX_EPOCH};
// Import asynchronous channel helpers.
use tokio::sync::mpsc;

/// Lifecycle fact emitted by the example service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceEvent {
    /// The service has created its runtime attempt and marked readiness.
    Initialized {
        /// Stable child identifier.
        child_id: String,
        /// Supervisor tree path for this attempt.
        path: String,
    },
    /// The service has emitted one running tick.
    Running {
        /// Stable child identifier.
        child_id: String,
        /// Monotonic example tick number.
        tick: u64,
    },
    /// The service observed cancellation and is stopping cooperatively.
    Stopping {
        /// Stable child identifier.
        child_id: String,
    },
}

/// Builds the service role child used by the example.
///
/// # Arguments
///
/// - `events`: Channel used to report service lifecycle facts to the example.
///
/// # Returns
///
/// Returns a [`ChildSpec`] whose `task_role` is [`TaskRole::Service`].
pub fn service_child(
    events: mpsc::UnboundedSender<ServiceEvent>,
) -> Result<ChildSpec, SupervisorError> {
    // Build a task factory from the service function.
    let factory = service_fn(move |ctx: TaskContext| {
        // Clone the event sender for this attempt.
        let events = events.clone();
        // Run one service attempt.
        async move { run_service(ctx, events).await }
    });
    // Build a base async worker child.
    let mut child = ChildSpec::worker(
        // Set the stable child identifier.
        ChildId::new("quote-service"),
        // Set the display name.
        "Quote Service",
        // Select async worker execution.
        TaskKind::AsyncWorker,
        // Store the factory behind shared ownership.
        Arc::new(factory),
    )?;
    // Classify the task as a long-running service.
    child.task_role = Some(TaskRole::Service);
    // Keep the service in the critical path.
    child.criticality = Criticality::Critical;
    // Add stable diagnostic tags.
    child.tags = vec!["service".to_owned(), "quotes".to_owned()];
    // Use short child shutdown budgets for a fast example.
    child.shutdown_policy =
        ShutdownPolicy::new(Duration::from_millis(150), Duration::from_millis(50));
    // Return the service child declaration.
    Ok(child)
}

/// Runs one service attempt until cancellation arrives.
///
/// # Arguments
///
/// - `ctx`: Runtime context for the current child attempt.
/// - `events`: Channel used to report service lifecycle facts.
///
/// # Returns
///
/// Returns [`TaskResult::Cancelled`] after a cooperative stop.
async fn run_service(ctx: TaskContext, events: mpsc::UnboundedSender<ServiceEvent>) -> TaskResult {
    // Mark readiness after initialization work finishes.
    ctx.mark_ready();
    // Emit an initial heartbeat for observers.
    ctx.heartbeat();
    // Publish the initialization fact.
    let _ignored = events.send(ServiceEvent::Initialized {
        child_id: ctx.child_id.value.clone(),
        path: ctx.path.to_string(),
    });
    // Build a periodic running loop.
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    // Keep the cancellation token alive across select waits.
    let cancellation_token = ctx.cancellation_token();
    // Track running ticks for observable output.
    let mut tick = 0_u64;
    // Keep the service alive until cancellation.
    loop {
        // Wait for either cancellation or the next service tick.
        tokio::select! {
            // Stop cooperatively when the runtime cancels this attempt.
            _ = cancellation_token.cancelled() => {
                // Publish the stopping fact.
                let _ignored = events.send(ServiceEvent::Stopping {
                    child_id: ctx.child_id.value.clone(),
                });
                // Report cooperative cancellation to the runtime.
                return TaskResult::Cancelled;
            }
            // Emit one running tick.
            _ = interval.tick() => {
                // Advance the tick counter.
                tick += 1;
                // Run one business tick for the service.
                run_quote_service_business(&ctx, &events, tick);
            }
        }
    }
}

/// Runs one quote service business tick.
///
/// # Arguments
///
/// - `ctx`: Runtime context for the current child attempt.
/// - `events`: Channel used to report service lifecycle facts.
/// - `tick`: Monotonic example tick number.
///
/// # Returns
///
/// This function does not return a value.
fn run_quote_service_business(
    ctx: &TaskContext,
    events: &mpsc::UnboundedSender<ServiceEvent>,
    tick: u64,
) {
    // Read the current system time for the business output.
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    // Print the current time once for this business tick.
    println!(
        "service business: child={} tick={} now_unix={}.{:09}",
        ctx.child_id,
        tick,
        now.as_secs(),
        now.subsec_nanos()
    );
    // Emit heartbeat for liveness observation.
    ctx.heartbeat();
    // Publish the running fact.
    let _ignored = events.send(ServiceEvent::Running {
        child_id: ctx.child_id.value.clone(),
        tick,
    });
}
