//! Demonstrates a supervised worker role with bounded work and observation.

mod observation;
mod worker_task;

// Import command result variants returned by the runtime handle.
use rust_supervisor::control::command::CommandResult;
// Import the supervisor runtime entry point.
use rust_supervisor::runtime::supervisor::Supervisor;
// Import supervisor specification values.
use rust_supervisor::spec::supervisor::SupervisorSpec;
// Import shutdown timing policy.
use rust_supervisor::shutdown::stage::ShutdownPolicy;
// Import duration values for the example timing budget.
use std::time::Duration;
// Import asynchronous channel helpers.
use tokio::sync::mpsc;

// Define the shared example result type.
type ExampleResult = Result<(), rust_supervisor::error::types::SupervisorError>;

// Use the Tokio runtime for the asynchronous example.
#[tokio::main]
// Return typed supervisor errors from the example.
/// Runs the worker role example.
async fn main() -> ExampleResult {
    // Build a channel that receives worker lifecycle facts.
    let (worker_event_sender, mut worker_events) = mpsc::unbounded_channel();
    // Build one child declared as a worker role.
    let worker_child = worker_task::worker_child(worker_event_sender)?;
    // Build a root supervisor with the worker child.
    let mut spec = SupervisorSpec::root(vec![worker_child]);
    // Keep enough event buffer for the shutdown observation sequence.
    spec.event_channel_capacity = 32;
    // Use short shutdown windows so the cleanup path finishes quickly.
    let shutdown_policy = ShutdownPolicy::new(
        Duration::from_millis(250),
        Duration::from_millis(50),
        true,
        Duration::from_millis(250),
        3,
    );
    // Start the runtime with the worker child.
    let handle = Supervisor::start_with_policy(spec, shutdown_policy).await?;
    // Subscribe to lifecycle event text before commands are sent.
    let mut runtime_events = handle.subscribe_events();
    // Wait until the worker reports initialization.
    observation::wait_for_initialization(&mut worker_events).await;
    // Print the state after initialization.
    observation::print_current_state("after-initialization", handle.current_state().await?);
    // Wait until the bounded worker completes.
    observation::wait_for_completion(&mut worker_events).await;
    // Wait until the runtime records the worker exit.
    observation::wait_for_child_exit(&mut runtime_events).await;
    // Print the state after the worker has completed.
    observation::print_current_state("after-worker-complete", handle.current_state().await?);
    // Shut down the supervisor tree after the worker has stopped itself.
    let shutdown = handle
        .shutdown_tree("operator", "worker role example cleanup")
        .await?;
    // Print shutdown events.
    observation::drain_runtime_events(&mut runtime_events);
    // Print the shutdown result.
    if let CommandResult::Shutdown { result } = shutdown {
        // Print shutdown details.
        observation::print_shutdown_result(result);
    }
    // Print the state after shutdown has completed.
    observation::print_current_state("after-shutdown", handle.current_state().await?);
    // Finish the example successfully.
    Ok(())
    // End the worker role example.
}
