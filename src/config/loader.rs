//! YAML configuration loader backed by `rust-config-tree` format handling.
//!
//! This module keeps parsing and validation centralized so runtime modules never
//! invent local defaults.

use crate::config::configurable::SupervisorConfig;
use crate::config::state::ConfigState;
use crate::error::types::SupervisorError;
use std::path::Path;

/// Loads validated supervisor configuration from a YAML file,
/// resolving `include` directives via `rust-config-tree`.
///
/// # Arguments
///
/// - `path`: Path to the root YAML configuration file.
///
/// # Returns
///
/// Returns a validated [`ConfigState`] when the file is readable and complete.
///
/// # Examples
///
/// ```no_run
/// let state = rust_supervisor::config::loader::load_config_from_yaml_file(
///     "examples/config/supervisor.yaml",
/// );
/// assert!(state.is_ok());
/// ```
pub fn load_config_from_yaml_file(path: impl AsRef<Path>) -> Result<ConfigState, SupervisorError> {
    ensure_yaml_format(path.as_ref())?;

    // Use rust-config-tree to resolve include directives and merge
    // multiple YAML files. This ensures the `include: [..]` field
    // in SupervisorConfig is consumed per the README design principle.
    let config: SupervisorConfig = rust_config_tree::load_config(path).map_err(|error| {
        SupervisorError::fatal_config(format!("rust-config-tree load failed: {error}"))
    })?;

    ConfigState::try_from(config)
}

/// Ensures the root file is treated as YAML by `rust-config-tree`.
///
/// # Arguments
///
/// - `path`: Configuration path whose extension should be checked.
///
/// # Returns
///
/// Returns `Ok(())` when `rust-config-tree` selects YAML.
fn ensure_yaml_format(path: &Path) -> Result<(), SupervisorError> {
    let format = rust_config_tree::ConfigFormat::from_path(path);
    if format == rust_config_tree::ConfigFormat::Yaml {
        Ok(())
    } else {
        Err(SupervisorError::fatal_config(
            "supervisor configuration must use YAML",
        ))
    }
}
