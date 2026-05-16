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
    ColdStartReason, HotLoopReason, ProtectionAction, SupervisorEvent, ThrottleGateOwner, What,
    Where,
};
use crate::id::types::{ChildId, SupervisorPath};
use crate::observe::pipeline::ObservabilityPipeline;
use crate::policy::decision::{PolicyFailureKind, TaskExit};
use crate::policy::failure_window::FailureWindow;
use crate::policy::meltdown::MeltdownTracker;
use crate::spec::supervisor::SupervisorSpec;
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
            Self::NonZeroExit { .. } => "non_zero_exit",
            Self::Crash { .. } => "crash",
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
            sequence,
            correlation_id: correlation_id.into(),
        }
    }
}

/// Six-stage supervision pipeline orchestrator.
pub struct SupervisionPipeline {
    /// Observability pipeline for event emission.
    pub observability: ObservabilityPipeline,
    /// Meltdown tracker for failure counting.
    pub meltdown_tracker: MeltdownTracker,
    /// Failure window for sliding accumulation.
    pub failure_window: FailureWindow,
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
        Self {
            observability: ObservabilityPipeline::new(journal_capacity, subscriber_capacity),
            meltdown_tracker,
            failure_window,
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
        ctx = self.stage_classify_exit(ctx, &exit);

        // Stage 2: Record Failure Window
        ctx = self.stage_record_failure_window(ctx, now);

        // Stage 3: Evaluate Budget
        ctx = self.stage_evaluate_budget(ctx, spec, tree);

        // Stage 4: Decide Action
        ctx = self.stage_decide_action(ctx);

        // Stage 5: Emit Typed Event
        ctx = self.stage_emit_typed_event(ctx, &exit, now_unix_nanos);

        // Stage 6: Execute Action
        ctx = self.stage_execute_action(ctx);

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
    fn stage_classify_exit(&self, mut ctx: PipelineContext, exit: &TaskExit) -> PipelineContext {
        let classification = match exit {
            TaskExit::Succeeded => ExitClassification::Success,
            TaskExit::Failed { kind, .. } => match kind {
                PolicyFailureKind::Cancelled => ExitClassification::ExternalCancel,
                PolicyFailureKind::Timeout => ExitClassification::Timeout,
                _ => ExitClassification::NonZeroExit { exit_code: -1 },
            },
        };

        ctx.exit_classification = Some(classification);
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
        &self,
        mut ctx: PipelineContext,
        spec: &SupervisorSpec,
        tree: &SupervisorTree,
    ) -> PipelineContext {
        // Get restart_execution_plan for this child
        let plan = restart_execution_plan(tree, spec, &ctx.child_id);

        let remaining = plan.restart_limit.map(|limit| {
            let current_count = self.failure_window.failure_count() as u32;
            limit.max_restarts.saturating_sub(current_count)
        });

        let limit_exhausted = remaining == Some(0);

        ctx.budget_evaluation = Some(BudgetEvaluation {
            remaining_restarts: remaining,
            limit_exhausted,
            escalation_policy: plan.escalation_policy.map(|p| format!("{:?}", p)),
        });

        // Set group_id from plan if available
        ctx.group_id = plan.group;

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
    fn stage_decide_action(&self, mut ctx: PipelineContext) -> PipelineContext {
        let classification = ctx.exit_classification.as_ref();
        let budget = ctx.budget_evaluation.as_ref();

        // Determine action based on classification and budget
        let action = match classification {
            Some(ExitClassification::ExternalCancel) | Some(ExitClassification::ManualStop) => {
                // Cancel/stop signals have highest priority - do not restart
                ProtectionAction::SupervisedStop
            }
            _ => {
                // Check budget exhaustion
                if let Some(budget_eval) = budget {
                    if budget_eval.limit_exhausted {
                        ProtectionAction::RestartDenied
                    } else {
                        ProtectionAction::RestartAllowed
                    }
                } else {
                    ProtectionAction::RestartAllowed
                }
            }
        };

        let reason = match &action {
            ProtectionAction::SupervisedStop => "external_cancel_or_manual_stop".to_string(),
            ProtectionAction::RestartDenied => "restart_limit_exhausted".to_string(),
            ProtectionAction::RestartAllowed => "within_restart_budget".to_string(),
            _ => "default".to_string(),
        };

        ctx.action_decision = Some(ActionDecision {
            action,
            delay_ms: None,
            reason,
        });

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

        // Emit through observability pipeline
        let _lagged = self.observability.emit(event);

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
    fn stage_execute_action(&self, ctx: PipelineContext) -> PipelineContext {
        // In a full implementation, this would:
        // - Actually restart the child if RestartAllowed
        // - Queue the restart if RestartQueued
        // - Block restart if RestartDenied or higher
        // For now, we just record the decision

        let _execution_result = if let Some(ref decision) = ctx.action_decision {
            format!("action={:?}", decision.action)
        } else {
            "no_decision".to_string()
        };

        // TODO(T014): Implement actual execution logic:
        // - Check that execute_action does not conflict with earlier stage decisions
        // - Respect restart delays from backoff policy
        // - Handle concurrency throttle gates

        ctx
    }
}

#[cfg(test)]
mod tests {
    use crate::event::payload::ProtectionAction;
    use crate::id::types::{ChildId, SupervisorPath};
    use crate::policy::decision::{PolicyFailureKind, TaskExit};
    use crate::policy::failure_window::{FailureWindow, FailureWindowConfig};
    use crate::policy::meltdown::{MeltdownPolicy, MeltdownTracker};
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
        let ctx = pipeline.stage_classify_exit(ctx, &exit);

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
        let ctx = pipeline.stage_classify_exit(ctx, &exit);

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
        });

        // Classify as external cancel
        ctx.exit_classification = Some(ExitClassification::ExternalCancel);

        // Decide action should prioritize cancel over budget
        let ctx = pipeline.stage_decide_action(ctx);

        assert_eq!(
            ctx.action_decision.unwrap().action,
            ProtectionAction::SupervisedStop
        );
    }
}
