//! Six-stage supervision pipeline orchestration.
//!
//! This module implements the unified failure processing pipeline:
//! 1. **classify exit**: Classify the exit reason and category
//! 2. **record failure window**: Record failure into sliding window
//! 3. **evaluate budget**: Evaluate restart budget and limits from restart_execution_plan
//! 4. **decide action**: Decide protective action based on merged verdicts
//! 5. **emit typed event**: Emit structured supervision event with all diagnostic fields
//! 6. **execute action**: Execute the decided action (restart, queue, deny, etc.)

use crate::error::types::TaskFailure;
use crate::event::payload::{
    ColdStartReason, HotLoopReason, MeltdownScope, ProtectionAction, SupervisorEvent,
    ThrottleGateOwner, What, Where,
};
use crate::id::types::{ChildId, SupervisorPath};
use crate::observe::pipeline::{ObservabilityPipeline, PipelineStage, PipelineStageDiagnostic};
use crate::policy::backoff::{ColdStartBudget, HotLoopDetector};
use crate::policy::decision::{PolicyFailureKind, TaskExit};
use crate::policy::failure_window::FailureWindow;
use crate::policy::meltdown::{
    LocalVerdict, MeltdownOutcome, MeltdownTracker, merge_meltdown_verdicts,
};
use crate::policy::role_defaults::{
    EffectivePolicy, OnBudgetExhaustedAction, OnFailureAction, OnSuccessAction, OnTimeoutAction,
};
use crate::spec::supervisor::{EscalationPolicy, RestartLimit, SupervisorSpec};
use crate::tree::builder::SupervisorTree;
use crate::tree::order::restart_execution_plan;
use std::time::{Instant, SystemTime};

/// Exit classification result from stage 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExitClassification {
    /// Successful completion.
    Success,
    /// Non-zero exit code.
    NonZeroExit { exit_code: i32 },
    /// Process crash or panic.
    Crash { reason: String },
    /// Timeout exceeded.
    Timeout,
    /// External cancellation requested.
    ExternalCancel,
    /// Manual stop requested by operator.
    ManualStop,
}

impl ExitClassification {
    /// Returns a string representation for diagnostics.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::NonZeroExit { .. } => "nonzero_exit",
            Self::Crash { .. } => "panic",
            Self::Timeout => "timeout",
            Self::ExternalCancel => "external_cancel",
            Self::ManualStop => "manual_stop",
        }
    }

    /// Checks if this exit should trigger automatic restart.
    pub fn should_restart(&self) -> bool {
        match self {
            Self::Success => false,
            Self::NonZeroExit { .. } => true,
            Self::Crash { .. } => true,
            Self::Timeout => true,
            Self::ExternalCancel => false,
            Self::ManualStop => false,
        }
    }
}

/// Budget evaluation result from stage 3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetEvaluation {
    /// Remaining restart count before limit is reached.
    pub remaining_restarts: Option<u32>,
    /// Whether the restart limit has been exhausted.
    pub limit_exhausted: bool,
    /// Escalation policy if defined.
    pub escalation_policy: Option<String>,
    /// Effective meltdown outcome after merging local verdicts.
    pub meltdown_outcome: MeltdownOutcome,
}

/// Final decision from stage 4.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionDecision {
    /// The chosen protection action.
    pub action: ProtectionAction,
    /// Optional delay before execution.
    pub delay_ms: Option<u64>,
    /// Reason for the decision.
    pub reason: String,
}

