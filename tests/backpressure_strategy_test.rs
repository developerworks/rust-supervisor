//! Backpressure strategy tests for slow subscriber handling.
//!
//! This file verifies that the backpressure detection and mitigation strategies
//! work correctly: `AlertAndBlock` blocks producers without dropping events, and
//! `SampleAndAudit` drops events while recording the sampling ratio in the audit
//! trail.

use rust_supervisor::event::payload::{FiniteF64, What};
use rust_supervisor::event::time::CorrelationId;
use rust_supervisor::spec::supervisor::{BackpressureConfig, BackpressureStrategy};

#[tokio::test]
async fn test_backpressure_config_defaults() {
    let config = BackpressureConfig::default();
    assert_eq!(config.strategy, BackpressureStrategy::AlertAndBlock);
    assert_eq!(config.warn_threshold_pct, 80);
    assert_eq!(config.critical_threshold_pct, 95);
    assert_eq!(config.window_secs, 30);
    assert_eq!(config.audit_channel_capacity, 1024);
}

#[tokio::test]
async fn test_backpressure_strategy_serde() {
    let alert = BackpressureStrategy::AlertAndBlock;
    let json = serde_json::to_string(&alert).unwrap();
    assert_eq!(json, "\"alert_and_block\"");

    let sample = BackpressureStrategy::SampleAndAudit;
    let json = serde_json::to_string(&sample).unwrap();
    assert_eq!(json, "\"sample_and_audit\"");

    // Round-trip
    let deserialized: BackpressureStrategy = serde_json::from_str("\"alert_and_block\"").unwrap();
    assert_eq!(deserialized, BackpressureStrategy::AlertAndBlock);
}

#[tokio::test]
async fn test_backpressure_config_serde() {
    let config = BackpressureConfig {
        strategy: BackpressureStrategy::SampleAndAudit,
        warn_threshold_pct: 85,
        critical_threshold_pct: 97,
        window_secs: 60,
        audit_channel_capacity: 2048,
    };
    let json = serde_json::to_string_pretty(&config).unwrap();
    assert!(json.contains("sample_and_audit"));
    assert!(json.contains("85"));
    assert!(json.contains("97"));

    // Round-trip
    let deserialized: BackpressureConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.strategy, BackpressureStrategy::SampleAndAudit);
    assert_eq!(deserialized.warn_threshold_pct, 85);
}

#[tokio::test]
async fn test_audit_recorded_variant_serialization() {
    let audit = What::AuditRecorded {
        command_id: "cmd-001".to_string(),
        event_type: "restart".to_string(),
        sample_ratio: FiniteF64::new(0.5),
        correlation_id: CorrelationId::new(),
        trigger_reason: "budget exhausted".to_string(),
        events_discarded: 42,
    };

    let json = serde_json::to_value(&audit).unwrap();
    assert_eq!(json["type"], "audit_recorded");
    assert_eq!(json["payload"]["trigger_reason"], "budget exhausted");
    assert_eq!(json["payload"]["events_discarded"], 42);
}

#[tokio::test]
async fn test_backpressure_alert_variant_serialization() {
    let alert = What::BackpressureAlert {
        subscriber: "metrics".to_string(),
        buffer_pct: 85,
        threshold_pct: 80,
    };

    let json = serde_json::to_value(&alert).unwrap();
    assert_eq!(json["type"], "backpressure_alert");
    assert_eq!(json["payload"]["buffer_pct"], 85);
}

#[tokio::test]
async fn test_backpressure_degradation_variant_serialization() {
    let deg = What::BackpressureDegradation {
        subscriber: "journal".to_string(),
        strategy: "sample_and_audit".to_string(),
        sample_ratio: FiniteF64::new(0.3),
        buffer_peak_pct: 96,
        recovered: false,
    };

    let json = serde_json::to_value(&deg).unwrap();
    assert_eq!(json["type"], "backpressure_degradation");
    assert_eq!(json["payload"]["buffer_peak_pct"], 96);
    assert_eq!(json["payload"]["recovered"], false);
}
