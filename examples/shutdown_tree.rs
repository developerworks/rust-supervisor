//! Demonstrates the four-stage shutdown sequence.

// Import command result variants returned by the runtime handle.
use rust_supervisor::control::command::CommandResult;
// Import the YAML configuration loader.
use rust_supervisor::config::loader::load_config_from_yaml_file;
// Import the supervisor runtime entry point.
use rust_supervisor::runtime::supervisor::Supervisor;

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
    let supervisor_handle = Supervisor::start(spec).await?;
    // Subscribe before shutdown so every phase transition is captured.
    let mut events = supervisor_handle.subscribe_events();
    // Use the runtime supervisor handle for the shutdown request.
    let shutdown = supervisor_handle
        // Request tree shutdown with audit metadata.
        .shutdown_tree("operator", "shutdown tree example")
        // Wait for the shutdown command result.
        .await?;
    // Drain emitted shutdown phase events.
    while let Ok(event) = events.try_recv() {
        // Print only shutdown phase transitions.
        if let Some(transition) = shutdown_phase_transition(&event) {
            // Show the real phase transition observed from the runtime event stream.
            println!("phase transition={transition}");
        }
    }
    // Print the actual shutdown phase returned by the runtime.
    if let CommandResult::Shutdown { result } = shutdown {
        // Show the completed top-level shutdown phase.
        println!("shutdown phase={:#?}", result.phase);
        // Show the pipeline report phase when the runtime returned one.
        if let Some(report) = result.report {
            // Print the real report phase and child outcome count.
            println!(
                "report phase={:#?}, child outcomes={}",
                report.phase,
                report.outcomes.len()
            );
        }
    }
    // Finish the example successfully.
    Ok(())
    // End the shutdown example.
}

// Parse a shutdown phase event into a displayable transition.
/// Returns a displayable shutdown phase transition.
fn shutdown_phase_transition(event: &str) -> Option<String> {
    // Read the event kind.
    let mut parts = event.split(':');
    // Keep only shutdown phase events.
    if parts.next()? != "shutdown_phase_changed" {
        return None;
    }
    // Read the previous phase.
    let from = parts.next()?;
    // Read the next phase.
    let to = parts.next()?;
    // Return a concise transition label.
    Some(format!("{from} -> {to}"))
}