/// Complete pipeline context carrying state through all six stages.
#[derive(Debug, Clone)]
pub struct PipelineContext {
    /// Child identifier being supervised.
    pub child_id: ChildId,
    /// Supervisor path owning the scope.
    pub supervisor_path: SupervisorPath,
    /// Group identifier if the child belongs to a group.
    pub group_id: Option<String>,
    /// Exit classification from stage 1.
    pub exit_classification: Option<ExitClassification>,
    /// Failure window state from stage 2.
    pub failure_window_state: Option<String>,
    /// Budget evaluation from stage 3.
    pub budget_evaluation: Option<BudgetEvaluation>,
    /// Action decision from stage 4.
    pub action_decision: Option<ActionDecision>,
    /// Cold start reason determined during evaluation.
    pub cold_start_reason: ColdStartReason,
    /// Hot loop reason determined during detection.
    pub hot_loop_reason: HotLoopReason,
    /// Throttle gate owner that limited concurrent restarts.
    pub throttle_gate_owner: ThrottleGateOwner,
    /// Effective role policy applied to this pipeline run.
    pub effective_policy: Option<EffectivePolicy>,
    /// Meltdown scopes that triggered in this pipeline round.
    pub scopes_triggered: Vec<MeltdownScope>,
    /// Dominant meltdown scope selected for attribution.
    pub lead_scope: Option<MeltdownScope>,
    /// Stage diagnostics emitted by the six-stage pipeline.
    pub stage_diagnostics: Vec<PipelineStageDiagnostic>,
    /// Result summary produced by the execute action stage.
    pub execution_result: Option<String>,
    /// Event sequence number.
    pub sequence: u64,
    /// Correlation identifier.
    pub correlation_id: String,
}

impl PipelineContext {
    /// Creates a new pipeline context.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Child identifier.
    /// - `supervisor_path`: Supervisor path.
    /// - `sequence`: Event sequence number.
    /// - `correlation_id`: Correlation identifier.
    ///
    /// # Returns
    ///
    /// Returns a new [`PipelineContext`].
    pub fn new(
        child_id: ChildId,
        supervisor_path: SupervisorPath,
        sequence: u64,
        correlation_id: impl Into<String>,
    ) -> Self {
        Self {
            child_id,
            supervisor_path,
            group_id: None,
            exit_classification: None,
            failure_window_state: None,
            budget_evaluation: None,
            action_decision: None,
            cold_start_reason: ColdStartReason::NotApplicable,
            hot_loop_reason: HotLoopReason::NotApplicable,
            throttle_gate_owner: ThrottleGateOwner::None,
            effective_policy: None,
            scopes_triggered: Vec::new(),
            lead_scope: None,
            stage_diagnostics: Vec::new(),
            execution_result: None,
            sequence,
            correlation_id: correlation_id.into(),
        }
    }
}

/// Six-stage supervision pipeline orchestrator.
#[derive(Debug)]
pub struct SupervisionPipeline {
    /// Observability pipeline for event emission.
    pub observability: ObservabilityPipeline,
    /// Meltdown tracker for failure counting.
    pub meltdown_tracker: MeltdownTracker,
    /// Failure window for sliding accumulation.
    pub failure_window: FailureWindow,
    /// Cold start restart budget for initial startup protection.
    pub cold_start_budget: ColdStartBudget,
    /// Hot loop detector for rapid crash-restart cycles.
    pub hot_loop_detector: HotLoopDetector,
}

impl SupervisionPipeline {
    /// Creates a new supervision pipeline.
    ///
    /// # Arguments
    ///
    /// - `journal_capacity`: Event journal capacity.
    /// - `subscriber_capacity`: Subscriber queue capacity.
    /// - `meltdown_tracker`: Configured meltdown tracker.
    /// - `failure_window`: Configured failure window.
    ///
    /// # Returns
    ///
    /// Returns a new [`SupervisionPipeline`].
    pub fn new(
        journal_capacity: usize,
        subscriber_capacity: usize,
        meltdown_tracker: MeltdownTracker,
        failure_window: FailureWindow,
    ) -> Self {
        let started_at_secs = current_unix_secs();
        Self {
            observability: ObservabilityPipeline::new(journal_capacity, subscriber_capacity),
            meltdown_tracker,
            failure_window,
            cold_start_budget: ColdStartBudget::new(60, 5, started_at_secs),
            hot_loop_detector: HotLoopDetector::new(10, 3),
        }
    }

