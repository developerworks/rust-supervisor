//! Demonstrates typed restart policy decisions for learning.

// Import typed task failure values.
use rust_supervisor::error::types::{TaskFailure, TaskFailureKind};
// Import policy decision types.
use rust_supervisor::policy::decision::{RestartDecision, RestartPolicy};
// Import the canonical supervision strategy declaration type.
use rust_supervisor::spec::supervisor::SupervisionStrategy;
// Import duration values for restart delay examples.
use std::time::Duration;

// Run the restart policy learning example.
/// Runs the restart policy learning example.
fn main() {
    // Build a typed panic failure.
    let failure = TaskFailure::new(TaskFailureKind::Panic, "panic", "worker panicked");
    // Select a policy that restarts failed attempts only.
    let policy = RestartPolicy::Transient;
    // Select a restart scope for one failed child.
    let strategy = SupervisionStrategy::OneForOne;
    // Set the example restart delay.
    let delay = Duration::from_millis(100);
    // Build a concrete delayed restart decision.
    let decision = RestartDecision::RestartAfter { delay };
    // Print the typed failure.
    println!("failure={failure:#?}");
    // Print the selected restart policy.
    println!("policy={policy:#?}");
    // Print the selected supervision strategy.
    println!("strategy={strategy:#?}");
    // Print the resulting restart decision.
    println!("decision={decision:#?}");
    // End the restart policy example.
}
