//! Demonstrates a supervised job role with one-shot work and observation.

mod job_task;
mod observation;

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
/// Runs the job role example.
async fn main() -> ExampleResult {
    // Build a channel that receives job lifecycle facts.
    let (job_event_sender, mut job_events) = mpsc::unbounded_channel();
    // Build one child declared as a job role.
    let job_child = job_task::job_child(job_event_sender)?;
    // Build a root supervisor with the job child.
    let mut spec = SupervisorSpec::root(vec![job_child]);
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
    // Start the runtime with the job child.
    let handle = Supervisor::start_with_policy(spec, shutdown_policy).await?;
    // Subscribe to lifecycle event text before commands are sent.
    let mut runtime_events = handle.subscribe_events();
    // Wait until the job reports initialization.
    observation::wait_for_initialization(&mut job_events).await;
    // Print the state after initialization.
    observation::print_current_state("after-initialization", handle.current_state().await?);
    // Wait until the one-shot job completes.
    observation::wait_for_completion(&mut job_events).await;
    // Wait until the runtime records the job exit.
    observation::wait_for_child_exit(&mut runtime_events).await;
    // Print the state after the job has completed.
    observation::print_current_state("after-job-complete", handle.current_state().await?);
    // Shut down the supervisor tree after the job has stopped itself.
    let shutdown = handle
        .shutdown_tree("operator", "job role example cleanup")
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
    // End the job role example.
}
