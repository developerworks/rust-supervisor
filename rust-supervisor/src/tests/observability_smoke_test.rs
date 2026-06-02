//! Observability smoke integration tests.
//!
//! These tests verify fan-out from lifecycle events to retained diagnostics.

use rust_supervisor::event::payload::What;
use rust_supervisor::id::types::{ChildId, ChildStartCount, Generation};
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

/// Verifies shutdown pipeline events reach metric and audit sinks.
#[test]
fn observability_pipeline_records_shutdown_pipeline_events() {
    let fixture = EventFixture::new(PausedTime::new(1, 2, 3), 1);
    let mut pipeline = ObservabilityPipeline::new(8, 2);

    emit_shutdown_pipeline_events(&fixture, &mut pipeline);

    assert_shutdown_metrics(&pipeline);
    assert_shutdown_audit_records(&pipeline);
}

/// Emits representative shutdown pipeline events.
fn emit_shutdown_pipeline_events(fixture: &EventFixture, pipeline: &mut ObservabilityPipeline) {
    let generation = Generation::initial();
    let child_start_count = ChildStartCount::first();
    let worker_id = ChildId::new("worker");
    let slow_worker_id = ChildId::new("slow-worker");
    let late_worker_id = ChildId::new("late-worker");

    pipeline.emit(fixture.child_event(
        worker_id.clone(),
        "worker",
        What::ChildShutdownCancelDelivered {
            child_id: worker_id.clone(),
            generation,
            child_start_count,
            phase: "RequestStop".to_owned(),
        },
    ));
    pipeline.emit(fixture.child_event(
        worker_id.clone(),
        "worker",
        What::ChildShutdownGraceful {
            child_id: worker_id.clone(),
            generation,
            child_start_count,
            phase: "GracefulDrain".to_owned(),
            exit: "succeeded".to_owned(),
        },
    ));
    pipeline.emit(fixture.child_event(
        slow_worker_id.clone(),
        "slow-worker",
        What::ChildShutdownAborted {
            child_id: slow_worker_id.clone(),
            generation,
            child_start_count,
            phase: "AbortStragglers".to_owned(),
            result: "aborted".to_owned(),
            reason: "graceful_timeout".to_owned(),
        },
    ));
    pipeline.emit(fixture.child_event(
        late_worker_id.clone(),
        "late-worker",
        What::ChildShutdownLateReport {
            child_id: late_worker_id.clone(),
            generation,
            child_start_count,
            phase: "Reconcile".to_owned(),
            exit: "cancelled".to_owned(),
        },
    ));
    pipeline.emit(fixture.supervisor_event(What::ShutdownCompleted {
        phase: "Completed".to_owned(),
        result: "completed".to_owned(),
        duration_ms: 2750,
    }));
}

/// Asserts metric samples derived from shutdown events.
fn assert_shutdown_metrics(pipeline: &ObservabilityPipeline) {
    assert!(pipeline.test_recorder.metrics.iter().any(|sample| {
        sample.name == SupervisorMetricName::ShutdownDurationSeconds.as_str()
            && (sample.value - 2.75).abs() < f64::EPSILON
            && sample
                .labels
                .get("phase")
                .is_some_and(|value| value == "Completed")
    }));
    assert!(pipeline.test_recorder.metrics.iter().any(|sample| {
        sample.name == SupervisorMetricName::ShutdownAbortTotal.as_str()
            && sample
                .labels
                .get("reason")
                .is_some_and(|value| value == "timeout")
    }));
    assert!(pipeline.test_recorder.metrics.iter().any(|sample| {
        sample.name == SupervisorMetricName::ShutdownLateReportsTotal.as_str()
            && sample
                .labels
                .get("phase")
                .is_some_and(|value| value == "Reconcile")
    }));

    let outcome_samples = pipeline
        .test_recorder
        .metrics
        .iter()
        .filter(|sample| sample.name == SupervisorMetricName::ShutdownChildOutcomesTotal.as_str())
        .collect::<Vec<_>>();
    assert_eq!(outcome_samples.len(), 3);
    assert!(
        outcome_samples
            .iter()
            .all(|sample| !sample.labels.contains_key("child_id"))
    );
    assert!(outcome_samples.iter().any(|sample| {
        sample
            .labels
            .get("status")
            .is_some_and(|value| value == "graceful")
    }));
}

/// Asserts audit records derived from shutdown events.
fn assert_shutdown_audit_records(pipeline: &ObservabilityPipeline) {
    assert!(pipeline.test_recorder.audits.iter().any(|record| {
        record.result == "cancel_delivered"
            && record.child_id.as_deref() == Some("worker")
            && record.phase == "RequestStop"
    }));
    assert!(pipeline.test_recorder.audits.iter().any(|record| {
        record.result == "aborted"
            && record.child_id.as_deref() == Some("slow-worker")
            && record.phase == "AbortStragglers"
            && record
                .context
                .get("generation")
                .is_some_and(|value| value == "0")
    }));
    assert!(pipeline.test_recorder.audits.iter().any(|record| {
        record.result == "late_report"
            && record.child_id.as_deref() == Some("late-worker")
            && record
                .context
                .get("exit")
                .is_some_and(|value| value == "cancelled")
    }));
    assert!(pipeline.test_recorder.audits.iter().any(|record| {
        record.result == "completed"
            && record.phase == "Completed"
            && record
                .context
                .get("duration_ms")
                .is_some_and(|value| value == "2750")
    }));
}