    /// Executes the complete six-stage pipeline for a child exit.
    ///
    /// # Arguments
    ///
    /// - `ctx`: Pipeline context with child information.
    /// - `exit`: The task exit to process.
    /// - `spec`: Supervisor specification for restart_execution_plan.
    /// - `tree`: Supervisor tree for scope calculation.
    ///
    /// # Returns
    ///
    /// Returns the updated pipeline context with all stage results.
    pub fn execute_pipeline(
        &mut self,
        mut ctx: PipelineContext,
        exit: TaskExit,
        spec: &SupervisorSpec,
        tree: &SupervisorTree,
    ) -> PipelineContext {
        let now = Instant::now();
        let now_unix_nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();

        // Stage 1: Classify Exit
        ctx = self.stage_classify_exit(ctx, &exit, now_unix_nanos);

        // Stage 2: Record Failure Window
        ctx = self.stage_record_failure_window(ctx, now, now_unix_nanos);

        // Stage 3: Evaluate Budget
        ctx = self.stage_evaluate_budget(ctx, spec, tree, now, now_unix_nanos);

        // Stage 4: Decide Action
        ctx = self.stage_decide_action(ctx, now_unix_nanos);

        // Stage 5: Emit Typed Event
        ctx = self.stage_emit_typed_event(ctx, &exit, now_unix_nanos);

        // Stage 6: Execute Action
        ctx = self.stage_execute_action(ctx, now_unix_nanos);

        ctx
    }

    /// Stage 1: Classify the exit reason and category.
    ///
    /// # Arguments
    ///
    /// - `ctx`: Current pipeline context.
    /// - `exit`: Task exit to classify.
    ///
    /// # Returns
    ///
    /// Returns the updated context with exit classification.
    fn stage_classify_exit(
        &self,
        mut ctx: PipelineContext,
        exit: &TaskExit,
        completed_at_unix_nanos: u128,
    ) -> PipelineContext {
        let classification = ctx
            .exit_classification
            .clone()
            .unwrap_or_else(|| match exit {
                TaskExit::Succeeded => ExitClassification::Success,
                TaskExit::Failed { kind, .. } => match kind {
                    PolicyFailureKind::Cancelled => ExitClassification::ExternalCancel,
                    PolicyFailureKind::Panic => ExitClassification::Crash {
                        reason: "panic".to_string(),
                    },
                    PolicyFailureKind::Timeout => ExitClassification::Timeout,
                    _ => ExitClassification::NonZeroExit { exit_code: -1 },
                },
            });

        ctx.exit_classification = Some(classification);
        append_stage_diagnostic(
            &mut ctx,
            PipelineStage::ClassifyExit,
            completed_at_unix_nanos,
        );
        ctx
    }

    /// Stage 2: Record failure into sliding window.
    ///
    /// # Arguments
    ///
    /// - `ctx`: Current pipeline context.
    /// - `now`: Current monotonic time.
    ///
    /// # Returns
    ///
    /// Returns the updated context with failure window state.
    fn stage_record_failure_window(
        &mut self,
        mut ctx: PipelineContext,
        now: Instant,
        completed_at_unix_nanos: u128,
    ) -> PipelineContext {
        // Only record failures, not successes
        if let Some(ref classification) = ctx.exit_classification
            && classification.should_restart()
        {
            let state = self.failure_window.record_failure(now);
            ctx.failure_window_state = Some(format!(
                "count={}, threshold_reached={}",
                state.current_count, state.threshold_reached
            ));
        }
        append_stage_diagnostic(
            &mut ctx,
            PipelineStage::RecordFailureWindow,
            completed_at_unix_nanos,
        );
        ctx
    }

