//! Sidecar role child construction for the sidecar example.

// Import supervisor error values.
use rust_supervisor::error::types::SupervisorError;
// Import child identifiers.
use rust_supervisor::id::types::ChildId;
// Import task role defaults and sidecar configuration.
use rust_supervisor::policy::task_role_defaults::{SidecarConfig, TaskRole};
// Import child specification values.
use rust_supervisor::spec::child::{ChildSpec, Criticality, ShutdownPolicy, TaskKind};
// Import task context values.
use rust_supervisor::task::context::TaskContext;
// Import task factory helpers.
use rust_supervisor::task::factory::{TaskResult, service_fn};
// Import shared ownership for task factories.
use std::sync::Arc;
// Import time values for sidecar work.
use std::time::{Duration, SystemTime, UNIX_EPOCH};
// Import asynchronous channel helpers.
use tokio::sync::mpsc;

/// Lifecycle fact emitted by the sidecar example.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidecarEvent {
    /// A child has initialized.
    Initialized {
        /// Stable child identifier.
        child_id: String,
        /// Role label for the child.
        role: String,
    },
    /// A child emitted one running tick.
    Running {
        /// Stable child identifier.
        child_id: String,
        /// Role label for the child.
        role: String,
        /// Monotonic example tick number.
        tick: u64,
    },
    /// A child observed cancellation and is stopping cooperatively.
    Stopping {
        /// Stable child identifier.
        child_id: String,
        /// Role label for the child.
        role: String,
    },
}

/// Builds the primary service child used by the sidecar example.
///
/// # Arguments
///
/// - `events`: Channel used to report lifecycle facts to the example.
///
/// # Returns
///
/// Returns a primary service [`ChildSpec`].
pub fn primary_service_child(
    events: mpsc::UnboundedSender<SidecarEvent>,
) -> Result<ChildSpec, SupervisorError> {
    // Build a task factory from the primary service function.
    let factory = service_fn(move |ctx: TaskContext| {
        // Clone the event sender for this attempt.
        let events = events.clone();
        // Run one primary service attempt.
        async move { run_long_lived_child(ctx, events, "primary-service").await }
    });
    // Build a base async worker child.
    let mut child = ChildSpec::worker(
        // Set the stable child identifier.
        primary_child_id(),
        // Set the display name.
        "API Service",
        // Select async worker execution.
        TaskKind::AsyncWorker,
        // Store the factory behind shared ownership.
        Arc::new(factory),
    )?;
    // Classify the primary as a long-running service.
    child.task_role = Some(TaskRole::Service);
    // Keep the primary in the critical path.
    child.criticality = Criticality::Critical;
    // Add stable diagnostic tags.
    child.tags = vec!["service".to_owned(), "api".to_owned()];
    // Use short child shutdown budgets for a fast example.
    child.shutdown_policy =
        ShutdownPolicy::new(Duration::from_millis(150), Duration::from_millis(50));
    // Return the primary child declaration.
    Ok(child)
}

/// Builds the sidecar child attached to the primary service.
///
/// # Arguments
///
/// - `events`: Channel used to report lifecycle facts to the example.
///
/// # Returns
///
/// Returns a [`ChildSpec`] whose `task_role` is [`TaskRole::Sidecar`].
pub fn sidecar_child(
    events: mpsc::UnboundedSender<SidecarEvent>,
) -> Result<ChildSpec, SupervisorError> {
    // Build a task factory from the sidecar function.
    let factory = service_fn(move |ctx: TaskContext| {
        // Clone the event sender for this attempt.
        let events = events.clone();
        // Run one sidecar attempt.
        async move { run_long_lived_child(ctx, events, "metrics-sidecar").await }
    });
    // Build a base async worker child.
    let mut child = ChildSpec::worker(
        // Set the stable child identifier.
        ChildId::new("metrics-sidecar"),
        // Set the display name.
        "Metrics Sidecar",
        // Select async worker execution.
        TaskKind::AsyncWorker,
        // Store the factory behind shared ownership.
        Arc::new(factory),
    )?;
    // Classify the task as a sidecar.
    child.task_role = Some(TaskRole::Sidecar);
    // Attach the sidecar to the primary service.
    child.sidecar_config = Some(SidecarConfig::new(primary_child_id(), true));
    // Require the primary service before the sidecar starts.
    child.dependencies = vec![primary_child_id()];
    // Sidecars are important but subordinate to the primary service.
    child.criticality = Criticality::Critical;
    // Add stable diagnostic tags.
    child.tags = vec!["sidecar".to_owned(), "metrics".to_owned()];
    // Use short child shutdown budgets for a fast example.
    child.shutdown_policy =
        ShutdownPolicy::new(Duration::from_millis(150), Duration::from_millis(50));
    // Return the sidecar child declaration.
    Ok(child)
}

