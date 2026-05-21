//! Runs the supervisor dashboard demo process for local three-end integration.

// Register command-line argument parsing.
mod args;
// Register the demo-owned IPC and registration runtime.
mod bootstrap;
// Register startup summary output.
mod output;
// Register the demo process runner.
mod runner;
// Register dashboard data scenario construction.
mod scenario;
// Register graceful shutdown helpers.
mod shutdown;

// Use the Tokio runtime for the asynchronous demo process.
#[tokio::main]
// Return boxed errors from the example process.
/// Runs the long-lived supervisor demo process.
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Parse the optional configuration argument.
    let config_path = args::parse_config_path(std::env::args().skip(1))?;
    // Run the modular demo process.
    runner::run_demo(config_path).await?;
    // Finish the demo successfully.
    Ok(())
    // End the demo process.
}
