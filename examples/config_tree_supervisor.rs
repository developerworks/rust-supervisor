//! Demonstrates loading centralized YAML configuration into a supervisor spec.

// Import the YAML configuration loader.
use rust_supervisor::config::loader::load_config_from_yaml_file;

// Define the shared example result type.
type ExampleResult = Result<(), rust_supervisor::error::types::SupervisorError>;

// Return typed supervisor errors from the example.
/// Runs the centralized configuration example.
fn main() -> ExampleResult {
    // Load centralized YAML configuration.
    let state = load_config_from_yaml_file("examples/config/supervisor.yaml")?;
    // Convert configuration into a supervisor specification.
    let spec = state.to_supervisor_spec()?;
    // Print the derived specification for inspection.
    println!("{spec:#?}");
    // Finish the example successfully.
    Ok(())
    // End the configuration example.
}
