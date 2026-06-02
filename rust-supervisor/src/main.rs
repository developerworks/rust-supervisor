//! Supervisor configuration command entrypoint.
//!
//! This binary mirrors the `rust-config-tree` config command example while
//! binding the command handlers to the public `SupervisorConfig` type.

use clap::{Parser, Subcommand};
use rust_config_tree::cli::{ConfigCommand, handle_config_command};
use rust_config_tree::config::write_config_templates_with_schema;
use rust_supervisor::config::configurable::SupervisorConfig;
use rust_supervisor::config::factory_schema::supervisor_schema_targets_with_factory_registry;
use rust_supervisor::spec::child::TaskKind;
use rust_supervisor::task::factory::{TaskResult, service_fn};
use rust_supervisor::task::factory_registry::{TaskFactoryDescriptor, TaskFactoryRegistry};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const DEFAULT_CONFIG_PATH: &str = "examples/config/supervisor.yaml";
const DEFAULT_TEMPLATE_OUTPUT: &str = "config/supervisor_config/supervisor_config.example.yaml";
const DEFAULT_SCHEMA_OUTPUT: &str = "config/supervisor_config/supervisor_config.schema.json";

type CliResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

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
fn main() -> CliResult<()> {
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
            handle_supervisor_config_command(command, &default_config_path)?;
        }
    }

    Ok(())
}

/// Handles config commands that need supervisor-specific schema enrichment.
///
/// # Arguments
///
/// - `command`: Selected config command.
/// - `default_config_path`: Root config file used as the template source.
///
/// # Returns
///
/// Returns `Ok(())` after the selected command completes.
///
/// # Errors
///
/// Returns an error when schema generation, template generation, config
/// validation, or file writing fails.
fn handle_supervisor_config_command(
    command: ConfigCommand,
    default_config_path: &Path,
) -> CliResult<()> {
    match command {
        ConfigCommand::GenerateTemplate { output, schema } => {
            let output = output.unwrap_or_else(|| PathBuf::from(DEFAULT_TEMPLATE_OUTPUT));
            let schema = schema.unwrap_or_else(|| PathBuf::from(DEFAULT_SCHEMA_OUTPUT));
            let registry = default_task_factory_registry()?;
            write_supervisor_config_schemas(&schema, &registry)?;
            write_config_templates_with_schema::<SupervisorConfig>(
                default_config_path,
                output,
                schema,
            )?;
            Ok(())
        }
        ConfigCommand::GenerateSchema { output } => {
            let output = output.unwrap_or_else(|| PathBuf::from(DEFAULT_SCHEMA_OUTPUT));
            let registry = default_task_factory_registry()?;
            write_supervisor_config_schemas(&output, &registry)
        }
        other => {
            handle_config_command::<Cli, SupervisorConfig>(other, default_config_path)?;
            Ok(())
        }
    }
}

/// Writes supervisor config schemas with registry-backed completion values.
///
/// # Arguments
///
/// - `output_path`: Root JSON Schema output path.
/// - `registry`: Registry that provides `factory_key` completion candidates.
///
/// # Returns
///
/// Returns `Ok(())` after all schema targets have been written.
///
/// # Errors
///
/// Returns an error when schema rendering, JSON serialization, directory
/// creation, or file writing fails.
fn write_supervisor_config_schemas(
    output_path: &Path,
    registry: &TaskFactoryRegistry,
) -> CliResult<()> {
    for target in supervisor_schema_targets_with_factory_registry(output_path, registry)? {
        if let Some(parent) = target
            .path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)?;
        }
        fs::write(&target.path, target.content)?;
    }
    Ok(())
}

/// Builds the built-in factory registry used for generated config completion.
///
/// # Arguments
///
/// This function has no arguments.
///
/// # Returns
///
/// Returns a registry whose keys are available to generated JSON Schemas.
///
/// # Errors
///
/// Returns an error when a built-in descriptor is invalid or duplicated.
fn default_task_factory_registry() -> CliResult<TaskFactoryRegistry> {
    let mut registry = TaskFactoryRegistry::new();
    registry.register(TaskFactoryDescriptor::new(
        "api_server",
        "API Server",
        "Runs the API service.",
        [TaskKind::AsyncWorker],
        Arc::new(service_fn(|_ctx| async { TaskResult::Succeeded })),
    ))?;
    registry.register(TaskFactoryDescriptor::new(
        "report_exporter",
        "Report Exporter",
        "Runs blocking export work.",
        [TaskKind::BlockingWorker],
        Arc::new(service_fn(|_ctx| async { TaskResult::Succeeded })),
    ))?;
    registry.register(TaskFactoryDescriptor::new(
        "worker_factory",
        "Worker Factory",
        "Runs a generic asynchronous worker.",
        [TaskKind::AsyncWorker],
        Arc::new(service_fn(|_ctx| async { TaskResult::Succeeded })),
    ))?;
    Ok(registry)
}
