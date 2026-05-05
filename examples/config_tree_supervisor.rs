//! rust-config-tree(集中配置树) configuration(配置) example(示例).

// Import the YAML configuration loader.
use rust_supervisor::config::loader::load_config_state;

// Define the shared example result type.
type ExampleResult = Result<(), rust_supervisor::error::types::SupervisorError>;

// Return typed supervisor errors from the example.
fn main() -> ExampleResult {
    // Load centralized YAML configuration.
    let state = load_config_state("examples/config/supervisor.yaml")?;
    // Convert configuration into a supervisor specification.
    let spec = state.to_supervisor_spec()?;
    // Print the derived specification for inspection.
    println!("{spec:#?}");
    // Finish the example successfully.
    Ok(())
    // End the configuration example.
}
