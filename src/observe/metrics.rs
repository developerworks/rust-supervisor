//! Metrics facade and low-cardinality label validation.
//!
//! This module defines the stable metric names used by the supervisor core and
//! validates labels before a recorder receives them.

use crate::control::outcome::StaleReportHandling;
use crate::event::payload::{SupervisorEvent, What};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

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
    /// Total child shutdown outcomes.
    ShutdownChildOutcomesTotal,
    /// Total shutdown abort outcomes.
    ShutdownAbortTotal,
    /// Total late child shutdown reports.
    ShutdownLateReportsTotal,
    /// Total event lag.
    EventLagTotal,
    /// Current configuration version.
    ConfigVersion,
    /// Runtime control loop exit counter.
    RuntimeControlLoopExitTotal,
    /// Runtime control plane alive gauge.
    RuntimeControlPlaneAlive,
    /// Total child control commands.
    ChildControlCommandTotal,
    /// Remaining restart limit for child runtime state.
    ChildRuntimeRestartLimitRemaining,
    /// Total stale heartbeat observations.
    ChildRuntimeHeartbeatStaleTotal,
    /// Total child control operation transitions.
    ChildRuntimeOperationTransitionsTotal,
    /// Total generation fence lifecycle transitions for manual restart isolation.
    ChildRestartFenceTotal,
    /// Total stale completion reports classified outside authoritative triples.
    ChildAttemptStaleReportTotal,
    /// Observed pending restart gauge mirrored from lifecycle signals.
    ChildRestartPendingTotal,
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
            Self::ShutdownChildOutcomesTotal => "supervisor_shutdown_child_outcomes_total",
            Self::ShutdownAbortTotal => "supervisor_shutdown_abort_total",
            Self::ShutdownLateReportsTotal => "supervisor_shutdown_late_reports_total",
            Self::EventLagTotal => "supervisor_event_lag_total",
            Self::ConfigVersion => "supervisor_config_version",
            Self::RuntimeControlLoopExitTotal => "supervisor_runtime_control_loop_exit_total",
            Self::RuntimeControlPlaneAlive => "supervisor_runtime_control_plane_alive",
            Self::ChildControlCommandTotal => "supervisor_child_control_command_total",
            Self::ChildRuntimeRestartLimitRemaining => {
                "supervisor_child_runtime_restart_limit_remaining"
            }
            Self::ChildRuntimeHeartbeatStaleTotal => {
                "supervisor_child_runtime_heartbeat_stale_total"
            }
            Self::ChildRuntimeOperationTransitionsTotal => {
                "supervisor_child_runtime_operation_transitions_total"
            }
            Self::ChildRestartFenceTotal => "supervisor_child_restart_fence_total",
            Self::ChildAttemptStaleReportTotal => "supervisor_child_attempt_stale_report_total",
            Self::ChildRestartPendingTotal => "supervisor_child_restart_pending_total",
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
pub struct MetricsFacade {
    /// Maximum accepted label value length.
    pub max_label_value_len: usize,
    /// Best-effort mirror of pending restart depth for gauge export.
    restart_pending_total: Arc<AtomicI64>,
}

impl Clone for MetricsFacade {
    /// Clones this metrics facade while sharing the pending restart gauge counter.
    fn clone(&self) -> Self {
        Self {
            max_label_value_len: self.max_label_value_len,
            restart_pending_total: Arc::clone(&self.restart_pending_total),
        }
    }
}