    /// Stage 3: Evaluate restart budget and limits.
    ///
    /// # Arguments
    ///
    /// - `ctx`: Current pipeline context.
    /// - `spec`: Supervisor specification.
    /// - `tree`: Supervisor tree.
    ///
    /// # Returns
    ///
    /// Returns the updated context with budget evaluation.
    fn stage_evaluate_budget(
        &mut self,
        mut ctx: PipelineContext,
        spec: &SupervisorSpec,
        tree: &SupervisorTree,
        now: Instant,
        completed_at_unix_nanos: u128,
    ) -> PipelineContext {
        // Get restart_execution_plan for this child
        let plan = restart_execution_plan(tree, spec, &ctx.child_id);

        let restart_failure_count = self.failure_window.failure_count() as u32;
        let restart_limit = effective_restart_limit(&ctx, plan.restart_limit);
        let escalation_policy = effective_escalation_policy(&ctx, plan.escalation_policy);
        let remaining =
            restart_limit.map(|limit| limit.max_restarts.saturating_sub(restart_failure_count));

        let limit_exhausted =
            restart_limit.is_some_and(|limit| restart_failure_count > limit.max_restarts);
        let group_id = plan.group.clone();
        let should_restart = ctx
            .exit_classification
            .as_ref()
            .is_some_and(ExitClassification::should_restart);
        let now_secs = nanos_to_secs(completed_at_unix_nanos);
        if should_restart {
            let exhausted = self.cold_start_budget.record_restart(now_secs);
            ctx.cold_start_reason = if exhausted {
                ColdStartReason::BudgetExhausted
            } else if self.cold_start_budget.is_window_active(now_secs) {
                ColdStartReason::InitialStartup
            } else {
                ColdStartReason::NotApplicable
            };

            if self.hot_loop_detector.record_crash(now_secs) {
                ctx.hot_loop_reason = HotLoopReason::RapidCrashDetected;
            }
        }

        let meltdown_outcome = if should_restart {
            self.meltdown_tracker.record_child_restart_with_group(
                ctx.child_id.clone(),
                group_id.clone(),
                now,
            );
            let merged = merge_meltdown_verdicts(
                child_local_verdict(&self.meltdown_tracker, &ctx.child_id),
                group_local_verdict(&self.meltdown_tracker, group_id.as_deref()),
                supervisor_local_verdict(&self.meltdown_tracker),
            );
            ctx.scopes_triggered = merged.scopes_triggered;
            ctx.lead_scope = merged.lead_scope;
            merged.effective_outcome
        } else {
            MeltdownOutcome::Continue
        };

        ctx.budget_evaluation = Some(BudgetEvaluation {
            remaining_restarts: remaining,
            limit_exhausted,
            escalation_policy: escalation_policy.map(|policy| format!("{policy:?}")),
            meltdown_outcome,
        });

        // Set group_id from plan if available
        ctx.group_id = group_id;

        append_stage_diagnostic(
            &mut ctx,
            PipelineStage::EvaluateBudget,
            completed_at_unix_nanos,
        );
        ctx
    }

    /// Stage 4: Decide protective action based on merged verdicts.
    ///
    /// # Arguments
    ///
    /// - `ctx`: Current pipeline context.
    ///
    /// # Returns
    ///
    /// Returns the updated context with action decision.
    fn stage_decide_action(
        &self,
        mut ctx: PipelineContext,
        completed_at_unix_nanos: u128,
    ) -> PipelineContext {
        let classification = ctx.exit_classification.as_ref();
        let budget = ctx.budget_evaluation.as_ref();

        let (mut action, mut reason) = match classification {
            Some(ExitClassification::ExternalCancel) | Some(ExitClassification::ManualStop) => (
                ProtectionAction::SupervisedStop,
                "external_cancel_or_manual_stop".to_string(),
            ),
            Some(classification) => {
                role_or_budget_action(classification, ctx.effective_policy.as_ref(), budget)
            }
            None => budget_action(ctx.effective_policy.as_ref(), budget),
        };

        if let Some(budget_eval) = budget {
            let meltdown_action = protection_action_for_meltdown(budget_eval.meltdown_outcome);
            if meltdown_action > action {
                action = meltdown_action;
                reason = meltdown_reason(action).to_string();
            }
        }
        if ctx.cold_start_reason == ColdStartReason::BudgetExhausted
            && ProtectionAction::RestartDenied > action
        {
            action = ProtectionAction::RestartDenied;
            reason = "cold_start_budget_exhausted".to_string();
        }
        if ctx.hot_loop_reason != HotLoopReason::NotApplicable
            && ProtectionAction::SupervisionPaused > action
        {
            action = ProtectionAction::SupervisionPaused;
            reason = "hot_loop_detected".to_string();
        }

        ctx.action_decision = Some(ActionDecision {
            action,
            delay_ms: None,
            reason,
        });

        append_stage_diagnostic(
            &mut ctx,
            PipelineStage::DecideAction,
            completed_at_unix_nanos,
        );
        ctx
    }

