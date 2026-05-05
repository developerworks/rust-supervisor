//! Event and log conversion for dashboard IPC.
//!
//! Target processes keep recent lifecycle facts in [`crate::journal::ring`].
//! This module turns those facts into dashboard records with target identity,
//! sequence, correlation, and dropped-count metadata.

use crate::dashboard::model::{EventRecord, LogRecord};
use crate::event::payload::{SupervisorEvent, What};
use crate::journal::ring::EventJournal;
use serde_json::json;
use std::collections::BTreeMap;

/// Converts one supervisor event into a dashboard event record.
///
/// # Arguments
///
/// - `target_id`: Target process identifier.
/// - `config_version`: Configuration version string.
/// - `event`: Supervisor lifecycle event.
///
/// # Returns
///
/// Returns an [`EventRecord`] preserving sequence and correlation fields.
pub fn event_to_record(
    target_id: impl Into<String>,
    config_version: impl Into<String>,
    event: &SupervisorEvent,
) -> EventRecord {
    let target_id = target_id.into();
    let target_path = event.r#where.supervisor_path.to_string();
    EventRecord {
        target_id,
        sequence: event.sequence.value,
        correlation_id: event.correlation_id.value.to_string(),
        event_type: event.what.name().to_owned(),
        severity: severity_for_event(&event.what).to_owned(),
        target_path,
        child_id: event.r#where.child_id.as_ref().map(ToString::to_string),
        occurred_at_unix_nanos: event.when.time.unix_nanos,
        config_version: config_version.into(),
        payload: json!({
            "event_type": event.what.name(),
            "policy": event.policy,
        }),
    }
}

/// Converts recent supervisor events into dashboard event records.
///
/// # Arguments
///
/// - `target_id`: Target process identifier.
/// - `config_version`: Configuration version string.
/// - `journal`: Event journal that owns recent events.
/// - `limit`: Maximum number of events to convert.
///
/// # Returns
///
/// Returns recent events in oldest-to-newest order.
pub fn journal_to_event_records(
    target_id: impl Into<String>,
    config_version: impl Into<String>,
    journal: &EventJournal,
    limit: usize,
) -> Vec<EventRecord> {
    let target_id = target_id.into();
    let config_version = config_version.into();
    journal
        .recent(limit)
        .iter()
        .map(|event| event_to_record(target_id.clone(), config_version.clone(), event))
        .collect()
}

/// Builds a log record associated with a dashboard event.
///
/// # Arguments
///
/// - `event`: Event whose sequence and correlation should be reused.
/// - `message`: Human-readable log message.
///
/// # Returns
///
/// Returns a [`LogRecord`] correlated to the event.
pub fn log_record_for_event(event: &EventRecord, message: impl Into<String>) -> LogRecord {
    let mut fields = BTreeMap::new();
    fields.insert("event_type".to_owned(), event.event_type.clone());
    fields.insert("target_path".to_owned(), event.target_path.clone());
    LogRecord {
        target_id: event.target_id.clone(),
        sequence: Some(event.sequence),
        correlation_id: Some(event.correlation_id.clone()),
        severity: event.severity.clone(),
        message: message.into(),
        fields,
        occurred_at_unix_nanos: event.occurred_at_unix_nanos,
    }
}

/// Returns the severity label for an event payload.
///
/// # Arguments
///
/// - `what`: Event payload.
///
/// # Returns
///
/// Returns a stable dashboard severity label.
fn severity_for_event(what: &What) -> &'static str {
    match what {
        What::ChildFailed { .. }
        | What::ChildPanicked { .. }
        | What::Meltdown { .. }
        | What::ChildUnhealthy { .. } => "error",
        What::BackoffScheduled { .. }
        | What::ChildRestarting { .. }
        | What::ChildRestarted { .. }
        | What::ChildQuarantined { .. }
        | What::SubscriberLagged { .. } => "warning",
        _ => "info",
    }
}
