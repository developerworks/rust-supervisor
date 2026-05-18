//! External command allowlist (C9).
//!
//! Only absolute executable paths listed in the allowlist configuration
//! are eligible for execution via control-plane extension points.
//! Default: empty list (deny all external commands).

use crate::config::ipc_security::AllowlistConfig;
use crate::dashboard::error::DashboardError;

/// Checks whether an executable path is in the allowlist (C9).
///
/// # Arguments
///
/// - `path`: Absolute executable path to check.
/// - `config`: Allowlist configuration.
///
/// # Returns
///
/// Returns `Ok(())` when the path is allowed, or `Err(DashboardError)`
/// with `allowlist_empty` or `allowlist_denied`.
pub fn check_allowlist(path: &str, config: &AllowlistConfig) -> Result<(), DashboardError> {
    if !config.enabled {
        return Ok(());
    }

    if config.allowed_paths.is_empty() {
        return Err(DashboardError::allowlist_empty());
    }

    if !config.allowed_paths.iter().any(|p| p == path) {
        return Err(DashboardError::allowlist_denied(path));
    }

    Ok(())
}