    /// Stage 5: Emit typed supervision event with all diagnostic fields.
    ///
    /// # Arguments
    ///
    /// - `ctx`: Current pipeline context.
    /// - `exit`: Original task exit.
    /// - `now_unix_nanos`: Current timestamp.
    ///
    /// # Returns
    ///
    /// Returns the updated context.
    fn stage_emit_typed_event(
        &mut self,
        ctx: PipelineContext,
        exit: &TaskExit,
        now_unix_nanos: u128,
    ) -> PipelineContext {
        // Build the What payload based on exit
        let what = match exit {
            TaskExit::Succeeded => What::ChildRunning { transition: None },
            TaskExit::Failed { .. } => What::ChildFailed {
                failure: TaskFailure::new(
                    crate::error::types::TaskFailureKind::Error,
                    "pipeline_exit",
                    "processed through six-stage pipeline",
                ),
            },
        };

        // Create event with all diagnostic fields populated
        let location = Where::new(ctx.supervisor_path.clone())
            .with_child(ctx.child_id.clone(), "pipeline-child");

        let mut event = SupervisorEvent::new(
            crate::event::time::When::new(crate::event::time::EventTime::deterministic(
                now_unix_nanos,
                now_unix_nanos,
                0,
                crate::id::types::Generation::initial(),
                crate::id::types::ChildStartCount::first(),
            )),
            location,
            what,
            crate::event::time::EventSequence::new(ctx.sequence),
            crate::event::time::CorrelationId::from_uuid(uuid::Uuid::nil()),
            1,
        );

        // Populate new fields from pipeline processing
        event.effective_protective_action = ctx.action_decision.as_ref().map(|d| d.action);
        event.cold_start_reason = ctx.cold_start_reason.clone();
        event.hot_loop_reason = ctx.hot_loop_reason.clone();
        event.throttle_gate_owner = ctx.throttle_gate_owner.clone();
        event.scopes_triggered = ctx.scopes_triggered.clone();
        event.lead_scope = ctx.lead_scope;
        if let Some(effective_policy) = ctx.effective_policy.as_ref() {
            event.work_role = Some(effective_policy.work_role);
            event.used_fallback_default = effective_policy.used_fallback;
            event.effective_policy_source = Some(effective_policy.source);
        }

        // Emit through observability pipeline
        let _lagged = self.observability.emit(event);

        let mut ctx = ctx;
        append_stage_diagnostic(&mut ctx, PipelineStage::EmitTypedEvent, now_unix_nanos);
        ctx
    }

    /// Stage 6: Execute the decided action.
    ///
    /// # Arguments
    ///
    /// - `ctx`: Current pipeline context.
    ///
    /// # Returns
    ///
    /// Returns the updated context with execution result.
    fn stage_execute_action(
        &self,
        mut ctx: PipelineContext,
        completed_at_unix_nanos: u128,
    ) -> PipelineContext {
        ctx.execution_result = if let Some(ref decision) = ctx.action_decision {
            Some(match decision.action {
                ProtectionAction::RestartAllowed => "restart_allowed_for_runtime".to_string(),
                ProtectionAction::RestartQueued => "restart_queued".to_string(),
                ProtectionAction::RestartDenied => "restart_denied".to_string(),
                ProtectionAction::SupervisionPaused => "supervision_paused".to_string(),
                ProtectionAction::Escalated => "escalated".to_string(),
                ProtectionAction::SupervisedStop => "supervised_stop".to_string(),
            })
        } else {
            Some("no_decision".to_string())
        };

        append_stage_diagnostic(
            &mut ctx,
            PipelineStage::ExecuteAction,
            completed_at_unix_nanos,
        );
        ctx
    }
}

/// Selects the restart limit for the current pipeline run.
///
/// # Arguments
///
/// - `ctx`: Pipeline context carrying the effective role policy.
/// - `plan_limit`: Restart limit selected by the restart execution plan.
///
/// # Returns
///
/// Returns the explicit plan limit, or the role default limit when the plan does not define one.
fn effective_restart_limit(
    ctx: &PipelineContext,
    plan_limit: Option<RestartLimit>,
) -> Option<RestartLimit> {
    plan_limit.or_else(|| {
        ctx.effective_policy
            .as_ref()
            .and_then(|policy| policy.policy_pack.default_restart_limit)
    })
}

/// Selects the escalation policy for the current pipeline run.
///
/// # Arguments
///
/// - `ctx`: Pipeline context carrying the effective role policy.
/// - `plan_policy`: Escalation policy selected by the restart execution plan.
///
/// # Returns
///
/// Returns the explicit plan policy, or the role default policy when the plan does not define one.
fn effective_escalation_policy(
    ctx: &PipelineContext,
    plan_policy: Option<EscalationPolicy>,
) -> Option<EscalationPolicy> {
    plan_policy.or_else(|| {
        ctx.effective_policy
            .as_ref()
            .and_then(|policy| policy.policy_pack.default_escalation_policy)
    })
}