impl std::fmt::Debug for MetricsFacade {
    /// Renders diagnostic information for this metrics facade.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MetricsFacade")
            .field("max_label_value_len", &self.max_label_value_len)
            .field(
                "restart_pending_total",
                &self.restart_pending_total.load(Ordering::SeqCst),
            )
            .finish()
    }
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
            restart_pending_total: Arc::new(AtomicI64::new(0)),
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

    /// Applies a bounded adjustment to the mirrored pending restart gauge counter.
    ///
    /// # Arguments
    ///
    /// - `delta`: Signed delta applied to the gauge counter (never drops below zero).
    ///
    /// # Returns
    ///
    /// Returns the gauge counter after applying `delta`.
    fn adjust_restart_pending_total(&self, delta: i64) -> i64 {
        let mut next = self
            .restart_pending_total
            .load(Ordering::SeqCst)
            .saturating_add(delta);
        if next < 0 {
            next = 0;
        }
        self.restart_pending_total.store(next, Ordering::SeqCst);
        next
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
            What::ShutdownCompleted {
                phase,
                result,
                duration_ms,
            } => vec![MetricSample::new(
                SupervisorMetricName::ShutdownDurationSeconds,
                *duration_ms as f64 / 1000.0,
                shutdown_completed_labels_for_event(event, phase, result),
            )],
            What::ChildShutdownGraceful { phase, .. } => vec![MetricSample::new(
                SupervisorMetricName::ShutdownChildOutcomesTotal,
                1.0,
                shutdown_child_outcome_labels_for_event(event, "graceful", phase),
            )],
            What::ChildShutdownAborted {
                phase,
                result,
                reason,
                ..
            } => shutdown_aborted_samples(event, phase, result, reason),
            What::ChildShutdownLateReport { phase, .. } => {
                shutdown_late_report_samples(event, phase)
            }
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
            What::RuntimeControlLoopCompleted { phase, .. } => {
                runtime_terminal_samples(event, "completed", phase)
            }
            What::RuntimeControlLoopFailed { phase, .. } => {
                runtime_terminal_samples(event, "failed", phase)
            }
            What::ChildControlCommandCompleted {
                command, result, ..
            } => vec![MetricSample::new(
                SupervisorMetricName::ChildControlCommandTotal,
                1.0,
                child_control_command_labels(command, result),
            )],
            What::ChildControlOperationChanged { from, to, .. } => vec![MetricSample::new(
                SupervisorMetricName::ChildRuntimeOperationTransitionsTotal,
                1.0,
                operation_transition_labels(format!("{from:?}"), format!("{to:?}")),
            )],
            What::ChildRuntimeRestartLimitUpdated {
                child_id,
                restart_limit,
            } => vec![MetricSample::new(
                SupervisorMetricName::ChildRuntimeRestartLimitRemaining,
                restart_limit.remaining as f64,
                restart_limit_labels(child_id),
            )],
            What::ChildRestartFenceEntered { .. } => {
                let gauge = self.adjust_restart_pending_total(1);
                vec![
                    MetricSample::new(
                        SupervisorMetricName::ChildRestartFenceTotal,
                        1.0,
                        child_restart_fence_labels(event, "entered"),
                    ),
                    MetricSample::new(
                        SupervisorMetricName::ChildRestartPendingTotal,
                        gauge as f64,
                        restart_pending_gauge_labels(),
                    ),
                ]
            }
            What::ChildRestartFenceAbortRequested { .. } => vec![MetricSample::new(
                SupervisorMetricName::ChildRestartFenceTotal,
                1.0,
                child_restart_fence_labels(event, "abort_requested"),
            )],
            What::ChildRestartFenceReleased { .. } => vec![MetricSample::new(
                SupervisorMetricName::ChildRestartFenceTotal,
                1.0,
                child_restart_fence_labels(event, "released"),
            )],
            What::ChildRestartFencePendingDrained { .. } => {
                let gauge = self.adjust_restart_pending_total(-1);
                vec![MetricSample::new(
                    SupervisorMetricName::ChildRestartPendingTotal,
                    gauge as f64,
                    restart_pending_gauge_labels(),
                )]
            }
            What::ChildRestartConflict { decision, .. } => {
                let fence_bucket = if decision == "already_pending" {
                    "already_pending"
                } else {
                    "rejected"
                };
                vec![MetricSample::new(
                    SupervisorMetricName::ChildRestartFenceTotal,
                    1.0,
                    child_restart_fence_labels(event, fence_bucket),
                )]
            }
            What::ChildAttemptStaleReport { handled_as, .. } => vec![MetricSample::new(
                SupervisorMetricName::ChildAttemptStaleReportTotal,
                1.0,
                stale_report_metric_labels(*handled_as),
            )],
            What::ChildHeartbeatStale { .. } => vec![MetricSample::new(
                SupervisorMetricName::ChildRuntimeHeartbeatStaleTotal,
                1.0,
                BTreeMap::new(),
            )],
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
        "supervisor_path"
            | "child_id"
            | "state"
            | "phase"
            | "status"
            | "result"
            | "reason"
            | "decision"
            | "failure_category"
            | "command"
            | "from"
            | "to"
            | "handled_as"
    )
}

