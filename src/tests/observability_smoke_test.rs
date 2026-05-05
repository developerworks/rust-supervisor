//! Observability smoke integration tests.
//!
//! These tests verify fan-out from lifecycle events to retained diagnostics.

use rust_supervisor::event::payload::What;
use rust_supervisor::id::types::ChildId;
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
