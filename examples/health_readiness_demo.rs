//! Demonstrates health policy, readiness signaling, and heartbeat monitoring.
//!
//! A child task can expose:
//!   - Heartbeats: periodic signals that the child is still executing.
//!   - Readiness: a signal that the child has completed its initialization
//!     and is ready to serve requests.
//!
//! This example shows how to configure health intervals, stale detection,
//! and readiness policies without running a full supervisor.

// Import readiness policy and signal types.
use rust_supervisor::readiness::signal::{ReadinessPolicy, ReadySignal};
// Import health and readiness configuration types.
use rust_supervisor::spec::child::{HealthCheckConfig, HealthPolicy, ReadinessConfig};
// Import duration values for deterministic configuration.
use std::time::Duration;

/// Runs the health and readiness demonstration.
fn main() {
    // Print the demo title.
    println!("=== Health & Readiness Demo ===");
    // Add spacing before the first section.
    println!();

    // --- Health Policy ---
    println!("--- Health Policy ---");
    // Add spacing before the health policy values.
    println!();

    // Build a health policy with heartbeat and stale thresholds.
    let health = HealthPolicy::new(
        // Set the expected heartbeat interval.
        Duration::from_secs(1), // heartbeat_interval
        // Set the stale detection threshold.
        Duration::from_secs(3), // stale_after
                                // Finish the health policy construction.
    );

    // Print the heartbeat interval.
    println!("  heartbeat_interval = {:?}", health.heartbeat_interval);
    // Print the stale detection threshold.
    println!("  stale_after        = {:?}", health.stale_after);
    // Print the combined health interpretation.
    println!(
        // Format the heartbeat requirement.
        "  -> child must heartbeat every {:?}; after {:?} of silence it is declared stale",
        // Pass the heartbeat interval.
        health.heartbeat_interval,
        health.stale_after,
        // Finish the combined health interpretation.
    );

    // --- Health Check Config ---
    println!();
    // Print the health check config section title.
    println!("--- Health Check Config ---");
    // Add spacing before the health check config values.
    println!();

    // Build a health check configuration.
    let hc = HealthCheckConfig {
        // Set the periodic check interval.
        check_interval_secs: 10,
        // Set the per-check timeout.
        timeout_secs: 5,
        // Set the retry limit.
        max_retries: 3,
    };

    // Print the health check interval.
    println!("  check_interval = {}s", hc.check_interval_secs);
    // Print the health check timeout.
    println!("  timeout        = {}s", hc.timeout_secs);
    // Print the health check retry limit.
    println!("  max_retries    = {}", hc.max_retries);

    // --- Readiness Policy ---
    println!();
    // Print the readiness policy section title.
    println!("--- Readiness Policy ---");
    // Add spacing before readiness policy values.
    println!();

    // Select immediate readiness for comparison.
    let immediate = ReadinessPolicy::Immediate;
    // Select explicit readiness for comparison.
    let explicit = ReadinessPolicy::Explicit;

    // Print the immediate readiness behavior.
    println!("  {immediate:?} -> child is considered ready as soon as it starts");
    // Print the explicit readiness behavior.
    println!("  {explicit:?} -> child must call mark_ready() before being considered ready");

    // --- ReadySignal ---
    println!();
    // Print the ready signal section title.
    println!("--- ReadySignal (Explicit mode) ---");
    // Add spacing before ready signal values.
    println!();

    // Create a readiness signal and receiver pair.
    let (signal, receiver) = ReadySignal::new();

    // Print the initial readiness state.
    println!("  initial state:     {:?}", *receiver.borrow());
    // Add spacing before the simulated transition.
    println!();

    // Simulate the child marking itself ready after initialization.
    println!("  [child initializes and calls mark_ready()]");
    // Mark the child as ready.
    signal.mark_ready();
    // Print the readiness state after the transition.
    println!("  after mark_ready() = {:?}", *receiver.borrow());

    // Readiness Config
    println!();
    // Print the readiness config section title.
    println!("--- Readiness Config ---");
    // Add spacing before readiness config values.
    println!();

    // Build a readiness configuration.
    let rc = ReadinessConfig {
        // Set the readiness check interval.
        check_interval_secs: 5,
        // Set the readiness timeout.
        timeout_secs: 3,
    };

    // Print the readiness check interval.
    println!("  check_interval  = {}s", rc.check_interval_secs);
    // Print the readiness timeout.
    println!("  timeout         = {}s", rc.timeout_secs);

    // Child lifecycle phases.
    println!();
    // Print the lifecycle phase section title.
    println!("--- Child Lifecycle Phases ---");
    // Add spacing before lifecycle phase rows.
    println!();

    // Build the ordered phase descriptions.
    let phases = [
        // Describe the starting phase.
        ("Starting", "child is spawned, no heartbeat yet"),
        // Describe the running phase.
        ("Running", "child emits heartbeats periodically"),
        // Describe the ready phase.
        ("Ready", "child called mark_ready() (if Explicit)"),
        // Describe the stale phase.
        ("Stale", "heartbeat timeout exceeded -> unhealthy"),
        // Describe the stopped phase.
        ("Stopped", "child exited normally or was cancelled"),
        // Finish the phase list.
    ];

    // Iterate over each lifecycle phase.
    for (phase, desc) in &phases {
        // Print the lifecycle phase row.
        println!("  {phase:12} - {desc}");
        // Finish the lifecycle phase row.
    }

    // Add spacing before the summary.
    println!();
    // Print the summary title.
    println!("=== Summary ===");
    // Print the health policy summary.
    println!("HealthPolicy controls heartbeat interval and stale detection thresholds.");
    // Print the readiness policy summary.
    println!("ReadinessPolicy::Immediate -> ready on start; Explicit -> requires mark_ready().");
    // Print the ready signal summary.
    println!("ReadySignal is thread-safe and can be shared between the task and supervisor.");
    // Finish the health and readiness demo.
}