/// Builds an empty label map for pending restart gauges to avoid label cardinality explosions.
///
/// # Arguments
///
/// This function has no arguments.
///
/// # Returns
///
/// Returns an empty [`BTreeMap`] suitable for gauge exports.
fn restart_pending_gauge_labels() -> BTreeMap<String, String> {
    BTreeMap::new()
}

/// Builds metric labels for stale completion counters without embedding child identifiers.
///
/// # Arguments
///
/// - `handled_as`: Low-cardinality handling bucket copied into the label map.
///
/// # Returns
///
/// Returns a label map safe for `supervisor_child_attempt_stale_report_total`.
fn stale_report_metric_labels(handled_as: StaleReportHandling) -> BTreeMap<String, String> {
    let mut labels = BTreeMap::new();
    labels.insert("handled_as".to_owned(), format!("{handled_as:?}"));
    labels
}

/// Extends event labels with the low-cardinality classification for generation-fence metrics.
///
/// # Arguments
///
/// - `event`: Supervisor event whose identifiers should seed the label map.
/// - `result`: Stable fence outcome fragment such as `entered`, `abort_requested`, or `released`.
///
/// # Returns
///
/// Returns merged labels ready for [`SupervisorMetricName::ChildRestartFenceTotal`].
fn child_restart_fence_labels(event: &SupervisorEvent, result: &str) -> BTreeMap<String, String> {
    let mut labels = labels_for_event(event);
    labels.insert("result".to_owned(), result.to_owned());
    labels
}

/// Builds labels for child control command counters.
///
/// # Arguments
///
/// - `command`: Stable command name.
/// - `result`: Stable result classification.
///
/// # Returns
///
/// Returns labels for a child control command counter.
fn child_control_command_labels(
    command: impl Into<String>,
    result: impl Into<String>,
) -> BTreeMap<String, String> {
    let mut labels = BTreeMap::new();
    labels.insert("command".to_owned(), command.into());
    labels.insert("result".to_owned(), result.into());
    labels
}

/// Builds labels for operation transition counters.
///
/// # Arguments
///
/// - `from`: Previous operation.
/// - `to`: New operation.
///
/// # Returns
///
/// Returns labels for an operation transition counter.
fn operation_transition_labels(
    from: impl Into<String>,
    to: impl Into<String>,
) -> BTreeMap<String, String> {
    let mut labels = BTreeMap::new();
    labels.insert("from".to_owned(), from.into());
    labels.insert("to".to_owned(), to.into());
    labels
}

/// Builds labels for restart limit gauges.
///
/// # Arguments
///
/// - `child_id`: Child whose restart limit was refreshed.
///
/// # Returns
///
/// Returns labels for the restart limit gauge.
fn restart_limit_labels(child_id: &crate::id::types::ChildId) -> BTreeMap<String, String> {
    let mut labels = BTreeMap::new();
    labels.insert("child_id".to_owned(), child_id.to_string());
    labels
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
    labels.insert(
        "correlation_id".to_owned(),
        event.correlation_id.value.to_string(),
    );
    labels
}

/// Builds metric samples for an aborted shutdown child.
///
/// # Arguments
///
/// - `event`: Lifecycle event used as the label source.
/// - `phase`: Shutdown phase label.
/// - `result`: Child shutdown result label.
/// - `reason`: Abort reason supplied by the event.
///
/// # Returns
///
/// Returns child outcome and abort counter samples.
fn shutdown_aborted_samples(
    event: &SupervisorEvent,
    phase: &str,
    result: &str,
    reason: &str,
) -> Vec<MetricSample> {
    vec![
        MetricSample::new(
            SupervisorMetricName::ShutdownChildOutcomesTotal,
            1.0,
            shutdown_child_outcome_labels_for_event(event, result, phase),
        ),
        MetricSample::new(
            SupervisorMetricName::ShutdownAbortTotal,
            1.0,
            shutdown_abort_labels_for_event(event, phase, reason),
        ),
    ]
}

