//! Demonstrates step 01: creating an empty supervisor runtime.

// Import the supervisor runtime entry point.
use rust_supervisor::runtime::supervisor::Supervisor;
// Import supervisor specification values.
use rust_supervisor::spec::supervisor::SupervisorSpec;

// Define the shared example result type.
type ExampleResult = Result<(), rust_supervisor::error::types::SupervisorError>;

// Use the Tokio runtime for the asynchronous example.
#[tokio::main]
/// Runs the step 01 supervisor creation example.
async fn main() -> ExampleResult {
    // Build an empty root supervisor specification.
    let spec = SupervisorSpec::root(Vec::new());

    // Start the supervisor runtime from the root specification.
    let handle = Supervisor::start(spec).await?;

    // Query the current runtime state after startup.
    let current_state = handle.current_state().await?;

    // Print the empty supervisor runtime state.
    println!("step=01 supervisor_state={current_state:#?}");

    // Request shutdown for the empty supervisor tree.
    handle
        // Attach operator metadata to the shutdown command.
        .shutdown_tree("operator", "step 01 create supervisor complete")
        // Wait until the shutdown command finishes.
        .await?;

    // Query the current runtime state after shutdown.
    let current_state = handle.current_state().await?;

    // Print the stopped supervisor runtime state.
    println!("step=01 after_shutdown={current_state:#?}");
    // Finish the example successfully.
    Ok(())
}
