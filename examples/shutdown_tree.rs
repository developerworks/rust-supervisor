//! Demonstrates the four-stage shutdown sequence.

// Import the YAML configuration loader.
use rust_supervisor::config::loader::load_config_from_yaml_file;
// Import the supervisor runtime entry point.
use rust_supervisor::runtime::supervisor::Supervisor;
// Import shutdown phase names for display.
use rust_supervisor::shutdown::stage::ShutdownPhase;

// Define the shared example result type.
type ExampleResult = Result<(), rust_supervisor::error::types::SupervisorError>;

// Use the Tokio runtime for the asynchronous example.
#[tokio::main]
// Return typed supervisor errors from the example.
/// Runs the shutdown tree example.
async fn main() -> ExampleResult {
    // Load centralized YAML configuration.
    let state = load_config_from_yaml_file("examples/config/supervisor.yaml")?;
    // Derive the supervisor specification from configuration.
    let spec = state.to_supervisor_spec()?;
    // Start the supervisor runtime from the specification.
    let handle = Supervisor::start(spec).await?;
    // Build the visible shutdown phase list.
    let phases = [
        // Show the stop request phase.
        ShutdownPhase::RequestStop,
        // Show the graceful drain phase.
        ShutdownPhase::GracefulDrain,
        // Show the abort stragglers phase.
        ShutdownPhase::AbortStragglers,
        // Show the final reconcile phase.
        ShutdownPhase::Reconcile,
        // Finish the shutdown phase list.
    ];
    // Iterate over the visible shutdown phases.
    for phase in phases {
        // Print each planned phase.
        println!("planned phase={phase:#?}");
        // Finish the phase display loop.
    }
    // Use the runtime handle for the shutdown request.
    handle
        // Request tree shutdown with audit metadata.
        .shutdown_tree("operator", "shutdown tree example")
        // Wait for the shutdown command result.
        .await?;
    // Finish the example successfully.
    Ok(())
    // End the shutdown example.
}
