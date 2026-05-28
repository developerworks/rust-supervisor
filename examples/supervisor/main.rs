//! Demonstrates a supervisor role unit with initialization, running,
//! cooperative stop, and observation output.

mod observation;
mod supervisor_task;

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
/// Runs the supervisor role example.
async fn main() -> ExampleResult {
    // Build a channel that receives supervisor role lifecycle facts.
    let (supervisor_event_sender, mut supervisor_events) = mpsc::unbounded_channel();
    // Build one child declared as a supervisor role unit.
    let supervisor_child = supervisor_task::supervisor_role_child(supervisor_event_sender)?;
    // Build a root supervisor with the supervisor role child.
    let mut spec = SupervisorSpec::root(vec![supervisor_child]);
    // Keep enough event buffer for the full shutdown observation sequence.
    spec.event_channel_capacity = 32;
    // Use short shutdown windows so the example finishes quickly.
    let shutdown_policy = ShutdownPolicy::new(
        Duration::from_millis(250),
        Duration::from_millis(50),
        true,
        Duration::from_millis(250),
        3,
    );
    // Start the runtime with the supervisor role child.
    let handle = Supervisor::start_with_policy(spec, shutdown_policy).await?;
    // Subscribe to lifecycle event text before commands are sent.
    let mut runtime_events = handle.subscribe_events();
    // Wait until the supervisor role child reports initialization.
    observation::wait_for_initialization(&mut supervisor_events).await;
    // Print the state after initialization.
    observation::print_current_state("after-initialization", handle.current_state().await?);
    // Print the long-running operation hint.
    println!("supervisor example: running until Ctrl+C");
    // Build periodic observation ticks for the long-running supervisor role unit.
    let mut observation_interval = tokio::time::interval(Duration::from_secs(1));
    // Keep the supervisor role unit running until the operator requests shutdown.
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
            // Print periodic supervisor role observation.
            _ = observation_interval.tick() => {
                // Print supervisor role facts emitted while the unit is running.
                observation::drain_supervisor_events("while-running", &mut supervisor_events);
                // Print the current runtime state.
                observation::print_current_state("while-running", handle.current_state().await?);
                // Print relevant runtime events that arrived during the tick.
                observation::drain_runtime_events(&mut runtime_events);
            }
        }
    }
    // Request cooperative shutdown for the whole supervisor tree.
    let shutdown = handle
        .shutdown_tree("operator", "supervisor role example stopped by operator")
        .await?;
    // Print supervisor role facts emitted during cooperative stop.
    observation::drain_supervisor_events("during-stop", &mut supervisor_events);
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
    // End the supervisor role example.
}
