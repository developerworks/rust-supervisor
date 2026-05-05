//! Demonstrates a multi-child supervisor tree declaration and traversal.

// Import child identifiers.
use rust_supervisor::id::types::ChildId;
// Import readiness policy values.
use rust_supervisor::readiness::signal::ReadinessPolicy;
// Import child specification values.
use rust_supervisor::spec::child::{ChildSpec, Criticality, TaskKind};
// Import supervisor specification values.
use rust_supervisor::spec::supervisor::{SupervisionStrategy, SupervisorSpec};
// Import task factory helpers.
use rust_supervisor::task::factory::{TaskResult, service_fn};
// Import supervisor tree builder.
use rust_supervisor::tree::builder::SupervisorTree;
// Import tree ordering helpers.
use rust_supervisor::tree::order::{restart_scope, shutdown_order, startup_order};
// Import shared ownership for task factories.
use std::sync::Arc;

// Define the shared example result type.
type ExampleResult = Result<(), rust_supervisor::error::types::SupervisorError>;

// Run the supervisor tree declaration example.
/// Runs the supervisor tree declaration example.
fn main() -> ExampleResult {
    // Build the market feed child.
    let mut market_feed = worker("market_feed", "Market Feed");
    // Add low-cardinality market feed tags.
    market_feed.tags = vec!["market".to_owned(), "network".to_owned()];
    // Require explicit readiness for the market feed.
    market_feed.readiness_policy = ReadinessPolicy::Explicit;

    // Build the risk engine child.
    let mut risk_engine = worker("risk_engine", "Risk Engine");
    // Make the risk engine depend on the market feed.
    risk_engine.dependencies = vec![market_feed.id.clone()];
    // Add low-cardinality risk engine tags.
    risk_engine.tags = vec!["risk".to_owned()];

    // Build the audit sink child.
    let mut audit_sink = worker("audit_sink", "Audit Sink");
    // Mark the audit sink as optional.
    audit_sink.criticality = Criticality::Optional;
    // Add low-cardinality audit tags.
    audit_sink.tags = vec!["audit".to_owned()];

    // Build the root supervisor specification.
    let mut spec = SupervisorSpec::root(vec![market_feed.clone(), risk_engine, audit_sink]);
    // Select the RestForOne restart strategy.
    spec.strategy = SupervisionStrategy::RestForOne;
    // Set an example configuration version.
    spec.config_version = "examples-supervisor-tree-story".to_owned();

    // Build the indexed supervisor tree.
    let tree = SupervisorTree::build(&spec)?;
    // Print the root supervisor path.
    println!("root_path={}", tree.root_path);
    // Print startup order by child name.
    println!("startup_order={:?}", child_names(startup_order(&tree)));
    // Print shutdown order by child name.
    println!("shutdown_order={:?}", child_names(shutdown_order(&tree)));
    // Print the restart scope after market feed failure.
    println!(
        // Provide the output template.
        "restart_scope_after_market_feed={:?}",
        // Calculate the restart scope.
        restart_scope(&tree, spec.strategy, &market_feed.id),
        // Finish printing the restart scope.
    );

    // Finish the example successfully.
    Ok(())
    // End the supervisor tree example.
}

// Build a worker child specification.
/// Builds one worker child specification.
fn worker(id: &str, name: &str) -> ChildSpec {
    // Capture the task name for the async task.
    let task_name = name.to_owned();
    // Create a task factory from a closure.
    let factory = service_fn(move |ctx| {
        // Clone the captured task name for this attempt.
        let task_name = task_name.clone();
        // Return the async task body.
        async move {
            // Emit a heartbeat from the task context.
            ctx.heartbeat();
            // Mark the task as ready.
            ctx.mark_ready();
            // Print the task path for learners.
            println!("worker={task_name} path={}", ctx.path);
            // Report a successful task result.
            TaskResult::Succeeded
            // Finish the async task body.
        }
        // Finish the task factory closure.
    });

    // Create the worker child specification.
    ChildSpec::worker(
        // Set the child identifier.
        ChildId::new(id),
        // Set the child name.
        name,
        // Set the task kind.
        TaskKind::AsyncWorker,
        // Store the task factory behind shared ownership.
        Arc::new(factory),
        // Finish the worker child specification.
    )
    // Finish the worker builder.
}

// Collect child names from tree nodes.
/// Collects child names from tree nodes.
fn child_names(nodes: Vec<&rust_supervisor::tree::builder::SupervisorTreeNode>) -> Vec<String> {
    // Convert node references into owned child names.
    nodes
        // Consume the node vector.
        .into_iter()
        // Clone each child name.
        .map(|node| node.child.name.clone())
        // Collect the names into a vector.
        .collect()
    // Finish collecting child names.
}
