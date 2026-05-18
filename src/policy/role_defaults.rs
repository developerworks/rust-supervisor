//! Work role defaults for supervised children.
//!
//! This module owns role classification, default policy bundles, effective
//! policy attribution, and semantic conflict diagnostics.

use crate::id::types::ChildId;
use crate::spec::child::{BackoffPolicy, RestartPolicy};
use crate::spec::supervisor::{EscalationPolicy, RestartLimit};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::time::Duration;

/// Work role classification for supervised children.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkRole {
    /// Long-running service that should stay online.
    Service,
    /// Background worker with bounded retry semantics.
    Worker,
    /// One-shot job that must not auto-restart on success.
    Job,
    /// Auxiliary sidecar process attached to a primary service.
    Sidecar,
    /// Nested supervisor tree treated as a single unit.
    Supervisor,
}

impl WorkRole {
    /// Returns a stable low-cardinality role label.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a snake_case static role label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Service => "service",
            Self::Worker => "worker",
            Self::Job => "job",
            Self::Sidecar => "sidecar",
            Self::Supervisor => "supervisor",
        }
    }
}

impl Display for WorkRole {
    /// Formats the role as a stable label.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Configuration for sidecar attachment to a primary service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SidecarConfig {
    /// Child ID of the primary service this sidecar attaches to.
    pub primary_child_id: ChildId,
    /// Whether lifecycle events are linked.
    #[serde(default)]
    pub linked_lifecycle: bool,
}

impl SidecarConfig {
    /// Creates a sidecar binding configuration.
    ///
    /// # Arguments
    ///
    /// - `primary_child_id`: Child ID of the primary service.
    /// - `linked_lifecycle`: Whether lifecycle operations are linked.
    ///
    /// # Returns
    ///
    /// Returns a [`SidecarConfig`] value.
    pub fn new(primary_child_id: ChildId, linked_lifecycle: bool) -> Self {
        Self {
            primary_child_id,
            linked_lifecycle,
        }
    }
}

/// Action taken when a child exits successfully.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OnSuccessAction {
    /// Restart the child to keep it online.
    Restart,
    /// Stop the child permanently.
    Stop,
    /// Take no automatic action.
    NoOp,
}

/// Action taken when a child exits with failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OnFailureAction {
    /// Restart with backoff policy applied.
    RestartWithBackoff,
    /// Restart indefinitely.
    RestartPermanent,
    /// Stop and escalate to parent or shutdown tree.
    StopAndEscalate,
}

/// Action taken when a child receives an explicit stop request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OnManualStopAction {
    /// Stop permanently until explicitly restarted.
    StopForever,
    /// Stop but allow a future explicit restart.
    StopUntilExplicitRestart,
}

/// Action taken when a child exceeds its execution timeout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OnTimeoutAction {
    /// Restart with backoff policy applied.
    RestartWithBackoff,
    /// Stop and escalate to parent or shutdown tree.
    StopAndEscalate,
}

/// Action taken when restart budget is exhausted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OnBudgetExhaustedAction {
    /// Stop and escalate to parent or shutdown tree.
    StopAndEscalate,
    /// Quarantine the child or scope without escalating.
    Quarantine,
}

/// Default policy bundle bound to a specific work role.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RoleDefaultPolicy {
    /// Action on successful exit.
    pub on_success_exit: OnSuccessAction,
    /// Action on failure exit.
    pub on_failure_exit: OnFailureAction,
    /// Action on explicit manual stop.
    pub on_manual_stop: OnManualStopAction,
    /// Action on execution timeout.
    pub on_timeout: OnTimeoutAction,
    /// Action when restart budget is exhausted.
    pub on_budget_exhausted: OnBudgetExhaustedAction,
    /// Default restart limit.
    pub default_restart_limit: Option<RestartLimit>,
    /// Default escalation policy.
    pub default_escalation_policy: Option<EscalationPolicy>,
    /// Default backoff policy.
    pub default_backoff_policy: Option<BackoffPolicy>,
    /// Exit codes considered successful.
    #[serde(default = "default_success_exit_codes")]
    pub success_exit_codes: Vec<i32>,
}

/// Role-specific differences used to build a default policy.
struct RoleDefaultPolicyDifferences {
    /// Action on successful exit.
    on_success_exit: OnSuccessAction,
    /// Action on execution timeout.
    on_timeout: OnTimeoutAction,
    /// Maximum restart count inside the default restart limit window.
    max_restarts: u32,
}

