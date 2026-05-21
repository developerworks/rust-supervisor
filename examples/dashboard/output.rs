//! Prints operator-facing demo startup summaries.

// Import the demo runtime guard summary accessors.
use crate::bootstrap::DemoDashboardRuntimeGuard;
// Import path values.
use std::path::Path;

/// Prints the demo startup summary.
///
/// # Arguments
///
/// - `config_path`: Configuration file path.
/// - `runtime`: Optional demo dashboard runtime guard.
///
/// # Returns
///
/// This function has no return value.
pub(crate) fn print_startup_summary(
    // Continue the demo expression.
    config_path: &Path,
    // Continue the demo expression.
    runtime: Option<&DemoDashboardRuntimeGuard>,
    // Continue the demo expression.
) {
    // Print the configuration path.
    println!("demo config: {}", config_path.display());
    // Print the runtime information when IPC is enabled.
    if let Some(runtime) = runtime {
        // Print target identifier.
        println!("demo target: {}", runtime.target_id());
        // Print IPC path.
        println!("demo ipc: {}", runtime.ipc_path().display());
        // Print registration path when available.
        if let Some(path) = runtime.registration_path() {
            // Print relay registration path.
            println!("demo registration: {}", path.display());
            // End registration branch.
        }
        // End runtime summary branch.
    } else {
        // Print disabled IPC summary.
        println!("demo ipc: disabled");
        // End disabled runtime branch.
    }
    // Print long-running status.
    println!("demo supervisor running");
    // End startup summary.
}
