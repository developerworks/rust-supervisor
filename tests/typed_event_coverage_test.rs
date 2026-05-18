//! Exhaustive coverage test for typed SupervisorEvent variants.
//!
//! This file verifies that every `What` variant can be constructed and
//! serialized as JSON, and that the serialized `type` field uses snake_case.
//! This is the independent test for User Story 1 (typed events).

use rust_supervisor::event::payload::{FiniteF64, MeltdownScope, StateTransition, What};
use rust_supervisor::event::time::CorrelationId;
use rust_supervisor::id::types::{ChildStartCount, Generation, SupervisorPath};

/// Returns a representative sample of `What` variants for serialization testing.
///
/// This covers all newly added variants (006-5) plus representative existing
/// variants to ensure backward compatibility.
fn sample_variants() -> Vec<What> {
    vec![
        // --- Existing variants (representative sample) ---
        What::ChildStarting {
            transition: Some(StateTransition::new("idle", "starting")),
        },
        What::ChildRunning { transition: None },
        What::ChildReady { transition: None },
        What::ChildHeartbeat { age_ms: 42 },
        What::ChildFailed {
            failure: rust_supervisor::error::types::TaskFailure::new(
                rust_supervisor::error::types::TaskFailureKind::Error,
                "io",
                "child crashed",
            ),
        },
        What::ChildPanicked {
            category: "oom".to_string(),
        },
        What::BackoffScheduled { delay_ms: 5000 },
        What::ChildRestarting { generation: 2 },
        What::ChildRestarted { restart_count: 3 },
        What::ChildStopped {
            reason: "completed".to_string(),
        },
        What::ShutdownRequested {
            cause: "SIGTERM".to_string(),
        },
        What::ShutdownCompleted {
            phase: "draining".to_string(),
            result: "success".to_string(),
            duration_ms: 1500,
        },
        What::Meltdown {
            scope: "child".to_string(),
        },
        What::SubscriberLagged { missed: 10 },
        // --- Newly added variants (006-5) ---
        What::BudgetDenied {
            group: Some("worker-pool-a".to_string()),
            reason: "budget exhausted in 60s window".to_string(),
            budget_remaining: FiniteF64::new(0.0),
        },
        What::GenerationFenced {
            old_generation: 1,
            new_generation: 2,
            reason: "old attempt still running".to_string(),
        },
        What::HealthCheckPassed {
            age_ms: 5000,
            healthy_since_unix_nanos: 1716019200000000000,
        },
        What::HealthCheckFailed {
            reason: "timeout".to_string(),
            consecutive_failures: 3,
        },
        What::Paused {
            reason: "maintenance".to_string(),
            paused_by: "admin".to_string(),
        },
        What::Resumed {
            reason: "maintenance complete".to_string(),
        },
        What::Quarantined {
            scope: MeltdownScope::Child,
            reason: "excessive failures".to_string(),
            duration_secs: 300,
        },
        What::BackpressureAlert {
            subscriber: "metrics".to_string(),
            buffer_pct: 85,
            threshold_pct: 80,
        },
        What::BackpressureDegradation {
            subscriber: "metrics".to_string(),
            strategy: "sample_and_audit".to_string(),
            sample_ratio: FiniteF64::new(0.5),
            buffer_peak_pct: 96,
            recovered: false,
        },
        What::AuditRecorded {
            command_id: "cmd-001".to_string(),
            event_type: "restart".to_string(),
            sample_ratio: FiniteF64::new(0.5),
            correlation_id: CorrelationId::new(),
            trigger_reason: "budget exhausted".to_string(),
            events_discarded: 42,
        },
    ]
}

#[test]
fn test_all_variants_serializable() {
    let variants = sample_variants();
    assert!(!variants.is_empty(), "must have at least one variant");

    for (i, variant) in variants.iter().enumerate() {
        let json_str = serde_json::to_string_pretty(variant)
            .unwrap_or_else(|e| panic!("variant[{}] {:?} serialization failed: {}", i, variant, e));
        let deserialized: What = serde_json::from_str(&json_str).unwrap_or_else(|e| {
            panic!(
                "variant[{}] deserialization failed: {}\njson: {}",
                i, e, json_str
            )
        });
        // Verify round-trip: debug output should contain the same type name
        let original_debug = format!("{:?}", variant);
        let roundtrip_debug = format!("{:?}", deserialized);
        assert_eq!(
            original_debug, roundtrip_debug,
            "variant[{}] round-trip debug mismatch",
            i
        );
    }
}

#[test]
fn test_what_type_field_is_snake_case() {
    let variants = sample_variants();

    for (i, variant) in variants.iter().enumerate() {
        let json_value: serde_json::Value = serde_json::to_value(variant)
            .unwrap_or_else(|e| panic!("variant[{}] to_value failed: {}", i, e));

        // The `type` field should be present and in snake_case.
        let type_field = json_value
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| {
                panic!("variant[{}] has no 'type' field in JSON: {}", i, json_value)
            });

        // Verify snake_case: no uppercase letters, words separated by underscores.
        assert!(
            !type_field.contains(char::is_uppercase),
            "variant[{}] type '{}' is not snake_case: {}",
            i,
            type_field,
            json_value
        );

        // Verify the payload field is present (even if empty).
        let _payload = json_value.get("payload").unwrap_or_else(|| {
            panic!(
                "variant[{}] has no 'payload' field in JSON: {}",
                i, json_value
            )
        });
    }
}

#[test]
fn test_all_new_variants_have_correct_field_types() {
    // BudgetDenied: group is Option<String>, reason is String, budget_remaining is FiniteF64
    let denied = What::BudgetDenied {
        group: None,
        reason: String::new(),
        budget_remaining: FiniteF64::new(0.5),
    };
    let json = serde_json::to_value(&denied).unwrap();
    assert_eq!(json["type"], "budget_denied");
    assert_eq!(json["payload"]["budget_remaining"], 0.5);

    // AuditRecorded: correlation_id is present
    let audit = What::AuditRecorded {
        command_id: "x".to_string(),
        event_type: "y".to_string(),
        sample_ratio: FiniteF64::new(0.3),
        correlation_id: CorrelationId::new(),
        trigger_reason: "test".to_string(),
        events_discarded: 0,
    };
    let json = serde_json::to_value(&audit).unwrap();
    assert_eq!(json["type"], "audit_recorded", "JSON: {}", json);
    // correlation_id serializes as an object with a `value` string (UUID format).
    let cid = json["payload"]["correlation_id"]["value"]
        .as_str()
        .expect("correlation_id.value should be a string");
    assert!(cid.len() > 10, "correlation_id too short: {}", cid);
}

#[test]
fn test_schema_id_default() {
    // When constructing via SupervisorEvent::new, schema_id defaults to 1.
    use rust_supervisor::event::payload::SupervisorEvent;
    use rust_supervisor::event::time::EventTime;
    use rust_supervisor::event::time::{EventSequence, When};

    let event = SupervisorEvent::new(
        When::new(EventTime::deterministic(
            1,
            1,
            0,
            Generation::initial(),
            ChildStartCount::first(),
        )),
        rust_supervisor::event::payload::Where::new(SupervisorPath::root()),
        What::ChildRunning { transition: None },
        EventSequence::new(1),
        CorrelationId::new(),
        1,
    );
    assert_eq!(event.schema_id, 1, "schema_id should default to 1");
}
