//! YAML configuration loader backed by `rust-config-tree` format handling.
//!
//! This module keeps parsing and validation centralized so runtime modules never
//! invent local defaults.

use crate::config::configurable::SupervisorConfig;
use crate::config::state::ConfigState;
use crate::error::types::SupervisorError;
use std::fs;
use std::path::Path;

/// Loads validated supervisor configuration from a YAML file.
///
/// # Arguments
///
/// - `path`: Path to the YAML configuration file.
///
/// # Returns
///
/// Returns a validated [`ConfigState`] when the file is readable and complete.
///
/// # Examples
///
/// ```no_run
/// let state = rust_supervisor::config::loader::load_config_state(
///     "examples/config/supervisor.yaml",
/// );
/// assert!(state.is_ok());
/// ```
pub fn load_config_state(path: impl AsRef<Path>) -> Result<ConfigState, SupervisorError> {
    ensure_yaml_format(path.as_ref())?;
    let contents = fs::read_to_string(path.as_ref()).map_err(|error| {
        SupervisorError::fatal_config(format!("failed to read config file: {error}"))
    })?;
    let config: SupervisorConfig = serde_yaml::from_str(&contents).map_err(|error| {
        SupervisorError::fatal_config(format!("failed to parse YAML config: {error}"))
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
