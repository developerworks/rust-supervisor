//! Demonstrates observability probing against a running supervisor.

// Import the YAML configuration loader.
use rust_supervisor::config::loader::load_config_state;
// Import the supervisor runtime entry point.
use rust_supervisor::runtime::supervisor::Supervisor;

// Define the shared example result type.
type ExampleResult = Result<(), rust_supervisor::error::types::SupervisorError>;

// Use the Tokio runtime for the asynchronous example.
#[tokio::main]
// Return typed supervisor errors from the example.
/// Runs the observability probe example.
async fn main() -> ExampleResult {
    // Load centralized YAML configuration.
    let state = load_config_state("examples/config/supervisor.yaml")?;
    // Derive the supervisor specification from configuration.
    let spec = state.to_supervisor_spec()?;
    // Start the supervisor runtime from the specification.
    let handle = Supervisor::start(spec).await?;
    // Subscribe to runtime event text.
    let mut events = handle.subscribe_events();
    // Query the current runtime state.
    let current = handle.current_state().await?;
    // Print the current state for the learner.
    println!("current={current:#?}");
    // Read one emitted runtime event when it is available.
    let received = events.recv().await;
    // Continue only when an event was received.
    if let Ok(event) = received {
        // Print the observed runtime event.
        println!("event={event:#?}");
        // Finish the optional event display.
    }
    // Use the runtime handle for the shutdown request.
    handle
        // Request tree shutdown with audit metadata.
        .shutdown_tree("operator", "observability probe complete")
        // Wait for the shutdown command result.
        .await?;
    // Finish the example successfully.
    Ok(())
    // End the observability example.
}