impl From<RoleDefaultPolicyDifferences> for RoleDefaultPolicy {
    /// Converts role-specific differences into a complete default policy.
    ///
    /// # Arguments
    ///
    /// - `differences`: Role-specific policy fields.
    ///
    /// # Returns
    ///
    /// Returns a complete [`RoleDefaultPolicy`] with shared defaults applied.
    fn from(differences: RoleDefaultPolicyDifferences) -> Self {
        Self {
            on_success_exit: differences.on_success_exit,
            on_failure_exit: OnFailureAction::RestartWithBackoff,
            on_manual_stop: OnManualStopAction::StopForever,
            on_timeout: differences.on_timeout,
            on_budget_exhausted: OnBudgetExhaustedAction::StopAndEscalate,
            default_restart_limit: Some(bounded_restart_limit(differences.max_restarts)),
            default_escalation_policy: Some(EscalationPolicy::EscalateToParent),
            default_backoff_policy: Some(default_backoff_policy()),
            success_exit_codes: default_success_exit_codes(),
        }
    }
}

impl RoleDefaultPolicy {
    /// Returns the default policy pack for a work role.
    ///
    /// # Arguments
    ///
    /// - `role`: Work role used to select defaults.
    ///
    /// # Returns
    ///
    /// Returns a role-specific [`RoleDefaultPolicy`].
    pub fn for_role(role: WorkRole) -> Self {
        match role {
            WorkRole::Service => service_default(),
            WorkRole::Worker => worker_default(),
            WorkRole::Job => job_default(),
            WorkRole::Sidecar => sidecar_default(),
            WorkRole::Supervisor => supervisor_default(),
        }
    }
}

/// Source used to build an effective policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PolicySource {
    /// Policy came from an explicit role default.
    RoleDefault,
    /// Policy contains user overrides.
    UserOverride,
    /// Policy used the conservative fallback role.
    FallbackDefault,
}

impl Display for PolicySource {
    /// Formats the policy source as a stable label.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::RoleDefault => "role_default",
            Self::UserOverride => "user_override",
            Self::FallbackDefault => "fallback_default",
        };
        formatter.write_str(label)
    }
}

/// Severity classification for failure escalation bifurcation.
///
/// Ordering: Critical > Standard > Optional (highest to lowest severity).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
pub enum SeverityClass {
    /// Optional: failure follows noise-reduction path (no alert upgrade).
    Optional,
    /// Standard: follows the default WorkRole behavior.
    Standard,
    /// Critical: failure must trigger escalation path.
    Critical,
}

/// Effective policy selected for one child.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct EffectivePolicy {
    /// Effective work role after fallback handling.
    pub work_role: WorkRole,
    /// Policy pack selected for the effective role.
    pub policy_pack: RoleDefaultPolicy,
    /// Source of the effective policy.
    pub source: PolicySource,
    /// Whether the worker fallback default was used.
    pub used_fallback: bool,
    /// Fields explicitly overridden by the user.
    pub overridden_fields: Vec<String>,
    /// Severity classification for escalation bifurcation.
    pub severity: SeverityClass,
    /// Group name for group isolation (None = not grouped).
    pub group_name: Option<String>,
}

impl EffectivePolicy {
    /// Merges role defaults with known user override markers.
    ///
    /// # Arguments
    ///
    /// - `role`: Optional declared work role.
    /// - `overridden_fields`: Fields explicitly set by the user.
    ///
    /// # Returns
    ///
    /// Returns an [`EffectivePolicy`] with fallback attribution.
    pub fn merge(role: Option<WorkRole>, overridden_fields: Vec<String>) -> Self {
        let used_fallback = role.is_none();
        let work_role = role.unwrap_or(WorkRole::Worker);
        let source = if used_fallback {
            PolicySource::FallbackDefault
        } else if overridden_fields.is_empty() {
            PolicySource::RoleDefault
        } else {
            PolicySource::UserOverride
        };
        let severity = Self::default_severity(work_role);
        Self {
            work_role,
            policy_pack: RoleDefaultPolicy::for_role(work_role),
            source,
            used_fallback,
            overridden_fields,
            severity,
            group_name: None,
        }
    }

    /// Returns the default [`SeverityClass`] for a given [`WorkRole`].
    fn default_severity(role: WorkRole) -> SeverityClass {
        match role {
            WorkRole::Service => SeverityClass::Critical,
            WorkRole::Supervisor => SeverityClass::Critical,
            WorkRole::Worker => SeverityClass::Standard,
            WorkRole::Job => SeverityClass::Optional,
            WorkRole::Sidecar => SeverityClass::Standard,
        }
    }