/// Selects either role-specific action or budget-only action.
///
/// # Arguments
///
/// - `classification`: Exit classification produced by stage 1.
/// - `effective_policy`: Optional role policy for the child.
/// - `budget`: Optional budget evaluation produced by stage 3.
///
/// # Returns
///
/// Returns the protection action and diagnostic reason.
fn role_or_budget_action(
    classification: &ExitClassification,
    effective_policy: Option<&EffectivePolicy>,
    budget: Option<&BudgetEvaluation>,
) -> (ProtectionAction, String) {
    let Some(effective_policy) = effective_policy else {
        return budget_action(None, budget);
    };
    match classification {
        ExitClassification::Success => match effective_policy.policy_pack.on_success_exit {
            OnSuccessAction::Restart => (
                ProtectionAction::RestartAllowed,
                "role_success_restart".to_string(),
            ),
            OnSuccessAction::Stop | OnSuccessAction::NoOp => (
                ProtectionAction::SupervisedStop,
                "role_success_stop".to_string(),
            ),
        },
        ExitClassification::Timeout => match effective_policy.policy_pack.on_timeout {
            OnTimeoutAction::RestartWithBackoff => budget_action(Some(effective_policy), budget),
            OnTimeoutAction::StopAndEscalate => (
                ProtectionAction::Escalated,
                "role_timeout_escalate".to_string(),
            ),
        },
        ExitClassification::NonZeroExit { .. } | ExitClassification::Crash { .. } => {
            match effective_policy.policy_pack.on_failure_exit {
                OnFailureAction::RestartWithBackoff | OnFailureAction::RestartPermanent => {
                    budget_action(Some(effective_policy), budget)
                }
                OnFailureAction::StopAndEscalate => (
                    ProtectionAction::Escalated,
                    "role_failure_escalate".to_string(),
                ),
            }
        }
        ExitClassification::ExternalCancel | ExitClassification::ManualStop => (
            ProtectionAction::SupervisedStop,
            "external_cancel_or_manual_stop".to_string(),
        ),
    }
}

/// Selects the budget-only protection action.
///
/// # Arguments
///
/// - `effective_policy`: Optional role policy used for exhausted budget semantics.
/// - `budget`: Optional budget evaluation produced by stage 3.
///
/// # Returns
///
/// Returns the protection action and diagnostic reason.
fn budget_action(
    effective_policy: Option<&EffectivePolicy>,
    budget: Option<&BudgetEvaluation>,
) -> (ProtectionAction, String) {
    let Some(budget_eval) = budget else {
        return (
            ProtectionAction::RestartAllowed,
            "within_restart_budget".to_string(),
        );
    };
    if !budget_eval.limit_exhausted {
        return (
            ProtectionAction::RestartAllowed,
            "within_restart_budget".to_string(),
        );
    }
    match effective_policy
        .map(|policy| policy.policy_pack.on_budget_exhausted)
        .unwrap_or(OnBudgetExhaustedAction::Quarantine)
    {
        OnBudgetExhaustedAction::StopAndEscalate => (
            ProtectionAction::Escalated,
            "restart_limit_exhausted".to_string(),
        ),
        OnBudgetExhaustedAction::Quarantine => (
            ProtectionAction::RestartDenied,
            "restart_limit_exhausted".to_string(),
        ),
    }
}

/// Returns a diagnostic reason for a meltdown action override.
///
/// # Arguments
///
/// - `action`: Protection action selected from a meltdown verdict.
///
/// # Returns
///
/// Returns a stable reason label.
fn meltdown_reason(action: ProtectionAction) -> &'static str {
    match action {
        ProtectionAction::RestartDenied => "meltdown_child_fuse",
        ProtectionAction::SupervisionPaused => "meltdown_group_fuse",
        ProtectionAction::Escalated => "meltdown_supervisor_fuse",
        ProtectionAction::RestartAllowed => "within_restart_budget",
        ProtectionAction::RestartQueued => "restart_queued_by_throttle",
        ProtectionAction::SupervisedStop => "external_cancel_or_manual_stop",
    }
}

