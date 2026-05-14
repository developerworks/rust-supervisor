//! Metrics facade and low-cardinality label validation.
//!
//! This module defines the stable metric names used by the supervisor core and
//! validates labels before a recorder receives them.

use crate::event::payload::{SupervisorEvent, What};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Stable metric names emitted by the supervisor core.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SupervisorMetricName {
    /// Total child restarts.
    RestartTotal,
    /// Current child state gauge.
    ChildState,
    /// Child uptime in seconds.
    ChildUptimeSeconds,
    /// Restart backoff in seconds.
    BackoffSeconds,
    /// Health check latency in seconds.
    HealthcheckLatencySeconds,
    /// Total meltdown events.
    MeltdownTotal,
    /// Shutdown duration in seconds.
    ShutdownDurationSeconds,
    /// Total event lag.
    EventLagTotal,
    /// Current configuration version.
    ConfigVersion,
    /// Runtime control loop exit counter.
    RuntimeControlLoopExitTotal,
    /// Runtime control plane alive gauge.
    RuntimeControlPlaneAlive,
}

impl SupervisorMetricName {
    /// Returns the wire metric name.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the stable metric name.
    ///
    /// # Examples
    ///
    /// ```
    /// let name = rust_supervisor::observe::metrics::SupervisorMetricName::RestartTotal;
    /// assert_eq!(name.as_str(), "supervisor_restart_total");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RestartTotal => "supervisor_restart_total",
            Self::ChildState => "supervisor_child_state",
            Self::ChildUptimeSeconds => "supervisor_child_uptime_seconds",
            Self::BackoffSeconds => "supervisor_backoff_seconds",
            Self::HealthcheckLatencySeconds => "supervisor_healthcheck_latency_seconds",
            Self::MeltdownTotal => "supervisor_meltdown_total",
            Self::ShutdownDurationSeconds => "supervisor_shutdown_duration_seconds",
            Self::EventLagTotal => "supervisor_event_lag_total",
            Self::ConfigVersion => "supervisor_config_version",
            Self::RuntimeControlLoopExitTotal => "supervisor_runtime_control_loop_exit_total",
            Self::RuntimeControlPlaneAlive => "supervisor_runtime_control_plane_alive",
        }
    }
}

/// Metric sample produced by the facade.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricSample {
    /// Stable metric name.
    pub name: String,
    /// Numeric metric value.
    pub value: f64,
    /// Low-cardinality labels.
    pub labels: BTreeMap<String, String>,
}

impl MetricSample {
    /// Creates a metric sample.
    ///
    /// # Arguments
    ///
    /// - `name`: Stable metric name.
    /// - `value`: Numeric metric value.
    /// - `labels`: Low-cardinality labels.
    ///
    /// # Returns
    ///
    /// Returns a [`MetricSample`].
    pub fn new(name: SupervisorMetricName, value: f64, labels: BTreeMap<String, String>) -> Self {
        Self {
            name: name.as_str().to_owned(),
            value,
            labels,
        }
    }
}

/// Validation error for metric labels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetricLabelError {
    /// Label key that failed validation.
    pub key: String,
    /// Human-readable validation reason.
    pub reason: String,
}

/// Facade that maps lifecycle events to metrics.
#[derive(Debug, Clone)]
pub struct MetricsFacade {
    /// Maximum accepted label value length.
    pub max_label_value_len: usize,
}

impl MetricsFacade {
    /// Creates a metrics facade.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a [`MetricsFacade`] with conservative label validation.
    ///
    /// # Examples
    ///
    /// ```
    /// let facade = rust_supervisor::observe::metrics::MetricsFacade::new();
    /// assert!(facade.validate_label("state", "running").is_ok());
    /// ```
    pub fn new() -> Self {
        Self {
            max_label_value_len: 96,
        }
    }

    /// Validates one low-cardinality metric label.
    ///
    /// # Arguments
    ///
    /// - `key`: Metric label key.
    /// - `value`: Metric label value.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` when the label is accepted.
    pub fn validate_label(&self, key: &str, value: &str) -> Result<(), MetricLabelError> {
        if !allowed_label_key(key) {
            return Err(MetricLabelError {
                key: key.to_owned(),
                reason: "label key is not allowed".to_owned(),
            });
        }
        if value.len() > self.max_label_value_len {
            return Err(MetricLabelError {
                key: key.to_owned(),
                reason: "label value is too long".to_owned(),
            });
        }
        if value.contains('\n') {
            return Err(MetricLabelError {
                key: key.to_owned(),
                reason: "label value contains a newline".to_owned(),
            });
        }
        Ok(())
    }

