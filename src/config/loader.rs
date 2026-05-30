//! YAML configuration loader backed by `rust-config-tree` format handling.
//!
//! This module keeps parsing and validation centralized so runtime modules never
//! invent local defaults. The root file format is auto-detected by
//! `rust-config-tree` — no hard-coded format check is needed.

use crate::config::configurable::SupervisorConfig;
use crate::config::split_section::load_supervisor_config;
use crate::config::state::ConfigState;
use crate::error::types::SupervisorError;
use std::path::Path;

/// Loads validated supervisor configuration from a YAML file,
/// resolving `include` directives via `rust-config-tree`.
///
/// # Arguments
///
/// - `path`: Path to the root configuration file.
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
    // Use rust-config-tree to resolve include directives and merge
    // multiple YAML files. This ensures the `include: [..]` field
    // in SupervisorConfig is consumed per the README design principle.
    let config: SupervisorConfig = load_supervisor_config(path).map_err(|error| {
        SupervisorError::fatal_config(format!("rust-config-tree load failed: {error}"))
    })?;

    ConfigState::try_from(config)
}
