//! Loads a supervisor tree from YAML, binds one long-running Service child, and
//! runs until the operator sends Ctrl+C.

use rust_supervisor::control::command::CommandResult;
use rust_supervisor::error::types::SupervisorError;
use rust_supervisor::runtime::supervisor::Supervisor;
use rust_supervisor::spec::child::TaskKind;
use rust_supervisor::task::context::TaskContext;
use rust_supervisor::task::factory::{TaskResult, service_fn};
use rust_supervisor::task::factory_registry::{TaskFactoryDescriptor, TaskFactoryRegistry};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

/// Shared result type for this example.
type ExampleResult = Result<(), SupervisorError>;

/// Factory registry key that must match `factory_key` in `supervisor.yaml`.
const SERVICE_FACTORY_KEY: &str = "yaml_step_service";

/// Returns the path to this example's YAML configuration file.
///
/// # Arguments
///
/// This function has no arguments.
///
/// # Returns
///
/// Returns an absolute path rooted at the crate manifest directory.
fn example_config_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/yaml_create_supervisor_with_service/supervisor.yaml")
}

/// Registers the long-running Service factory used by the YAML child declaration.
///
/// # Arguments
///
/// - `events`: Channel used to publish service lifecycle facts to the example.
///
/// # Returns
///
/// Returns a registry that maps [`SERVICE_FACTORY_KEY`] to the service factory,
/// or an error when registration fails.
fn task_factory_registry(
    events: mpsc::UnboundedSender<String>,
) -> Result<TaskFactoryRegistry, SupervisorError> {
    let factory = service_fn(move |ctx: TaskContext| {
        let events = events.clone();
        async move { run_service(ctx, events).await }
    });
    let mut registry = TaskFactoryRegistry::new();
    registry.register(TaskFactoryDescriptor::new(
        SERVICE_FACTORY_KEY,
        "YAML Step Service",
        "Long-running service started from YAML configuration.",
        [TaskKind::AsyncWorker],
        Arc::new(factory),
    ))?;
    Ok(registry)
}

/// Runs one service attempt until supervisor shutdown cancels it.
///
/// # Arguments
///
/// - `ctx`: Runtime context for the current child attempt.
/// - `events`: Channel used to publish service lifecycle facts.
///
/// # Returns
///
/// Returns [`TaskResult::Cancelled`] after cooperative shutdown.
async fn run_service(ctx: TaskContext, events: mpsc::UnboundedSender<String>) -> TaskResult {
    ctx.mark_ready();
    ctx.heartbeat();
    let _ignored = events.send(format!("service initialized: child={}", ctx.child_id));
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    let cancellation_token = ctx.cancellation_token();
    let mut tick = 0_u64;
    // Skip the immediate first interval tick so running output starts after one second.
    interval.tick().await;
    loop {
        tokio::select! {
            _ = cancellation_token.cancelled() => {
                let _ignored = events.send(format!("service stopping: child={}", ctx.child_id));
                return TaskResult::Cancelled;
            }
            _ = interval.tick() => {
                tick += 1;
                ctx.heartbeat();
                let _ignored = events.send(format!(
                    "service running: child={} tick={tick}",
                    ctx.child_id
                ));
            }
        }
    }
}

/// Drains service lifecycle facts that are already available.
///
/// # Arguments
///
/// - `events`: Receiver for service lifecycle facts.
///
/// # Returns
///
/// This function does not return a value.
fn drain_service_events(events: &mut mpsc::UnboundedReceiver<String>) {
    while let Ok(event) = events.try_recv() {
        println!("yaml_service {event}");
    }
}

/// Prints a labeled [`CurrentState`] value as YAML.
///
/// # Arguments
///
/// - `label`: Output label printed before the YAML document.
/// - `result`: Control command result expected to carry current state.
///
/// # Returns
///
/// Returns `Ok(())` when serialization and printing succeed.
fn print_current_state_yaml(label: &str, result: CommandResult) -> Result<(), SupervisorError> {
    let state = match result {
        CommandResult::CurrentState { state } => state,
        other => {
            return Err(SupervisorError::fatal_config(format!(
                "expected CurrentState command result, got {other:?}"
            )));
        }
    };
    let yaml = serde_yaml::to_string(&state).map_err(|error| {
        SupervisorError::fatal_config(format!(
            "failed to serialize current state as YAML: {error}"
        ))
    })?;
    println!("yaml_service {label}:\n{yaml}");
    Ok(())
}

/// Runs the YAML supervisor with service example.
#[tokio::main]
async fn main() -> ExampleResult {
    // Create a channel for service lifecycle facts.
    let (service_event_sender, mut service_events) = mpsc::unbounded_channel();
    // Register the service factory required by the YAML child declaration.
    let registry = task_factory_registry(service_event_sender)?;
    // Resolve the YAML configuration path for this example.
    let config_path = example_config_path();

    // Start the supervisor from YAML with explicit factory bindings.
    let handle = Supervisor::start_from_config_file_with_factories(&config_path, registry).await?;

    // Wait until the service reports initialization.
    let initialized = service_events
        .recv()
        .await
        .ok_or_else(|| SupervisorError::fatal_config("service initialization channel closed"))?;
    // Print the first service lifecycle fact.
    println!("yaml_service {initialized}");

    // Print the initial current state as YAML.
    print_current_state_yaml("supervisor_state", handle.current_state().await?)?;
    // Tell the operator how to stop the example.
    println!(
        "yaml_service running until Ctrl+C (config={})",
        config_path.display()
    );

    // Keep the example alive until the operator sends Ctrl+C.
    loop {
        // Wait for either an operator signal or the next service fact.
        tokio::select! {
            signal = tokio::signal::ctrl_c() => {
                // Convert Ctrl+C errors into supervisor errors.
                signal.map_err(|error| {
                    SupervisorError::fatal_config(format!(
                        "failed to receive Ctrl+C signal: {error}"
                    ))
                })?;
                // Print the operator stop fact.
                println!("\n\nyaml_service operator signal=ctrl_c\n");
                // Leave the long-running example loop.
                break;
            }
            event = service_events.recv() => {
                // Convert an unexpected event channel close into a supervisor error.
                let event = event.ok_or_else(|| {
                    SupervisorError::fatal_config("service event channel closed while running")
                })?;
                // Print the service fact immediately instead of batching it by observation tick.
                println!("yaml_service {event}");
            }
        }
    }

    // Request a graceful shutdown for the supervisor tree.
    handle
        .shutdown_tree("operator", "yaml create supervisor with service complete")
        .await?;
    // Print service facts emitted during shutdown.
    drain_service_events(&mut service_events);
    // Print the final current state after shutdown.
    print_current_state_yaml("after_shutdown", handle.current_state().await?)?;
    // Finish the example successfully.
    Ok(())
}