    /// Maps a lifecycle event into metric samples.
    ///
    /// # Arguments
    ///
    /// - `event`: Lifecycle event emitted by the runtime.
    ///
    /// # Returns
    ///
    /// Returns zero or more metric samples for the event.
    pub fn samples_for_event(&self, event: &SupervisorEvent) -> Vec<MetricSample> {
        match &event.what {
            What::ChildRestarted { .. } => vec![MetricSample::new(
                SupervisorMetricName::RestartTotal,
                1.0,
                labels_for_event(event),
            )],
            What::BackoffScheduled { delay_ms } => vec![MetricSample::new(
                SupervisorMetricName::BackoffSeconds,
                *delay_ms as f64 / 1000.0,
                labels_for_event(event),
            )],
            What::Meltdown { .. } => vec![MetricSample::new(
                SupervisorMetricName::MeltdownTotal,
                1.0,
                labels_for_event(event),
            )],
            What::SubscriberLagged { missed } => vec![MetricSample::new(
                SupervisorMetricName::EventLagTotal,
                *missed as f64,
                labels_for_event(event),
            )],
            What::RuntimeControlLoopStarted { phase, .. } => vec![MetricSample::new(
                SupervisorMetricName::RuntimeControlPlaneAlive,
                1.0,
                runtime_labels_for_event(event, "alive", phase),
            )],
            What::RuntimeControlLoopCompleted { phase, .. } => vec![
                MetricSample::new(
                    SupervisorMetricName::RuntimeControlLoopExitTotal,
                    1.0,
                    runtime_labels_for_event(event, "completed", phase),
                ),
                MetricSample::new(
                    SupervisorMetricName::RuntimeControlPlaneAlive,
                    0.0,
                    runtime_labels_for_event(event, "completed", phase),
                ),
            ],
            What::RuntimeControlLoopFailed { phase, .. } => vec![
                MetricSample::new(
                    SupervisorMetricName::RuntimeControlLoopExitTotal,
                    1.0,
                    runtime_labels_for_event(event, "failed", phase),
                ),
                MetricSample::new(
                    SupervisorMetricName::RuntimeControlPlaneAlive,
                    0.0,
                    runtime_labels_for_event(event, "failed", phase),
                ),
            ],
            _ => Vec::new(),
        }
    }
}

impl Default for MetricsFacade {
    /// Creates the default metrics facade.
    fn default() -> Self {
        Self::new()
    }
}

/// Checks whether a label key is part of the public low-cardinality set.
///
/// # Arguments
///
/// - `key`: Label key to validate.
///
/// # Returns
///
/// Returns `true` when the key is allowed.
fn allowed_label_key(key: &str) -> bool {
    matches!(
        key,
        "supervisor_path" | "child_id" | "state" | "phase" | "decision" | "failure_category"
    )
}

/// Builds labels that are safe to attach to event-derived metrics.
///
/// # Arguments
///
/// - `event`: Lifecycle event used as the label source.
///
/// # Returns
///
/// Returns a map of low-cardinality labels.
fn labels_for_event(event: &SupervisorEvent) -> BTreeMap<String, String> {
    let mut labels = BTreeMap::new();
    labels.insert(
        "supervisor_path".to_owned(),
        event.r#where.supervisor_path.to_string(),
    );
    if let Some(child_id) = &event.r#where.child_id {
        labels.insert("child_id".to_owned(), child_id.to_string());
    }
    if let Some(policy) = &event.policy {
        labels.insert("decision".to_owned(), policy.decision.clone());
    }
    labels
}

/// Builds labels for runtime control plane metrics.
fn runtime_labels_for_event(
    event: &SupervisorEvent,
    state: &str,
    phase: &str,
) -> BTreeMap<String, String> {
    let mut labels = labels_for_event(event);
    labels.insert("state".to_owned(), state.to_owned());
    labels.insert("phase".to_owned(), phase.to_owned());
    labels
}
