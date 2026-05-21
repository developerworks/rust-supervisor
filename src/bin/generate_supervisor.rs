//! Supervisor configuration command entrypoint.
//!
//! This binary mirrors the `rust-config-tree` config command example while
//! binding the command handlers to the public `SupervisorConfig` type.

use clap::{Parser, Subcommand};
use rust_config_tree::{ConfigCommand, handle_config_command, load_config};
use rust_supervisor::config::configurable::SupervisorConfig;
use std::path::PathBuf;

const DEFAULT_CONFIG_PATH: &str = "examples/config/supervisor.yaml";

/// Supervisor configuration CLI.
#[derive(Debug, Parser)]
#[command(name = "generate-supervisor")]
struct Cli {
    /// Root config file used as the template source and validation input.
    #[arg(long)]
    config: Option<PathBuf>,

    /// Command to execute.
    #[command(subcommand)]
    command: Option<Command>,
}

/// Supervisor configuration command set.
#[derive(Debug, Subcommand)]
enum Command {
    /// Validate and print the loaded supervisor config summary.
    Run,

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
    let cli = Cli::parse();
    let config_path = cli
        .config
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_PATH));

    match cli.command.unwrap_or(Command::Run) {
        Command::Run => {
            let config = load_config::<SupervisorConfig>(&config_path)?;
            println!("config path: {}", config_path.display());
            println!("strategy: {:?}", config.supervisor.strategy);
            println!("children: {}", config.children.len());
        }
        Command::Config(command) => {
            handle_config_command::<Cli, SupervisorConfig>(command, &config_path)?;
        }
    }

    Ok(())
}