    /// Builds an effective policy for a child specification.
    ///
    /// # Arguments
    ///
    /// - `child`: Child specification to inspect.
    ///
    /// # Returns
    ///
    /// Returns the effective role policy for the child.
    pub fn for_child(child: &crate::spec::child::ChildSpec) -> Self {
        let mut overridden = Vec::new();
        if child.restart_policy != RestartPolicy::Transient {
            overridden.push("restart_policy".to_string());
        }
        let effective_policy = Self::merge(child.work_role, overridden);
        if child.work_role.is_none() {
            tracing::warn!(
                child_id = %child.id,
                work_role = %effective_policy.work_role,
                used_fallback_default = effective_policy.used_fallback,
                effective_policy_source = %effective_policy.source,
                "work role missing, falling back to worker default"
            );
        }
        effective_policy
    }
}

/// Describes one role semantic conflict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleSemanticConflict {
    /// Child that owns the conflict.
    pub child_id: ChildId,
    /// Declared work role.
    pub work_role: WorkRole,
    /// Conflicting field name.
    pub conflicting_field: String,
    /// User-provided value.
    pub user_value: String,
    /// Role default expectation.
    pub expected_semantic: String,
    /// Human-readable reason.
    pub reason: String,
}

/// Returns semantic conflicts for one child.
///
/// # Arguments
///
/// - `child`: Child specification to inspect.
///
/// # Returns
///
/// Returns a list of role semantic conflicts.
pub fn semantic_conflicts_for_child(
    child: &crate::spec::child::ChildSpec,
) -> Vec<RoleSemanticConflict> {
    let mut conflicts = Vec::new();
    if child.work_role == Some(WorkRole::Job) && child.restart_policy == RestartPolicy::Permanent {
        conflicts.push(RoleSemanticConflict {
            child_id: child.id.clone(),
            work_role: WorkRole::Job,
            conflicting_field: "restart_policy".to_string(),
            user_value: "permanent".to_string(),
            expected_semantic: "job success should stop".to_string(),
            reason: "Job role must not silently use permanent restart semantics".to_string(),
        });
    }
    conflicts
}

/// Returns default success exit codes.
///
/// # Arguments
///
/// This function has no arguments.
///
/// # Returns
///
/// Returns a vector containing exit code zero.
fn default_success_exit_codes() -> Vec<i32> {
    vec![0]
}

/// Returns a bounded restart limit used by role defaults.
fn bounded_restart_limit(max_restarts: u32) -> RestartLimit {
    RestartLimit::new(max_restarts, Duration::from_secs(60))
}

/// Returns a default backoff policy used by role defaults.
fn default_backoff_policy() -> BackoffPolicy {
    BackoffPolicy::new(Duration::from_millis(50), Duration::from_secs(5), 0.2)
}

/// Returns service role defaults.
fn service_default() -> RoleDefaultPolicy {
    RoleDefaultPolicyDifferences {
        on_success_exit: OnSuccessAction::Restart,
        on_timeout: OnTimeoutAction::RestartWithBackoff,
        max_restarts: 10,
    }
    .into()
}

/// Returns worker role defaults.
fn worker_default() -> RoleDefaultPolicy {
    RoleDefaultPolicyDifferences {
        on_success_exit: OnSuccessAction::Stop,
        on_timeout: OnTimeoutAction::RestartWithBackoff,
        max_restarts: 3,
    }
    .into()
}

/// Returns job role defaults.
fn job_default() -> RoleDefaultPolicy {
    RoleDefaultPolicyDifferences {
        on_success_exit: OnSuccessAction::Stop,
        on_timeout: OnTimeoutAction::StopAndEscalate,
        max_restarts: 1,
    }
    .into()
}

/// Returns sidecar role defaults.
fn sidecar_default() -> RoleDefaultPolicy {
    RoleDefaultPolicyDifferences {
        on_success_exit: OnSuccessAction::Restart,
        on_timeout: OnTimeoutAction::RestartWithBackoff,
        max_restarts: 5,
    }
    .into()
}

/// Returns nested supervisor role defaults.
fn supervisor_default() -> RoleDefaultPolicy {
    RoleDefaultPolicyDifferences {
        on_success_exit: OnSuccessAction::Restart,
        on_timeout: OnTimeoutAction::RestartWithBackoff,
        max_restarts: 3,
    }
    .into()
}
