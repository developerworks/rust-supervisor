//! Supervisor configuration command entrypoint.
//!
//! This binary mirrors the `rust-config-tree` config command example while
//! binding the command handlers to the public `SupervisorConfig` type.

use clap::{Parser, Subcommand};
use rust_config_tree::cli::{ConfigCommand, handle_config_command};
use rust_supervisor::config::configurable::SupervisorConfig;
use std::path::PathBuf;

const DEFAULT_CONFIG_PATH: &str = "examples/config/supervisor.yaml";

/// Supervisor configuration CLI.
#[derive(Debug, Parser)]
#[command(name = "rust-tokio-supervisor")]
struct Cli {
    /// Command to execute.
    #[command(subcommand)]
    command: Option<Command>,
}

/// Supervisor configuration command set.
#[derive(Debug, Subcommand)]
enum Command {
    /// Validate and print the loaded supervisor config summary.
    Run {
        /// Root config file to load.
        #[arg(long)]
        config: Option<PathBuf>,
    },

    /// Flatten the reusable rust-config-tree config commands.
    #[command(flatten)]
    Config(ConfigCommand),
}

/// Parses CLI arguments and dispatches supervisor config commands.
///
/// # Returns
///
/// Returns `Ok(())` after the selected command completes.
///
/// # Errors
///
/// Returns an error when config loading, template rendering, or file writing
/// fails.
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if std::env::args_os().len() == 1 {
        let program = std::env::args_os()
            .next()
            .unwrap_or_else(|| "rust-tokio-supervisor".into());
        let _ = Cli::parse_from([program, "--help".into()]);
        return Ok(());
    }

    let cli = Cli::parse();
    let default_config_path = PathBuf::from(DEFAULT_CONFIG_PATH);

    match cli.command.unwrap_or(Command::Run { config: None }) {
        Command::Run { config } => {
            let config_path = config.unwrap_or(default_config_path);
            let config = rust_config_tree::config::load_config::<SupervisorConfig>(&config_path)?;
            println!("config path: {}", config_path.display());
            println!("strategy: {:?}", config.supervisor.strategy);
            println!("children: {}", config.children.len());
        }
        Command::Config(command) => {
            handle_config_command::<Cli, SupervisorConfig>(command, &default_config_path)?;
        }
    }

    Ok(())
}
