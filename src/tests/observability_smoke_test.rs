//! Observability smoke integration tests.
//!
//! These tests verify fan-out from lifecycle events to retained diagnostics.

use rust_supervisor::event::payload::What;
use rust_supervisor::id::types::ChildId;
use rust_supervisor::observe::metrics::SupervisorMetricName;
use rust_supervisor::observe::pipeline::ObservabilityPipeline;
use rust_supervisor::test_support::assertions::assert_recorder_has_metrics;
use rust_supervisor::test_support::factory::{EventFixture, PausedTime};

/// Verifies that one restart event reaches journal, metrics, and subscribers.
#[test]
fn observability_pipeline_records_restart_event() {
    let fixture = EventFixture::new(PausedTime::new(1, 2, 3), 1);
    let event = fixture.child_event(
        ChildId::new("worker"),
        "worker",
        What::ChildRestarted { restart_count: 1 },
    );
    let mut pipeline = ObservabilityPipeline::new(8, 2);
    let subscriber = pipeline.add_subscriber();

    let lag = pipeline.emit(event);

    assert_eq!(lag, 0);
    assert_eq!(pipeline.journal.len(), 1);
    assert_eq!(pipeline.drain_subscriber(subscriber).len(), 1);
    assert_recorder_has_metrics(&pipeline.test_recorder);
}

/// Verifies that a control-plane failure reaches metrics and audit records.
#[test]
fn observability_pipeline_records_runtime_control_plane_failure() {
    let fixture = EventFixture::new(PausedTime::new(1, 2, 3), 1);
    let event = fixture.supervisor_event(What::RuntimeControlLoopFailed {
        phase: "watchdog".to_owned(),
        reason: "runtime control loop panic".to_owned(),
        panic: true,
        recoverable: true,
    });
    let mut pipeline = ObservabilityPipeline::new(8, 2);

    pipeline.emit(event);

    assert!(
        pipeline
            .test_recorder
            .logs
            .iter()
            .any(|record| { record.event_name == "RuntimeControlLoopFailed" })
    );
    assert!(pipeline.test_recorder.metrics.iter().any(|sample| {
        sample.name == SupervisorMetricName::RuntimeControlLoopExitTotal.as_str()
    }));
    assert!(pipeline.test_recorder.metrics.iter().any(|sample| {
        sample.name == SupervisorMetricName::RuntimeControlPlaneAlive.as_str()
            && sample.value == 0.0
    }));
    assert!(
        pipeline
            .test_recorder
            .audits
            .iter()
            .any(|record| { record.result == "failed" && record.phase == "watchdog" })
    );
}
