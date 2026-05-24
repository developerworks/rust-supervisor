//! Demonstrates health policy, readiness signaling, and heartbeat monitoring.
//!
//! A child task can expose:
//!   - Heartbeats: periodic signals that the child is still executing.
//!   - Readiness: a signal that the child has completed its initialization
//!     and is ready to serve requests.
//!
//! This example shows how to configure health intervals, stale detection,
//! and readiness policies without running a full supervisor.

use rust_supervisor::readiness::signal::{ReadinessPolicy, ReadySignal};
use rust_supervisor::spec::child::{HealthCheckConfig, HealthPolicy, ReadinessConfig};
use std::time::Duration;

/// Runs the health and readiness demonstration.
fn main() {
    println!("=== Health & Readiness Demo ===");
    println!();

    // --- Health Policy ---
    println!("--- Health Policy ---");
    println!();

    let health = HealthPolicy::new(
        Duration::from_secs(1), // heartbeat_interval
        Duration::from_secs(3), // stale_after
    );

    println!("  heartbeat_interval = {:?}", health.heartbeat_interval);
    println!("  stale_after        = {:?}", health.stale_after);
    println!(
        "  -> child must heartbeat every {:?}; after {:?} of silence it is declared stale",
        health.heartbeat_interval, health.stale_after,
    );

    // --- Health Check Config ---
    println!();
    println!("--- Health Check Config ---");
    println!();

    let hc = HealthCheckConfig {
        check_interval_secs: 10,
        timeout_secs: 5,
        max_retries: 3,
    };

    println!("  check_interval = {}s", hc.check_interval_secs);
    println!("  timeout        = {}s", hc.timeout_secs);
    println!("  max_retries    = {}", hc.max_retries);

    // --- Readiness Policy ---
    println!();
    println!("--- Readiness Policy ---");
    println!();

    let immediate = ReadinessPolicy::Immediate;
    let explicit = ReadinessPolicy::Explicit;

    println!("  {immediate:?} -> child is considered ready as soon as it starts");
    println!("  {explicit:?} -> child must call mark_ready() before being considered ready");

    // --- ReadySignal ---
    println!();
    println!("--- ReadySignal (Explicit mode) ---");
    println!();

    let (signal, receiver) = ReadySignal::new();

    println!("  initial state:     {:?}", *receiver.borrow());
    println!();

    // Simulate the child marking itself ready after initialization.
    println!("  [child initializes and calls mark_ready()]");
    signal.mark_ready();
    println!("  after mark_ready() = {:?}", *receiver.borrow());

    // Readiness Config
    println!();
    println!("--- Readiness Config ---");
    println!();

    let rc = ReadinessConfig {
        check_interval_secs: 5,
        timeout_secs: 3,
    };

    println!("  check_interval  = {}s", rc.check_interval_secs);
    println!("  timeout         = {}s", rc.timeout_secs);

    // Child lifecycle phases.
    println!();
    println!("--- Child Lifecycle Phases ---");
    println!();

    let phases = [
        ("Starting", "child is spawned, no heartbeat yet"),
        ("Running", "child emits heartbeats periodically"),
        ("Ready", "child called mark_ready() (if Explicit)"),
        ("Stale", "heartbeat timeout exceeded -> unhealthy"),
        ("Stopped", "child exited normally or was cancelled"),
    ];

    for (phase, desc) in &phases {
        println!("  {phase:12} - {desc}");
    }

    println!();
    println!("=== Summary ===");
    println!("HealthPolicy controls heartbeat interval and stale detection thresholds.");
    println!("ReadinessPolicy::Immediate -> ready on start; Explicit -> requires mark_ready().");
    println!("ReadySignal is thread-safe and can be shared between the task and supervisor.");
}
