//! Demonstrates a sidecar role attached to a primary service.

mod observation;
mod sidecar_task;

// Import command result variants returned by the runtime handle.
use rust_supervisor::control::command::CommandResult;
// Import supervisor error values.
use rust_supervisor::error::types::SupervisorError;
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
/// Runs the sidecar role example.
async fn main() -> ExampleResult {
    // Build a channel that receives sidecar example lifecycle facts.
    let (event_sender, mut role_events) = mpsc::unbounded_channel();
    // Build the primary service and its attached sidecar.
    let primary = sidecar_task::primary_service_child(event_sender.clone());
    // Build the sidecar child with an attachment to the primary service.
    let sidecar = sidecar_task::sidecar_child(event_sender);
    // Build a root supervisor with both children.
    let mut spec = SupervisorSpec::root(vec![primary, sidecar]);
    // Keep enough event buffer for the full shutdown observation sequence.
    spec.event_channel_capacity = 64;
    // Use short shutdown windows so the example finishes quickly.
    let shutdown_policy = ShutdownPolicy::new(
        Duration::from_millis(250),
        Duration::from_millis(50),
        true,
        Duration::from_millis(250),
        3,
    );
    // Start the runtime with the primary service and sidecar.
    let handle = Supervisor::start_with_policy(spec, shutdown_policy).await?;
    // Subscribe to lifecycle event text before commands are sent.
    let mut runtime_events = handle.subscribe_events();
    // Wait until both children report initialization.
    observation::wait_for_initializations(&mut role_events, 2).await;
    // Print the state after both children initialize.
    observation::print_current_state("after-initialization", handle.current_state().await?);
    // Print the long-running operation hint.
    println!("sidecar example: running until Ctrl+C");
    // Build periodic observation ticks for both children.
    let mut observation_interval = tokio::time::interval(Duration::from_secs(1));
    // Keep both children running until the operator requests shutdown.
    loop {
        // Wait for either an operator signal or the next observation tick.
        tokio::select! {
            // Stop the example only when the operator sends Ctrl+C.
            signal = tokio::signal::ctrl_c() => {
                // Convert signal errors into the example error type.
                signal.map_err(|error| {
                    SupervisorError::fatal_config(format!(
                        "failed to receive Ctrl+C signal: {error}"
                    ))
                })?;
                // Print the operator stop signal.
                println!("operator signal=ctrl_c");
                // Leave the persistent running loop.
                break;
            }
            // Print periodic sidecar observation.
            _ = observation_interval.tick() => {
                // Print child facts emitted while both children are running.
                observation::drain_role_events("while-running", &mut role_events);
                // Print the current runtime state.
                observation::print_current_state("while-running", handle.current_state().await?);
                // Print relevant runtime events that arrived during the tick.
                observation::drain_runtime_events(&mut runtime_events);
            }
        }
    }
    // Request cooperative shutdown for the whole supervisor tree.
    let shutdown = handle
        .shutdown_tree("operator", "sidecar role example stopped by operator")
        .await?;
    // Print child facts emitted during cooperative stop.
    observation::drain_role_events("during-stop", &mut role_events);
    // Print runtime events that show command and shutdown observation.
    observation::drain_runtime_events(&mut runtime_events);
    // Print the shutdown result and per-child outcome.
    if let CommandResult::Shutdown { result } = shutdown {
        // Print the completed shutdown phase.
        observation::print_shutdown_result(result);
    }
    // Print the state after shutdown has completed.
    observation::print_current_state("after-shutdown", handle.current_state().await?);
    // Finish the example successfully.
    Ok(())
    // End the sidecar role example.
}
