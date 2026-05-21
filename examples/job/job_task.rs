//! Job role child construction for the job example.

// Import child identifiers.
use rust_supervisor::id::types::ChildId;
// Import job task role defaults.
use rust_supervisor::policy::task_role_defaults::TaskRole;
// Import child specification values.
use rust_supervisor::spec::child::{ChildSpec, Criticality, ShutdownPolicy, TaskKind};
// Import task context values.
use rust_supervisor::task::context::TaskContext;
// Import task factory helpers.
use rust_supervisor::task::factory::{TaskResult, service_fn};
// Import shared ownership for task factories.
use std::sync::Arc;
// Import time values for one-shot job work.
use std::time::{Duration, SystemTime, UNIX_EPOCH};
// Import asynchronous channel helpers.
use tokio::sync::mpsc;

/// Lifecycle fact emitted by the example job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobEvent {
    /// The job has initialized.
    Initialized {
        /// Stable child identifier.
        child_id: String,
    },
    /// The job ran its business unit.
    Running {
        /// Stable child identifier.
        child_id: String,
    },
    /// The job finished all one-shot work.
    Completed {
        /// Stable child identifier.
        child_id: String,
    },
}

/// Builds the job role child used by the example.
///
/// # Arguments
///
/// - `events`: Channel used to report job lifecycle facts to the example.
///
/// # Returns
///
/// Returns a [`ChildSpec`] whose `task_role` is [`TaskRole::Job`].
pub fn job_child(events: mpsc::UnboundedSender<JobEvent>) -> ChildSpec {
    // Build a task factory from the job function.
    let factory = service_fn(move |ctx: TaskContext| {
        // Clone the event sender for this attempt.
        let events = events.clone();
        // Run one job attempt.
        async move { run_job(ctx, events).await }
    });
    // Build a base async worker child.
    let mut child = ChildSpec::worker(
        // Set the stable child identifier.
        ChildId::new("daily-report-job"),
        // Set the display name.
        "Daily Report Job",
        // Select async worker execution.
        TaskKind::AsyncWorker,
        // Store the factory behind shared ownership.
        Arc::new(factory),
    );
    // Classify the task as a one-shot job.
    child.task_role = Some(TaskRole::Job);
    // Mark the job as optional in parent health policy.
    child.criticality = Criticality::Optional;
    // Add stable diagnostic tags.
    child.tags = vec!["job".to_owned(), "report".to_owned()];
    // Use short child shutdown budgets for a fast example.
    child.shutdown_policy =
        ShutdownPolicy::new(Duration::from_millis(150), Duration::from_millis(50));
    // Return the job child declaration.
    child
}

/// Runs one one-shot job attempt.
///
/// # Arguments
///
/// - `ctx`: Runtime context for the current child attempt.
/// - `events`: Channel used to report job lifecycle facts.
///
/// # Returns
///
/// Returns [`TaskResult::Succeeded`] after one business unit completes.
async fn run_job(ctx: TaskContext, events: mpsc::UnboundedSender<JobEvent>) -> TaskResult {
    // Mark readiness after initialization work finishes.
    ctx.mark_ready();
    // Emit an initial heartbeat for observers.
    ctx.heartbeat();
    // Publish the initialization fact.
    let _ignored = events.send(JobEvent::Initialized {
        child_id: ctx.child_id.value.clone(),
    });
    // Simulate one report generation step.
    tokio::time::sleep(Duration::from_millis(300)).await;
    // Run the one-shot business function.
    run_report_job_business(&ctx, &events);
    // Publish completion.
    let _ignored = events.send(JobEvent::Completed {
        child_id: ctx.child_id.value.clone(),
    });
    // Report successful one-shot completion.
    TaskResult::Succeeded
}

/// Runs the one-shot report job business function.
///
/// # Arguments
///
/// - `ctx`: Runtime context for the current child attempt.
/// - `events`: Channel used to report job lifecycle facts.
///
/// # Returns
///
/// This function does not return a value.
fn run_report_job_business(ctx: &TaskContext, events: &mpsc::UnboundedSender<JobEvent>) {
    // Read the current system time for the business output.
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    // Print the one-shot job time.
    println!(
        "job business: child={} now_unix={}.{:09}",
        ctx.child_id,
        now.as_secs(),
        now.subsec_nanos()
    );
    // Emit heartbeat for liveness observation.
    ctx.heartbeat();
    // Publish the running fact.
    let _ignored = events.send(JobEvent::Running {
        child_id: ctx.child_id.value.clone(),
    });
}
