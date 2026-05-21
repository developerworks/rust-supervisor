//! Worker role child construction for the worker example.

// Import child identifiers.
use rust_supervisor::id::types::ChildId;
// Import worker task role defaults.
use rust_supervisor::policy::task_role_defaults::TaskRole;
// Import child specification values.
use rust_supervisor::spec::child::{ChildSpec, Criticality, ShutdownPolicy, TaskKind};
// Import task context values.
use rust_supervisor::task::context::TaskContext;
// Import task factory helpers.
use rust_supervisor::task::factory::{TaskResult, service_fn};
// Import shared ownership for task factories.
use std::sync::Arc;
// Import time values for bounded worker work.
use std::time::{Duration, SystemTime, UNIX_EPOCH};
// Import asynchronous channel helpers.
use tokio::sync::mpsc;

/// Lifecycle fact emitted by the example worker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerEvent {
    /// The worker has initialized.
    Initialized {
        /// Stable child identifier.
        child_id: String,
    },
    /// The worker completed one batch.
    Running {
        /// Stable child identifier.
        child_id: String,
        /// Batch number completed by the worker.
        batch: u64,
    },
    /// The worker finished all bounded work.
    Completed {
        /// Stable child identifier.
        child_id: String,
    },
}

/// Builds the worker role child used by the example.
///
/// # Arguments
///
/// - `events`: Channel used to report worker lifecycle facts to the example.
///
/// # Returns
///
/// Returns a [`ChildSpec`] whose `task_role` is [`TaskRole::Worker`].
pub fn worker_child(events: mpsc::UnboundedSender<WorkerEvent>) -> ChildSpec {
    // Build a task factory from the worker function.
    let factory = service_fn(move |ctx: TaskContext| {
        // Clone the event sender for this attempt.
        let events = events.clone();
        // Run one worker attempt.
        async move { run_worker(ctx, events).await }
    });
    // Build a base async worker child.
    let mut child = ChildSpec::worker(
        // Set the stable child identifier.
        ChildId::new("invoice-worker"),
        // Set the display name.
        "Invoice Worker",
        // Select async worker execution.
        TaskKind::AsyncWorker,
        // Store the factory behind shared ownership.
        Arc::new(factory),
    );
    // Classify the task as a bounded worker.
    child.task_role = Some(TaskRole::Worker);
    // Keep the worker in the critical path for this example.
    child.criticality = Criticality::Critical;
    // Add stable diagnostic tags.
    child.tags = vec!["worker".to_owned(), "invoice".to_owned()];
    // Use short shutdown budgets for a fast example.
    child.shutdown_policy =
        ShutdownPolicy::new(Duration::from_millis(150), Duration::from_millis(50));
    // Return the worker child declaration.
    child
}

/// Runs one bounded worker attempt.
///
/// # Arguments
///
/// - `ctx`: Runtime context for the current child attempt.
/// - `events`: Channel used to report worker lifecycle facts.
///
/// # Returns
///
/// Returns [`TaskResult::Succeeded`] after all batches complete.
async fn run_worker(ctx: TaskContext, events: mpsc::UnboundedSender<WorkerEvent>) -> TaskResult {
    // Mark readiness after initialization work finishes.
    ctx.mark_ready();
    // Emit an initial heartbeat for observers.
    ctx.heartbeat();
    // Publish the initialization fact.
    let _ignored = events.send(WorkerEvent::Initialized {
        child_id: ctx.child_id.value.clone(),
    });
    // Process a small bounded batch set.
    for batch in 1..=3 {
        // Simulate one worker batch.
        tokio::time::sleep(Duration::from_millis(200)).await;
        // Run one business batch.
        run_invoice_worker_business(&ctx, &events, batch);
    }
    // Publish completion.
    let _ignored = events.send(WorkerEvent::Completed {
        child_id: ctx.child_id.value.clone(),
    });
    // Report successful bounded completion.
    TaskResult::Succeeded
}

/// Runs one invoice worker business batch.
///
/// # Arguments
///
/// - `ctx`: Runtime context for the current child attempt.
/// - `events`: Channel used to report worker lifecycle facts.
/// - `batch`: Batch number completed by the worker.
///
/// # Returns
///
/// This function does not return a value.
fn run_invoice_worker_business(
    ctx: &TaskContext,
    events: &mpsc::UnboundedSender<WorkerEvent>,
    batch: u64,
) {
    // Read the current system time for the business output.
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    // Print the current batch time.
    println!(
        "worker business: child={} batch={} now_unix={}.{:09}",
        ctx.child_id,
        batch,
        now.as_secs(),
        now.subsec_nanos()
    );
    // Emit heartbeat for liveness observation.
    ctx.heartbeat();
    // Publish the running fact.
    let _ignored = events.send(WorkerEvent::Running {
        child_id: ctx.child_id.value.clone(),
        batch,
    });
}
