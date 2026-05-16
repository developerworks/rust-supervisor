//! Demonstrates policy decisions across typed task exits and fuse tracking.

// Import restart backoff policy values.
use rust_supervisor::policy::backoff::BackoffPolicy;
// Import typed restart policy decision values.
use rust_supervisor::policy::decision::{PolicyEngine, PolicyFailureKind, RestartPolicy, TaskExit};
// Import meltdown fuse policy values.
use rust_supervisor::policy::meltdown::{MeltdownPolicy, MeltdownTracker};
// Import duration and instant values for deterministic examples.
use std::time::{Duration, Instant};

// Run the policy failure matrix example.
/// Runs the policy failure matrix example.
fn main() {
    // Build the reusable backoff policy.
    let backoff = BackoffPolicy::new(
        // Set the initial delay.
        Duration::from_millis(100),
        // Set the maximum delay.
        Duration::from_secs(5),
        // Set jitter percent.
        10,
        // Set the reset window.
        Duration::from_secs(60),
        // Finish the reusable backoff policy.
    )
    // Enable deterministic jitter for repeatable output.
    .with_deterministic_jitter(42);
    // Create the stateless policy engine.
    let engine = PolicyEngine::new();

    // Iterate over policy and exit combinations.
    for (policy, exit) in [
        // Include a permanent policy after success.
        (RestartPolicy::Permanent, TaskExit::Succeeded),
        // Include a transient external dependency failure.
        (
            // Select transient restart behavior.
            RestartPolicy::Transient,
            // Build an external dependency failure exit.
            TaskExit::Failed {
                // Set the failure category.
                kind: PolicyFailureKind::ExternalDependency,
                // Finish the failure exit.
            },
            // Finish the policy and exit pair.
        ),
        // Include a transient fatal bug failure.
        (
            // Select transient restart behavior.
            RestartPolicy::Transient,
            // Build a fatal bug failure exit.
            TaskExit::Failed {
                // Set the failure category.
                kind: PolicyFailureKind::FatalBug,
                // Finish the failure exit.
            },
            // Finish the policy and exit pair.
        ),
        // Include a temporary panic failure.
        (
            // Select temporary restart behavior.
            RestartPolicy::Temporary,
            // Build a panic failure exit.
            TaskExit::Failed {
                // Set the failure category.
                kind: PolicyFailureKind::Panic,
                // Finish the failure exit.
            },
            // Finish the policy and exit pair.
        ),
        // Finish the decision matrix.
    ] {
        // Calculate the restart decision.
        let decision = engine.decide(policy, exit, 3, &backoff);
        // Print the policy decision row.
        println!("policy={policy:?} exit={exit:?} decision={decision:?}");
        // Finish the matrix loop.
    }

    // Build the meltdown fuse policy.
    let policy = MeltdownPolicy::new(
        // Set the child restart limit.
        2,
        // Set the child restart window.
        Duration::from_secs(60),
        // Set the group failure limit.
        5,
        // Set the group failure window.
        Duration::from_secs(60),
        // Set the supervisor failure limit.
        10,
        // Set the supervisor failure window.
        Duration::from_secs(120),
        // Set the stable reset window.
        Duration::from_secs(300),
        // Finish the meltdown policy construction.
    );
    // Create the mutable meltdown tracker.
    let mut tracker = MeltdownTracker::new(policy);
    // Capture the current monotonic instant.
    let now = Instant::now();

    // Iterate over restart offsets.
    for offset_ms in [0, 10, 20] {
        // Record a child restart at the offset instant.
        let outcome = tracker.record_child_restart(now + Duration::from_millis(offset_ms));
        // Print the fuse state after the restart.
        println!(
            // Provide the output template.
            "restart_at_ms={offset_ms} child_failures={} outcome={outcome:?}",
            // Include the current child failure count.
            tracker.child_failure_count(),
            // Finish printing the fuse state.
        );
        // Finish the fuse loop.
    }
    // End the policy failure matrix example.
}
