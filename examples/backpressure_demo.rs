//! Demonstrates backpressure strategies for slow event subscribers.
//!
//! The supervisor supports two backpressure strategies:
//!   - AlertAndBlock: warns at soft threshold, blocks producer at hard threshold.
//!   - SampleAndAudit: samples and discards events when buffers fill up, records
//!     the discard ratio in the audit trail.
//!
//! This example constructs both strategy configurations and shows how buffer
//! occupancy thresholds trigger alerts and degradation.

use rust_supervisor::observe::pipeline::{ObservabilityPipeline, TestRecorder};
use rust_supervisor::spec::supervisor::{BackpressureConfig, BackpressureStrategy};

/// Runs the backpressure demonstration.
fn main() {
    println!("=== Backpressure Strategy Demo ===");
    println!();

    // Build two backpressure configurations.
    let alert_block = BackpressureConfig {
        strategy: BackpressureStrategy::AlertAndBlock,
        warn_threshold_pct: 80,
        critical_threshold_pct: 95,
        window_secs: 30,
        audit_channel_capacity: 1024,
    };

    // Build the sampling backpressure configuration.
    let sample_audit = BackpressureConfig {
        strategy: BackpressureStrategy::SampleAndAudit,
        warn_threshold_pct: 70,
        critical_threshold_pct: 90,
        window_secs: 30,
        audit_channel_capacity: 2048,
    };

    // Print the blocking strategy details.
    println!("--- AlertAndBlock (default) ---");
    println!(
        "  warn_threshold       = {}%",
        alert_block.warn_threshold_pct
    );
    println!(
        "  critical_threshold   = {}%",
        alert_block.critical_threshold_pct
    );
    println!("  window               = {}s", alert_block.window_secs);
    println!(
        "  audit_capacity       = {}",
        alert_block.audit_channel_capacity
    );
    println!("  behavior at warn:    emit backpressure alert");
    println!("  behavior at crit:    block producer until subscriber catches up");
    println!();

    // Print the sampling strategy details.
    println!("--- SampleAndAudit ---");
    println!(
        "  warn_threshold       = {}%",
        sample_audit.warn_threshold_pct
    );
    println!(
        "  critical_threshold   = {}%",
        sample_audit.critical_threshold_pct
    );
    println!("  window               = {}s", sample_audit.window_secs);
    println!(
        "  audit_capacity       = {}",
        sample_audit.audit_channel_capacity
    );
    println!("  behavior at warn:    start sampling events, record ratio in audit trail");
    println!("  behavior at crit:    increase sampling rate, record degradation");
    println!();

    // Demonstrate building an observability pipeline.
    println!("--- Pipeline Construction ---");
    println!();

    // Build a small observability pipeline.
    let _pipeline = ObservabilityPipeline::new(16, 16);
    println!("  pipeline created with journal_capacity=16, subscriber_capacity=16");

    // Pipeline is immutable for external callers; subscribers are added
    // during construction. The pipeline owns its journal and subscriber list.

    // Demonstrate TestRecorder for recording backpressure events.
    println!();
    println!("--- TestRecorder (lag recording) ---");
    println!();

    // Record a sample subscriber lag event.
    let mut recorder = TestRecorder::new();
    recorder.record_lag(5);
    println!("  recorded subscriber lag of 5 events");

    // Print the strategy summary.
    println!();
    println!("=== Summary ===");
    println!("AlertAndBlock   -> safe default, never drops events, blocks producers.");
    println!("SampleAndAudit  -> production choice under high volume, drops under pressure.");
    println!("Both strategies emit BackpressureAlert and BackpressureDegradation events.");
}
