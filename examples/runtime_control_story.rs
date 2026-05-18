//! Demonstrates an operator control flow against a running supervisor.

// Import the YAML configuration loader.
use rust_supervisor::config::loader::load_config_from_yaml_file;
// Import the command result type.
use rust_supervisor::control::command::CommandResult;
// Import child identifiers and supervisor paths.
use rust_supervisor::id::types::{ChildId, SupervisorPath};
// Import the supervisor runtime entry point.
use rust_supervisor::runtime::supervisor::Supervisor;

// Define the shared example result type.
type ExampleResult = Result<(), rust_supervisor::error::types::SupervisorError>;

// Use the Tokio runtime for the asynchronous example.
#[tokio::main]
// Return typed supervisor errors from the example.
/// Runs the runtime control story example.
async fn main() -> ExampleResult {
    // Load centralized YAML configuration.
    let state = load_config_from_yaml_file("examples/config/supervisor.yaml")?;
    // Derive the supervisor specification from configuration.
    let spec = state.to_supervisor_spec()?;
    // Start the supervisor runtime from the specification.
    let handle = Supervisor::start(spec).await?;
    // Subscribe to runtime event text.
    let mut events = handle.subscribe_events();
    // Build the child identifier used by operator commands.
    let child_id = ChildId::new("market_feed");

    // Send an add child command through the control handle.
    let add = handle
        // Add a manifest under the root supervisor.
        .add_child(
            // Target the root supervisor.
            SupervisorPath::root(),
            // Provide the child manifest text.
            "id=market_feed kind=AsyncWorker readiness=Explicit",
            // Provide the requesting actor.
            "operator",
            // Provide the audit reason.
            "attach market feed during incident rehearsal",
            // Finish the add child arguments.
        )
        // Wait for the add child command result.
        .await?;
    // Print the add child result.
    print_result("add_child", add);

    // Print the pause child result.
    print_result(
        // Label the pause result.
        "pause_child",
        // Send the pause child command.
        handle
            // Pause automatic governance.
            .pause_child(child_id.clone(), "operator", "stop automatic restart")
            // Wait for the pause result.
            .await?,
        // Finish the pause result print call.
    );
    // Print the resume child result.
    print_result(
        // Label the resume result.
        "resume_child",
        // Send the resume child command.
        handle
            // Resume lifecycle governance.
            .resume_child(child_id.clone(), "operator", "resume lifecycle governance")
            // Wait for the resume result.
            .await?,
        // Finish the resume result print call.
    );
    // Print the quarantine child result.
    print_result(
        // Label the quarantine result.
        "quarantine_child",
        // Send the quarantine child command.
        handle
            // Quarantine the child for manual investigation.
            .quarantine_child(child_id, "operator", "manual investigation")
            // Wait for the quarantine result.
            .await?,
        // Finish the quarantine result print call.
    );
    // Print the current state result.
    print_result("current_state", handle.current_state().await?);

    // Drain already available runtime events.
    while let Ok(event) = events.try_recv() {
        // Print one runtime event.
        println!("event={event}");
        // Finish the event drain loop.
    }

    // Print the shutdown result.
    print_result(
        // Label the shutdown result.
        "shutdown_tree",
        // Send the shutdown command.
        handle
            // Request tree shutdown.
            .shutdown_tree("operator", "runtime control story complete")
            // Wait for the shutdown result.
            .await?,
        // Finish the shutdown result print call.
    );

    // Finish the example successfully.
    Ok(())
    // End the runtime control example.
}

// Print a command result with a label.
/// Prints one labeled command result.
fn print_result(label: &str, result: CommandResult) {
    // Print the structured command result.
    println!("{label}={result:#?}");
    // End the print helper.
}
