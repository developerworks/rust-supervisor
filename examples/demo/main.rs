//! Runs the supervisor dashboard demo process for local three-end integration.

// Import the filesystem path type used by argument parsing.
use std::path::PathBuf;
// Import the supervisor runtime entry point.
use rust_supervisor::runtime::supervisor::Supervisor;

// Define the default demo configuration path.
const DEFAULT_CONFIG_PATH: &str = "examples/config/supervisor.yaml";

// Define the shared demo result type.
type DemoResult = Result<(), Box<dyn std::error::Error>>;

// Use the Tokio runtime for the asynchronous demo process.
#[tokio::main]
/// Runs the long-lived supervisor demo process.
async fn main() -> DemoResult {
    // Parse the optional configuration argument.
    let config_path = parse_config_path(std::env::args().skip(1))?;
    // Start from configuration so dashboard IPC and registration heartbeat run.
    let handle = Supervisor::start_from_config_file(&config_path).await?;
    // Query current state once to prove the supervisor is live.
    let current = handle.current_state().await?;
    // Print the current state for operator inspection.
    println!("{current:#?}");
    // Print that the local demo session is now waiting.
    println!("demo supervisor running");
    // Wait until the operator stops the demo process.
    tokio::signal::ctrl_c().await?;
    // Shut down the supervisor tree before dropping the runtime handle.
    handle.shutdown_tree("operator", "demo shutdown").await?;
    // Drop the handle so the dashboard IPC guard removes its socket.
    drop(handle);
    // Finish the demo successfully.
    Ok(())
    // End the demo process.
}

/// Parses the demo configuration argument.
///
/// # Arguments
///
/// - `args`: Command-line arguments after the program name.
///
/// # Returns
///
/// Returns the configured path or the default demo path.
fn parse_config_path(args: impl IntoIterator<Item = String>) -> Result<PathBuf, std::io::Error> {
    // Convert the incoming arguments into an iterator.
    let mut args = args.into_iter();
    // Read the first optional argument.
    let first = args.next();
    // Return the default path when no arguments are provided.
    if first.is_none() {
        // Return the default demo configuration path.
        return Ok(PathBuf::from(DEFAULT_CONFIG_PATH));
        // End the empty argument branch.
    }
    // Extract the first argument after the empty case has returned.
    let first = first.expect("first argument should exist after empty check");
    // Reject any unsupported argument name.
    if first != "--config" {
        // Return an unsupported argument error.
        return Err(invalid_input(format!("unknown argument: {first}")));
        // End the unsupported argument branch.
    }
    // Require a path after the configuration flag.
    let path = require_config_path(args.next())?;
    // Reject trailing arguments so the demo stays deterministic.
    if let Some(extra) = args.next() {
        // Return an unsupported trailing argument error.
        return Err(invalid_input(format!("unknown argument: {extra}")));
        // End the trailing argument branch.
    }
    // Return the explicit configuration path.
    Ok(PathBuf::from(path))
    // End argument parsing.
}

/// Requires the value after the configuration flag.
///
/// # Arguments
///
/// - `path`: Optional path argument.
///
/// # Returns
///
/// Returns the explicit path or an invalid-input error.
fn require_config_path(path: Option<String>) -> Result<String, std::io::Error> {
    // Convert the optional path into a typed result.
    let path = path.ok_or_else(|| invalid_input("--config requires a path"))?;
    // Return the validated path.
    Ok(path)
    // End configuration path validation.
}

/// Builds an invalid input error for argument parsing.
///
/// # Arguments
///
/// - `message`: Human-readable argument error.
///
/// # Returns
///
/// Returns a standard I/O error with invalid-input kind.
fn invalid_input(message: impl Into<String>) -> std::io::Error {
    // Convert argument validation failures into a standard error type.
    std::io::Error::new(std::io::ErrorKind::InvalidInput, message.into())
    // End invalid-input construction.
}
