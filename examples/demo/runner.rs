//! Orchestrates the modular supervisor demo process.

// Import the demo runtime starter.
use crate::bootstrap::start_demo_dashboard_runtime;
// Import startup summary output.
use crate::output::print_startup_summary;
// Import graceful shutdown helper.
use crate::shutdown::shutdown_demo;
// Import configuration loading.
use rust_supervisor::config::loader::load_config_state;
// Import validated configuration state.
use rust_supervisor::config::state::ConfigState;
// Import the supervisor runtime entry point.
use rust_supervisor::runtime::supervisor::Supervisor;
// Import path storage.
use std::path::PathBuf;

/// Runs the demo process.
///
/// # Arguments
///
/// - `config_path`: Supervisor configuration file path.
///
/// # Returns
///
/// Returns success after operator shutdown.
pub(crate) async fn run_demo(
    // Continue the demo expression.
    config_path: PathBuf,
    // Continue the demo expression.
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Load the full demo configuration.
    let state = load_config_state(&config_path)?;
    // Start the demo-owned dashboard IPC and registration runtime.
    let demo_runtime = start_demo_dashboard_runtime(&state)?;
    // Build a pure supervisor runtime configuration.
    let supervisor_state = supervisor_runtime_state(state);
    // Start the library supervisor without core demo state intrusion.
    let handle = Supervisor::start_from_config_state(supervisor_state).await?;
    // Query current state once to prove the runtime is live.
    let current = handle.current_state().await?;
    // Print the runtime state for operator inspection.
    println!("{current:#?}");
    // Print the demo startup summary.
    print_startup_summary(&config_path, demo_runtime.as_ref());
    // Wait until the operator stops the demo process.
    tokio::signal::ctrl_c().await?;
    // Shut down the supervisor tree before dropping resources.
    shutdown_demo(&handle).await?;
    // Drop the supervisor handle explicitly.
    drop(handle);
    // Drop the demo runtime explicitly so the IPC socket is cleaned up.
    drop(demo_runtime);
    // Finish the demo successfully.
    Ok(())
    // End demo runner.
}

/// Removes IPC from the library supervisor runtime state.
///
/// # Arguments
///
/// - `state`: Full loaded configuration state.
///
/// # Returns
///
/// Returns a configuration state for the pure library supervisor runtime.
fn supervisor_runtime_state(mut state: ConfigState) -> ConfigState {
    // Keep dashboard IPC owned by the demo runtime instead of core runtime.
    state.ipc = None;
    // Return the adjusted runtime state.
    state
    // End runtime state adjustment.
}
