//! Parses arguments for the dashboard demo example.

// Import the filesystem path type returned by argument parsing.
use std::path::PathBuf;

// Define the default demo configuration path.
const DEFAULT_CONFIG_PATH: &str = "examples/config/supervisor.yaml";

/// Parses the demo configuration argument.
///
/// # Arguments
///
/// - `args`: Command-line arguments after the program name.
///
/// # Returns
///
/// Returns the configured path or the default demo path.
pub(crate) fn parse_config_path(
    // Continue the demo expression.
    args: impl IntoIterator<Item = String>,
    // Continue the demo expression.
) -> Result<PathBuf, std::io::Error> {
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
