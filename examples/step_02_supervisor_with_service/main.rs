//! Demonstrates step 02: mounting one service under a supervisor runtime.

// Import supervisor error values.
use rust_supervisor::error::types::SupervisorError;
// Import child identifiers.
use rust_supervisor::id::types::ChildId;
// Import the supervisor runtime entry point.
use rust_supervisor::runtime::supervisor::Supervisor;
// Import child specification values.
use rust_supervisor::spec::child::{ChildSpec, TaskKind};
// Import child specification builder values.
use rust_supervisor::spec::child_builder::ChildSpecBuilder;
// Import supervisor specification values.
use rust_supervisor::spec::supervisor::SupervisorSpec;
// Import task context values.
use rust_supervisor::task::context::TaskContext;
// Import task factory helpers.
use rust_supervisor::task::factory::{TaskResult, service_fn};
// Import shared ownership for task factories.
use std::sync::Arc;
// Import asynchronous channel helpers.
use tokio::sync::mpsc;

// Define the shared example result type.
type ExampleResult = Result<(), rust_supervisor::error::types::SupervisorError>;

// Use the Tokio runtime for the asynchronous example.
#[tokio::main]
/// Runs the step 02 supervisor with service example.
async fn main() -> ExampleResult {
    // Build a channel that receives service lifecycle facts.
    let (service_event_sender, mut service_events) = mpsc::unbounded_channel();
    // Build one service child before creating the supervisor tree.
    let service_child = service_child(service_event_sender)?;
    // Mount the service child under the root supervisor specification.
    let spec = SupervisorSpec::root(vec![service_child]);
    // Start the supervisor runtime with the mounted service.
    let handle = Supervisor::start(spec).await?;
    // Wait until the service reports readiness.
    let initialized = service_events.recv().await.ok_or_else(|| {
        // Convert a closed channel into the example error type.
        SupervisorError::fatal_config("service initialization channel closed")
    })?;
    // Print the service initialization fact.
    println!("step=02 {initialized}");
    // Query the current runtime state after service startup.
    let current_state = handle.current_state().await?;
    // Print the supervisor state that contains the service child.
    println!("step=02 supervisor_state={current_state:#?}");
    // Request shutdown for the supervisor tree and mounted service.
    handle
        // Attach operator metadata to the shutdown command.
        .shutdown_tree("operator", "step 02 supervisor with service complete")
        // Wait until the shutdown command finishes.
        .await?;
    // Drain service facts emitted during cooperative shutdown.
    drain_service_events(&mut service_events);
    // Query the current runtime state after shutdown.
    let current_state = handle.current_state().await?;
    // Print the stopped supervisor runtime state.
    println!("step=02 after_shutdown={current_state:#?}");
    // Finish the example successfully.
    Ok(())
}

/// Builds the service child mounted under the supervisor.
///
/// # Arguments
///
/// - `events`: Channel used to publish service lifecycle facts.
///
/// # Returns
///
/// Returns a validated service [`ChildSpec`].
fn service_child(events: mpsc::UnboundedSender<String>) -> Result<ChildSpec, SupervisorError> {
    // Build a task factory from the service function.
    let factory = service_fn(move |ctx: TaskContext| {
        // Clone the event sender for this service attempt.
        let events = events.clone();
        // Run one service attempt.
        async move { run_service(ctx, events).await }
    });
    // Build a service child and finish construction with `build`.
    ChildSpecBuilder::service(
        // Set the stable child identifier.
        ChildId::new("step-02-service"),
        // Set the display name.
        "Step 02 Service",
        // Select async worker execution.
        TaskKind::AsyncWorker,
        // Store the factory behind shared ownership.
        Arc::new(factory),
    )
    // Add a diagnostic tag.
    .tag("step-02")
    // Validate and return the final child specification.
    .build()
}

/// Runs one service attempt until supervisor shutdown cancels it.
///
/// # Arguments
///
/// - `ctx`: Runtime context for the current child attempt.
/// - `events`: Channel used to publish service lifecycle facts.
///
/// # Returns
///
/// Returns [`TaskResult::Cancelled`] after cooperative shutdown.
async fn run_service(ctx: TaskContext, events: mpsc::UnboundedSender<String>) -> TaskResult {
    // Mark the service as ready for the supervisor.
    ctx.mark_ready();
    // Emit a heartbeat for liveness observation.
    ctx.heartbeat();
    // Publish the service initialization fact.
    let _ignored = events.send(format!("service initialized: child={}", ctx.child_id));
    // Keep a cancellation token for the wait below.
    let cancellation_token = ctx.cancellation_token();
    // Wait until the supervisor asks this service attempt to stop.
    cancellation_token.cancelled().await;
    // Publish the service stopping fact.
    let _ignored = events.send(format!("service stopping: child={}", ctx.child_id));
    // Report cooperative cancellation to the supervisor runtime.
    TaskResult::Cancelled
}

/// Drains service lifecycle facts that are already available.
///
/// # Arguments
///
/// - `events`: Receiver for service lifecycle facts.
///
/// # Returns
///
/// This function does not return a value.
fn drain_service_events(events: &mut mpsc::UnboundedReceiver<String>) {
    // Drain all service facts that arrived before this call.
    while let Ok(event) = events.try_recv() {
        // Print one service lifecycle fact.
        println!("step=02 {event}");
    }
}