/// Builds metric samples for a late shutdown child report.
///
/// # Arguments
///
/// - `event`: Lifecycle event used as the label source.
/// - `phase`: Shutdown phase label.
///
/// # Returns
///
/// Returns child outcome and late-report counter samples.
fn shutdown_late_report_samples(event: &SupervisorEvent, phase: &str) -> Vec<MetricSample> {
    vec![
        MetricSample::new(
            SupervisorMetricName::ShutdownChildOutcomesTotal,
            1.0,
            shutdown_child_outcome_labels_for_event(event, "late_report", phase),
        ),
        MetricSample::new(
            SupervisorMetricName::ShutdownLateReportsTotal,
            1.0,
            shutdown_late_report_labels_for_event(event, phase),
        ),
    ]
}

/// Builds metric samples for a terminal runtime control loop event.
///
/// # Arguments
///
/// - `event`: Lifecycle event used as the label source.
/// - `state`: Terminal runtime state.
/// - `phase`: Runtime phase label.
///
/// # Returns
///
/// Returns exit counter and alive gauge samples.
fn runtime_terminal_samples(
    event: &SupervisorEvent,
    state: &str,
    phase: &str,
) -> Vec<MetricSample> {
    vec![
        MetricSample::new(
            SupervisorMetricName::RuntimeControlLoopExitTotal,
            1.0,
            runtime_labels_for_event(event, state, phase),
        ),
        MetricSample::new(
            SupervisorMetricName::RuntimeControlPlaneAlive,
            0.0,
            runtime_labels_for_event(event, state, phase),
        ),
    ]
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

/// Builds low-cardinality labels for shutdown completion duration.
fn shutdown_completed_labels_for_event(
    event: &SupervisorEvent,
    phase: &str,
    result: &str,
) -> BTreeMap<String, String> {
    let mut labels = BTreeMap::new();
    labels.insert(
        "supervisor_path".to_owned(),
        event.r#where.supervisor_path.to_string(),
    );
    labels.insert("phase".to_owned(), phase.to_owned());
    labels.insert("result".to_owned(), result.to_owned());
    labels
}

/// Builds labels for child shutdown outcome counters.
fn shutdown_child_outcome_labels_for_event(
    event: &SupervisorEvent,
    status: &str,
    phase: &str,
) -> BTreeMap<String, String> {
    let mut labels = BTreeMap::new();
    labels.insert(
        "supervisor_path".to_owned(),
        event.r#where.supervisor_path.to_string(),
    );
    labels.insert("status".to_owned(), status.to_owned());
    labels.insert("phase".to_owned(), phase.to_owned());
    labels
}

/// Builds labels for shutdown abort counters.
fn shutdown_abort_labels_for_event(
    event: &SupervisorEvent,
    phase: &str,
    reason: &str,
) -> BTreeMap<String, String> {
    let mut labels = BTreeMap::new();
    labels.insert(
        "supervisor_path".to_owned(),
        event.r#where.supervisor_path.to_string(),
    );
    labels.insert("phase".to_owned(), phase.to_owned());
    labels.insert(
        "reason".to_owned(),
        shutdown_abort_reason(reason).to_owned(),
    );
    labels
}

/// Builds labels for late shutdown report counters.
fn shutdown_late_report_labels_for_event(
    event: &SupervisorEvent,
    phase: &str,
) -> BTreeMap<String, String> {
    let mut labels = BTreeMap::new();
    labels.insert(
        "supervisor_path".to_owned(),
        event.r#where.supervisor_path.to_string(),
    );
    labels.insert("phase".to_owned(), phase.to_owned());
    labels
}

/// Converts an abort reason into a bounded metric label value.
fn shutdown_abort_reason(reason: &str) -> &'static str {
    let reason = reason.to_ascii_lowercase();
    if reason.contains("timeout") {
        "timeout"
    } else if reason.contains("failed") {
        "abort_failed"
    } else if reason.contains("operator") {
        "operator"
    } else if reason.is_empty() {
        "unspecified"
    } else {
        "runtime"
    }
}