/// Appends a diagnostic record for one completed pipeline stage.
///
/// # Arguments
///
/// - `ctx`: Pipeline context that receives the diagnostic record.
/// - `stage`: Stage that has completed.
/// - `completed_at_unix_nanos`: Completion timestamp in Unix epoch nanoseconds.
///
/// # Returns
///
/// This function returns nothing.
fn append_stage_diagnostic(
    ctx: &mut PipelineContext,
    stage: PipelineStage,
    completed_at_unix_nanos: u128,
) {
    let mut diagnostic = PipelineStageDiagnostic::new(
        ctx.sequence,
        ctx.correlation_id.clone(),
        stage,
        completed_at_unix_nanos,
    )
    .with_child_id(ctx.child_id.value.clone())
    .with_supervisor_path(ctx.supervisor_path.to_string());

    diagnostic.group_id = ctx.group_id.clone();
    diagnostic.exit_classification = ctx
        .exit_classification
        .as_ref()
        .map(|classification| classification.as_str().to_string());
    diagnostic.failure_window_state = ctx.failure_window_state.clone();
    diagnostic.budget_evaluation = ctx.budget_evaluation.as_ref().map(|budget| {
        format!(
            "remaining_restarts={:?}, limit_exhausted={}, escalation_policy={:?}, meltdown_outcome={:?}",
            budget.remaining_restarts,
            budget.limit_exhausted,
            budget.escalation_policy,
            budget.meltdown_outcome
        )
    });
    diagnostic.decided_action = ctx
        .action_decision
        .as_ref()
        .map(|decision| decision.action.to_string());
    diagnostic.event_emitted = stage == PipelineStage::EmitTypedEvent;
    diagnostic.execution_result = ctx.execution_result.clone();

    ctx.stage_diagnostics.push(diagnostic);
}

/// Returns the current Unix epoch timestamp in seconds.
///
/// # Arguments
///
/// This function has no arguments.
///
/// # Returns
///
/// Returns zero if system time is before Unix epoch.
fn current_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

/// Converts Unix nanoseconds to seconds.
///
/// # Arguments
///
/// - `nanos`: Unix epoch nanoseconds.
///
/// # Returns
///
/// Returns the whole-second timestamp capped at `u64::MAX`.
fn nanos_to_secs(nanos: u128) -> u64 {
    (nanos / 1_000_000_000).min(u128::from(u64::MAX)) as u64
}

/// Builds a child-scope local meltdown verdict from current tracker state.
///
/// # Arguments
///
/// - `tracker`: Meltdown tracker containing current counters.
/// - `child_id`: Child scope to evaluate.
///
/// # Returns
///
/// Returns the local child verdict.
fn child_local_verdict(tracker: &MeltdownTracker, child_id: &ChildId) -> LocalVerdict {
    let triggered =
        tracker.child_failure_count(child_id) >= tracker.policy.child_max_restarts as usize;
    LocalVerdict {
        triggered,
        outcome: if triggered {
            MeltdownOutcome::ChildFuse
        } else {
            MeltdownOutcome::Continue
        },
    }
}

/// Builds a group-scope local meltdown verdict from current tracker state.
///
/// # Arguments
///
/// - `tracker`: Meltdown tracker containing current counters.
/// - `group_id`: Optional group scope to evaluate.
///
/// # Returns
///
/// Returns the local group verdict.
fn group_local_verdict(tracker: &MeltdownTracker, group_id: Option<&str>) -> LocalVerdict {
    let triggered = group_id.is_some_and(|group| {
        tracker.group_failure_count(group) >= tracker.policy.group_max_failures as usize
    });
    LocalVerdict {
        triggered,
        outcome: if triggered {
            MeltdownOutcome::GroupFuse
        } else {
            MeltdownOutcome::Continue
        },
    }
}

/// Builds a supervisor-scope local meltdown verdict from current tracker state.
///
/// # Arguments
///
/// - `tracker`: Meltdown tracker containing current counters.
///
/// # Returns
///
/// Returns the local supervisor verdict.
fn supervisor_local_verdict(tracker: &MeltdownTracker) -> LocalVerdict {
    let triggered = tracker.get_supervisor_outcome() == MeltdownOutcome::SupervisorFuse;
    LocalVerdict {
        triggered,
        outcome: if triggered {
            MeltdownOutcome::SupervisorFuse
        } else {
            MeltdownOutcome::Continue
        },
    }
}

