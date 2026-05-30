//! Default paths for generated supervisor configuration templates.

use std::path::PathBuf;

/// Resolves the default generated template directory for supervisor configuration.
pub fn default_generated_config_dir() -> PathBuf {
    PathBuf::from("config").join("supervisor_config")
}