/// Returns the primary service child identifier.
///
/// # Arguments
///
/// This function has no arguments.
///
/// # Returns
///
/// Returns the primary service [`ChildId`].
fn primary_child_id() -> ChildId {
    // Return a stable primary child identifier.
    ChildId::new("api-service")
}

/// Runs one long-lived child attempt.
///
/// # Arguments
///
/// - `ctx`: Runtime context for the current child attempt.
/// - `events`: Channel used to report lifecycle facts.
/// - `role`: Role label printed by the example.
///
/// # Returns
///
/// Returns [`TaskResult::Cancelled`] after a cooperative stop.
async fn run_long_lived_child(
    ctx: TaskContext,
    events: mpsc::UnboundedSender<SidecarEvent>,
    role: &'static str,
) -> TaskResult {
    // Mark readiness after initialization work finishes.
    ctx.mark_ready();
    // Emit an initial heartbeat for observers.
    ctx.heartbeat();
    // Publish the initialization fact.
    let _ignored = events.send(SidecarEvent::Initialized {
        child_id: ctx.child_id.value.clone(),
        role: role.to_owned(),
    });
    // Build a periodic running loop.
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    // Keep the cancellation token alive across select waits.
    let cancellation_token = ctx.cancellation_token();
    // Track running ticks for observable output.
    let mut tick = 0_u64;
    // Keep the child alive until cancellation.
    loop {
        // Wait for either cancellation or the next child tick.
        tokio::select! {
            // Stop cooperatively when the runtime cancels this attempt.
            _ = cancellation_token.cancelled() => {
                // Publish the stopping fact.
                let _ignored = events.send(SidecarEvent::Stopping {
                    child_id: ctx.child_id.value.clone(),
                    role: role.to_owned(),
                });
                // Report cooperative cancellation to the runtime.
                return TaskResult::Cancelled;
            }
            // Emit one running tick.
            _ = interval.tick() => {
                // Advance the tick counter.
                tick += 1;
                // Run one business tick for this child.
                run_sidecar_business(&ctx, &events, role, tick);
            }
        }
    }
}

/// Runs one sidecar example business tick.
///
/// # Arguments
///
/// - `ctx`: Runtime context for the current child attempt.
/// - `events`: Channel used to report lifecycle facts.
/// - `role`: Role label printed by the example.
/// - `tick`: Monotonic example tick number.
///
/// # Returns
///
/// This function does not return a value.
fn run_sidecar_business(
    ctx: &TaskContext,
    events: &mpsc::UnboundedSender<SidecarEvent>,
    role: &'static str,
    tick: u64,
) {
    // Read the current system time for the business output.
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    // Print the current tick time.
    println!(
        "sidecar business: role={} child={} tick={} now_unix={}.{:09}",
        role,
        ctx.child_id,
        tick,
        now.as_secs(),
        now.subsec_nanos()
    );
    // Emit heartbeat for liveness observation.
    ctx.heartbeat();
    // Publish the running fact.
    let _ignored = events.send(SidecarEvent::Running {
        child_id: ctx.child_id.value.clone(),
        role: role.to_owned(),
        tick,
    });
}