/// Maps meltdown outcomes onto the protection action ladder.
///
/// # Arguments
///
/// - `outcome`: Effective meltdown outcome.
///
/// # Returns
///
/// Returns the corresponding protection action.
fn protection_action_for_meltdown(outcome: MeltdownOutcome) -> ProtectionAction {
    match outcome {
        MeltdownOutcome::Continue => ProtectionAction::RestartAllowed,
        MeltdownOutcome::ChildFuse => ProtectionAction::RestartDenied,
        MeltdownOutcome::GroupFuse => ProtectionAction::SupervisionPaused,
        MeltdownOutcome::SupervisorFuse => ProtectionAction::Escalated,
    }
}

#[cfg(test)]
mod tests {
    use crate::event::payload::ProtectionAction;
    use crate::id::types::{ChildId, SupervisorPath};
    use crate::policy::decision::{PolicyFailureKind, TaskExit};
    use crate::policy::failure_window::{FailureWindow, FailureWindowConfig};
    use crate::policy::meltdown::{MeltdownOutcome, MeltdownPolicy, MeltdownTracker};
    use crate::runtime::pipeline::{
        BudgetEvaluation, ExitClassification, PipelineContext, SupervisionPipeline,
    };
    use std::time::Duration;

    /// Creates a test supervision pipeline with default meltdown policy configuration.
    fn test_pipeline() -> SupervisionPipeline {
        let meltdown_policy = MeltdownPolicy::new(
            3,
            Duration::from_secs(10),
            5,
            Duration::from_secs(30),
            10,
            Duration::from_secs(60),
            Duration::from_secs(120),
        );
        let meltdown_tracker = MeltdownTracker::new(meltdown_policy);

        let failure_config = FailureWindowConfig::time_sliding(60, 5);
        let failure_window = FailureWindow::new(failure_config);

        SupervisionPipeline::new(100, 10, meltdown_tracker, failure_window)
    }

    /// Tests that successful task exits are classified correctly by the pipeline.
    #[test]
    fn test_exit_classification_success() {
        let pipeline = test_pipeline();
        let child_id = ChildId::new("test".to_string());
        let path = SupervisorPath::root();
        let ctx = PipelineContext::new(child_id, path, 1, "test-correlation");

        let exit = TaskExit::Succeeded;
        let ctx = pipeline.stage_classify_exit(ctx, &exit, 1);

        assert_eq!(ctx.exit_classification, Some(ExitClassification::Success));
        assert!(!ctx.exit_classification.unwrap().should_restart());
    }

    /// Tests that failed task exits are classified correctly with restart decision.
    #[test]
    fn test_exit_classification_failure() {
        let pipeline = test_pipeline();
        let child_id = ChildId::new("test".to_string());
        let path = SupervisorPath::root();
        let ctx = PipelineContext::new(child_id, path, 1, "test-correlation");

        let exit = TaskExit::Failed {
            kind: PolicyFailureKind::Recoverable,
        };
        let ctx = pipeline.stage_classify_exit(ctx, &exit, 1);

        assert!(matches!(
            ctx.exit_classification,
            Some(ExitClassification::NonZeroExit { .. })
        ));
        assert!(ctx.exit_classification.unwrap().should_restart());
    }

    /// Tests that cancel exits have priority over budget evaluation in action decision.
    #[test]
    fn test_cancel_has_priority() {
        let pipeline = test_pipeline();
        let child_id = ChildId::new("test".to_string());
        let path = SupervisorPath::root();
        let mut ctx = PipelineContext::new(child_id, path, 1, "test-correlation");

        // Set up budget evaluation showing restarts allowed
        ctx.budget_evaluation = Some(BudgetEvaluation {
            remaining_restarts: Some(3),
            limit_exhausted: false,
            escalation_policy: None,
            meltdown_outcome: MeltdownOutcome::Continue,
        });

        // Classify as external cancel
        ctx.exit_classification = Some(ExitClassification::ExternalCancel);

        // Decide action should prioritize cancel over budget
        let ctx = pipeline.stage_decide_action(ctx, 1);

        assert_eq!(
            ctx.action_decision.unwrap().action,
            ProtectionAction::SupervisedStop
        );
    }
}
