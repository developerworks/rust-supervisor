//! Handles graceful demo shutdown.

// Import supervisor handle used for shutdown commands.
use rust_supervisor::control::handle::SupervisorHandle;

/// Shuts down the supervisor tree used by the demo.
///
/// # Arguments
///
/// - `handle`: Runtime handle returned by the supervisor.
///
/// # Returns
///
/// Returns success after the shutdown command is accepted.
pub(crate) async fn shutdown_demo(
    // Continue the demo expression.
    handle: &SupervisorHandle,
    // Continue the demo expression.
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Send a visible shutdown reason through the public handle.
    handle.shutdown_tree("operator", "demo shutdown").await?;
    // Finish graceful shutdown.
    Ok(())
    // End shutdown helper.
}
