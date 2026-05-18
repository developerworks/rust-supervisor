//! Tracing signal records for lifecycle events.
//!
//! The module defines project-owned tracing data that can be sent to a concrete
//! tracing subscriber by higher-level runtime code.

use crate::event::payload::SupervisorEvent;
use serde::{Deserialize, Serialize};

/// Tracing span metadata for a child child_start_count.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChildStartCountSpan {
    /// Span name used for tracing output.
    pub name: String,
    /// Event sequence that opened the span.
    pub sequence: u64,
    /// Correlation identifier shared with the event.
    pub correlation_id: String,
    /// Child identifier when the event has one.
    pub child_id: Option<String>,
}

impl ChildStartCountSpan {
    /// Creates span metadata from a supervisor event.
    ///
    /// # Arguments
    ///
    /// - `event`: Lifecycle event that starts or describes an child_start_count.
    ///
    /// # Returns
    ///
    /// Returns an [`ChildStartCountSpan`] value.
    ///
    /// # Examples
    ///
    /// ```
    /// let time = rust_supervisor::event::time::EventTime::deterministic(
    ///     1,
    ///     1,
    ///     0,
    ///     rust_supervisor::id::types::Generation::initial(),
    ///     rust_supervisor::id::types::ChildStartCount::first(),
    /// );
    /// let event = rust_supervisor::event::payload::SupervisorEvent::new(
    ///     rust_supervisor::event::time::When::new(time),
    ///     rust_supervisor::event::payload::Where::new(
    ///         rust_supervisor::id::types::SupervisorPath::root(),
    ///     ),
    ///     rust_supervisor::event::payload::What::ChildStarting { transition: None },
    ///     rust_supervisor::event::time::EventSequence::new(1),
    ///     rust_supervisor::event::time::CorrelationId::from_uuid(uuid::Uuid::nil()),
    ///     1,
    /// );
    /// let span = rust_supervisor::observe::tracing::ChildStartCountSpan::from_event(&event);
    /// assert_eq!(span.name, "supervisor.child_child_start_count");
    /// ```
    pub fn from_event(event: &SupervisorEvent) -> Self {
        Self {
            name: "supervisor.child_child_start_count".to_owned(),
            sequence: event.sequence.value,
            correlation_id: event.correlation_id.value.to_string(),
            child_id: event
                .r#where
                .child_id
                .as_ref()
                .map(std::string::ToString::to_string),
        }
    }
}

/// Tracing event metadata derived from a lifecycle event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TracingEvent {
    /// Tracing event name.
    pub name: String,
    /// Event sequence.
    pub sequence: u64,
    /// Correlation identifier shared with other observability signals.
    pub correlation_id: String,
    /// Event payload name.
    pub payload: String,
}

impl TracingEvent {
    /// Creates tracing event metadata from a supervisor event.
    ///
    /// # Arguments
    ///
    /// - `event`: Lifecycle event to translate.
    ///
    /// # Returns
    ///
    /// Returns a [`TracingEvent`] value.
    pub fn from_event(event: &SupervisorEvent) -> Self {
        Self {
            name: "supervisor.lifecycle_event".to_owned(),
            sequence: event.sequence.value,
            correlation_id: event.correlation_id.value.to_string(),
            payload: event.what.name().to_owned(),
        }
    }
}
