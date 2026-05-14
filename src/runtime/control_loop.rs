//! Runtime control loop.
//!
//! This module executes control-plane commands, receives child attempt exits,
//! and applies supervisor restart strategy decisions.

use crate::child_runner::attempt::TaskExit;
use crate::child_runner::runner::{ChildRunReport, ChildRunner, wait_for_report};
use crate::control::command::{CommandResult, ControlCommand, CurrentState, ManagedChildState};
use crate::error::types::SupervisorError;
use crate::id::types::ChildId;
use crate::policy::backoff::BackoffPolicy;
use crate::policy::decision::{
    PolicyEngine, RestartDecision, RestartPolicy, TaskExit as PolicyTaskExit,
};
use crate::registry::entry::{ChildRuntime, ChildRuntimeStatus};
use crate::registry::store::RegistryStore;
use crate::runtime::lifecycle::RuntimeExitReport;
use crate::runtime::message::{ChildAttemptMessage, ControlPlaneMessage, RuntimeLoopMessage};
use crate::runtime::shutdown_pipeline::{ActiveChildAttempt, ShutdownPipeline};
use crate::shutdown::coordinator::{ShutdownCoordinator, ShutdownResult};
use crate::shutdown::report::{
    ChildShutdownOutcome, ChildShutdownOutcomeInput, ChildShutdownStatus, ShutdownPipelineReport,
    ShutdownReconcileReport,
};
use crate::shutdown::stage::{ShutdownCause, ShutdownPhase, ShutdownPolicy};
use crate::spec::child::RestartPolicy as ChildRestartPolicy;
use crate::spec::supervisor::SupervisorSpec;
use crate::tree::builder::SupervisorTree;
use crate::tree::order::{restart_execution_plan, shutdown_order, startup_order};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, mpsc};
use tokio::time::{Instant, timeout};

/// Mutable state owned by the control loop.
#[derive(Debug)]
pub struct RuntimeControlState {
    /// Shutdown state machine used by tree-level shutdown commands.
    shutdown: ShutdownCoordinator,
    /// Runtime-owned shutdown pipeline state and cached report.
    shutdown_pipeline: ShutdownPipeline,
    /// Runtime child states set by explicit control commands.
    children: HashMap<ChildId, ManagedChildState>,
    /// Active child attempts that can be cancelled or aborted.
    active_attempts: HashMap<ChildId, ActiveChildAttempt>,
    /// Dynamic child manifests accepted after startup.
    manifests: Vec<String>,
    /// Registry that owns declared child runtime records.
    registry: RegistryStore,
    /// Built supervisor tree used for order and scope planning.
    tree: SupervisorTree,
    /// Supervisor specification that owns strategy and dynamic policies.
    spec: SupervisorSpec,
    /// Policy engine used to convert task exits into restart decisions.
    policy_engine: PolicyEngine,
    /// Sender used by spawned child attempts to report runtime messages.
    command_sender: mpsc::Sender<RuntimeLoopMessage>,
}

impl RuntimeControlState {
    /// Creates control state from a supervisor specification.
    ///
    /// # Arguments
    ///
    /// - `spec`: Supervisor declaration that owns children and strategy.
    /// - `shutdown_policy`: Policy used by the shutdown coordinator.
    /// - `command_sender`: Sender used by child attempts to report exits.
    ///
    /// # Returns
    ///
    /// Returns a [`RuntimeControlState`] value.
    pub fn new(
        spec: SupervisorSpec,
        shutdown_policy: ShutdownPolicy,
        command_sender: mpsc::Sender<RuntimeLoopMessage>,
    ) -> Result<Self, SupervisorError> {
        let tree = SupervisorTree::build(&spec)?;
        let mut registry = RegistryStore::new();
        registry.register_tree(&tree)?;
        Ok(Self {
            shutdown: ShutdownCoordinator::new(shutdown_policy),
            shutdown_pipeline: ShutdownPipeline::new(),
            children: HashMap::new(),
            active_attempts: HashMap::new(),
            manifests: Vec::new(),
            registry,
            tree,
            spec,
            policy_engine: PolicyEngine::new(),
            command_sender,
        })
    }

    /// Starts every declared child in supervisor startup order.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    pub fn start_declared_children(&mut self) {
        let child_ids = startup_order(&self.tree)
            .into_iter()
            .map(|node| node.child.id.clone())
            .collect::<Vec<_>>();
        for child_id in child_ids {
            self.spawn_child_attempt(child_id, false, Duration::ZERO);
        }
    }

    /// Executes one control command.
    ///
    /// # Arguments
    ///
    /// - `command`: Command received by the runtime.
    /// - `event_sender`: Event channel used for lifecycle text.
    ///
    /// # Returns
    ///
    /// Returns a command result.
    pub async fn execute_control(
        &mut self,
        command: ControlCommand,
        event_sender: &broadcast::Sender<String>,
    ) -> Result<CommandResult, SupervisorError> {
        command.validate_audit_metadata()?;
        match command {
            ControlCommand::AddChild { child_manifest, .. } => {
                self.ensure_dynamic_child_allowed()?;
                self.manifests.push(child_manifest.clone());
                Ok(CommandResult::ChildAdded { child_manifest })
            }
            ControlCommand::RemoveChild { child_id, .. } => {
                Ok(self.set_child_state(child_id, ManagedChildState::Removed))
            }
            ControlCommand::RestartChild { child_id, .. } => {
                self.spawn_child_attempt(child_id.clone(), true, Duration::ZERO);
                Ok(self.set_child_state(child_id, ManagedChildState::Running))
            }
            ControlCommand::PauseChild { child_id, .. } => {
                Ok(self.set_child_state(child_id, ManagedChildState::Paused))
            }
            ControlCommand::ResumeChild { child_id, .. } => {
                Ok(self.set_child_state(child_id, ManagedChildState::Running))
            }
            ControlCommand::QuarantineChild { child_id, .. } => {
                Ok(self.set_child_state(child_id, ManagedChildState::Quarantined))
            }
            ControlCommand::ShutdownTree { meta } => {
                let result = self
                    .execute_shutdown(meta.requested_by, meta.reason, event_sender)
                    .await?;
                Ok(CommandResult::Shutdown { result })
            }
            ControlCommand::CurrentState { .. } => Ok(CommandResult::CurrentState {
                state: CurrentState {
                    child_count: self.dynamic_child_count(),
                    shutdown_completed: self.shutdown.phase()
                        == crate::shutdown::stage::ShutdownPhase::Completed,
                },
            }),
        }
    }

    /// Applies policy to a completed child attempt.
    ///
    /// # Arguments
    ///
    /// - `report`: Completed child attempt report.
    /// - `event_sender`: Event channel used for lifecycle text.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    pub fn handle_child_exit(
        &mut self,
        report: ChildRunReport,
        event_sender: &broadcast::Sender<String>,
    ) {
        let child_id = report.runtime.id.clone();
        let was_active = self.active_attempts.remove(&child_id).is_some();
        let late_report = !was_active && self.shutdown.phase() == ShutdownPhase::Completed;
        self.record_child_exit(report);
        let _ignored = event_sender.send(format!("child_exit:{child_id}"));
        if late_report {
            let _ignored = event_sender.send(format!("child_shutdown_late_report:{child_id}"));
        }
        if !self.should_apply_automatic_policy(&child_id) {
            return;
        }
        let Some(decision) = self.restart_decision(&child_id) else {
            return;
        };
        self.execute_restart_decision(child_id, decision, event_sender);
    }

    /// Records a failed child start.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Child identifier whose attempt failed.
    /// - `message`: Diagnostic error message.
    /// - `event_sender`: Event channel used for lifecycle text.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    pub fn handle_child_start_failed(
        &mut self,
        child_id: ChildId,
        message: String,
        event_sender: &broadcast::Sender<String>,
    ) {
        let _ignored = event_sender.send(format!("child_start_failed:{child_id}:{message}"));
        let _result = self.set_child_state(child_id, ManagedChildState::Quarantined);
    }

    /// Executes the real shutdown pipeline.
    ///
    /// # Arguments
    ///
    /// - `requested_by`: Actor that requested shutdown.
    /// - `reason`: Human-readable shutdown reason.
    /// - `event_sender`: Event channel used for lifecycle text.
    ///
    /// # Returns
    ///
    /// Returns a [`ShutdownResult`] with a completed report attached.
    async fn execute_shutdown(
        &mut self,
        requested_by: String,
        reason: String,
        event_sender: &broadcast::Sender<String>,
    ) -> Result<ShutdownResult, SupervisorError> {
        if let Some(report) = self.shutdown_pipeline.cached_report() {
            return Ok(self
                .shutdown
                .result_with_report(report.as_idempotent(), true));
        }

        let cause = ShutdownCause::new(requested_by, reason);
        let requested = self.shutdown.request_stop(cause);
        let started_at_unix_nanos = unix_epoch_nanos();
        let wait_order = self.shutdown_wait_order();
        let mut outcomes = HashMap::<ChildId, ChildShutdownOutcome>::new();
        let _ignored = event_sender.send(format!(
            "shutdown_phase_changed:{:?}:{:?}",
            ShutdownPhase::Idle,
            self.shutdown.phase()
        ));
        self.deliver_shutdown_cancellations(&wait_order, event_sender);

        self.advance_shutdown_phase(event_sender);
        self.drain_graceful_children(&wait_order, &mut outcomes, event_sender)
            .await;

        self.advance_shutdown_phase(event_sender);
        self.abort_remaining_children(&wait_order, &mut outcomes, event_sender)
            .await;

        self.advance_shutdown_phase(event_sender);
        self.reconcile_shutdown_outcomes(&wait_order, &mut outcomes);
        let reconcile = ShutdownReconcileReport::core_runtime_completed();

        let from = self.shutdown.phase();
        self.shutdown.complete();
        let _ignored = event_sender.send(format!(
            "shutdown_phase_changed:{from:?}:{:?}",
            self.shutdown.phase()
        ));
        let ordered_outcomes = wait_order
            .iter()
            .filter_map(|child_id| outcomes.remove(child_id))
            .collect::<Vec<_>>();
        let report = ShutdownPipelineReport {
            cause: requested.cause,
            started_at_unix_nanos,
            completed_at_unix_nanos: unix_epoch_nanos(),
            phase: self.shutdown.phase(),
            outcomes: ordered_outcomes,
            reconcile,
            idempotent: false,
        };
        let _ignored = event_sender.send(format!("shutdown_completed:{}", report.outcomes.len()));
        self.shutdown_pipeline.cache_report(report.clone());
        Ok(self.shutdown.result_with_report(report, false))
    }

    /// Advances the shutdown phase and emits a phase event.
    ///
    /// # Arguments
    ///
    /// - `event_sender`: Event channel used for lifecycle text.
    ///
    /// # Returns
    ///
    /// Returns the phase after advancing.
    fn advance_shutdown_phase(
        &mut self,
        event_sender: &broadcast::Sender<String>,
    ) -> ShutdownPhase {
        let from = self.shutdown.phase();
        let to = self.shutdown.advance();
        let _ignored = event_sender.send(format!("shutdown_phase_changed:{from:?}:{to:?}"));
        to
    }

    /// Returns declared children in shutdown wait order.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns child identifiers in shutdown order.
    fn shutdown_wait_order(&self) -> Vec<ChildId> {
        shutdown_order(&self.tree)
            .into_iter()
            .map(|node| node.child.id.clone())
            .collect()
    }

    /// Delivers cancellation to every active child attempt.
    ///
    /// # Arguments
    ///
    /// - `wait_order`: Stable shutdown order for declared children.
    /// - `event_sender`: Event channel used for lifecycle text.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    fn deliver_shutdown_cancellations(
        &mut self,
        wait_order: &[ChildId],
        event_sender: &broadcast::Sender<String>,
    ) {
        for child_id in wait_order {
            let Some(attempt) = self.active_attempts.get_mut(child_id) else {
                continue;
            };
            attempt.cancel();
            let _ignored = event_sender.send(format!(
                "child_shutdown_cancel_delivered:{}:{}:{}",
                attempt.child_id, attempt.generation.value, attempt.attempt.value
            ));
        }
    }

    /// Drains cooperative child attempts within the graceful timeout budget.
    ///
    /// # Arguments
    ///
    /// - `wait_order`: Stable shutdown order for declared children.
    /// - `outcomes`: Output map for completed child outcomes.
    /// - `event_sender`: Event channel used for lifecycle text.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    async fn drain_graceful_children(
        &mut self,
        wait_order: &[ChildId],
        outcomes: &mut HashMap<ChildId, ChildShutdownOutcome>,
        event_sender: &broadcast::Sender<String>,
    ) {
        let deadline = Instant::now() + self.shutdown.policy.graceful_timeout;
        for child_id in wait_order {
            if outcomes.contains_key(child_id) {
                continue;
            }
            let Some(mut attempt) = self.active_attempts.remove(child_id) else {
                continue;
            };
            let completed = match remaining_duration(deadline) {
                Some(remaining) => timeout(remaining, attempt.wait_for_report()).await.ok(),
                None => None,
            };
            match completed {
                Some(Ok(report)) => {
                    let outcome = outcome_from_report(
                        &attempt,
                        &report,
                        ChildShutdownStatus::Graceful,
                        ShutdownPhase::GracefulDrain,
                        "child completed during graceful drain",
                    );
                    self.record_child_exit(report);
                    let _ignored = event_sender.send(format!("child_shutdown_graceful:{child_id}"));
                    outcomes.insert(child_id.clone(), outcome);
                }
                Some(Err(error)) => {
                    outcomes.insert(
                        child_id.clone(),
                        outcome_from_error(
                            &attempt,
                            ChildShutdownStatus::Graceful,
                            ShutdownPhase::GracefulDrain,
                            error,
                        ),
                    );
                }
                None => {
                    self.active_attempts.insert(child_id.clone(), attempt);
                }
            }
        }
    }

    /// Aborts children that did not complete during graceful drain.
    ///
    /// # Arguments
    ///
    /// - `wait_order`: Stable shutdown order for declared children.
    /// - `outcomes`: Output map for completed child outcomes.
    /// - `event_sender`: Event channel used for lifecycle text.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    async fn abort_remaining_children(
        &mut self,
        wait_order: &[ChildId],
        outcomes: &mut HashMap<ChildId, ChildShutdownOutcome>,
        event_sender: &broadcast::Sender<String>,
    ) {
        let policy = self.shutdown.policy;
        for child_id in wait_order {
            if outcomes.contains_key(child_id) {
                continue;
            }
            let Some(mut attempt) = self.active_attempts.remove(child_id) else {
                continue;
            };
            if !policy.abort_after_timeout {
                self.wait_for_late_report(
                    child_id,
                    attempt,
                    policy.abort_wait,
                    outcomes,
                    event_sender,
                )
                .await;
                continue;
            }
            attempt.abort();
            let _ignored = event_sender.send(format!(
                "child_shutdown_abort_requested:{}",
                attempt.child_id
            ));
            match timeout(policy.abort_wait, attempt.wait_for_report()).await {
                Ok(Ok(report)) => {
                    let outcome = outcome_from_report(
                        &attempt,
                        &report,
                        ChildShutdownStatus::Aborted,
                        ShutdownPhase::AbortStragglers,
                        "child completed after abort request",
                    );
                    self.record_child_exit(report);
                    let _ignored = event_sender.send(format!("child_shutdown_aborted:{child_id}"));
                    outcomes.insert(child_id.clone(), outcome);
                }
                Ok(Err(error)) => {
                    outcomes.insert(
                        child_id.clone(),
                        outcome_from_error(
                            &attempt,
                            ChildShutdownStatus::AbortFailed,
                            ShutdownPhase::AbortStragglers,
                            error,
                        ),
                    );
                }
                Err(_elapsed) => {
                    outcomes.insert(
                        child_id.clone(),
                        ChildShutdownOutcome::new(ChildShutdownOutcomeInput {
                            child_id: attempt.child_id,
                            path: attempt.path,
                            generation: attempt.generation,
                            attempt: attempt.attempt,
                            status: ChildShutdownStatus::AbortFailed,
                            cancel_delivered: attempt.cancel_delivered,
                            exit: None,
                            phase: ShutdownPhase::AbortStragglers,
                            reason: "child did not complete after abort request".to_owned(),
                        }),
                    );
                }
            }
        }
    }

    /// Waits for a late report when abort is disabled by policy.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Child whose attempt is being reconciled.
    /// - `attempt`: Active attempt removed from runtime tracking.
    /// - `wait`: Late report wait budget.
    /// - `outcomes`: Output map for completed child outcomes.
    /// - `event_sender`: Event channel used for lifecycle text.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    async fn wait_for_late_report(
        &mut self,
        child_id: &ChildId,
        mut attempt: ActiveChildAttempt,
        wait: Duration,
        outcomes: &mut HashMap<ChildId, ChildShutdownOutcome>,
        event_sender: &broadcast::Sender<String>,
    ) {
        match timeout(wait, attempt.wait_for_report()).await {
            Ok(Ok(report)) => {
                let outcome = outcome_from_report(
                    &attempt,
                    &report,
                    ChildShutdownStatus::LateReport,
                    ShutdownPhase::AbortStragglers,
                    "child reported after graceful timeout",
                );
                self.record_child_exit(report);
                let _ignored = event_sender.send(format!("child_shutdown_late_report:{child_id}"));
                outcomes.insert(child_id.clone(), outcome);
            }
            Ok(Err(error)) => {
                outcomes.insert(
                    child_id.clone(),
                    outcome_from_error(
                        &attempt,
                        ChildShutdownStatus::LateReport,
                        ShutdownPhase::AbortStragglers,
                        error,
                    ),
                );
            }
            Err(_elapsed) => {
                outcomes.insert(
                    child_id.clone(),
                    ChildShutdownOutcome::new(ChildShutdownOutcomeInput {
                        child_id: attempt.child_id,
                        path: attempt.path,
                        generation: attempt.generation,
                        attempt: attempt.attempt,
                        status: ChildShutdownStatus::AbortFailed,
                        cancel_delivered: attempt.cancel_delivered,
                        exit: None,
                        phase: ShutdownPhase::AbortStragglers,
                        reason: "abort disabled and child did not report before reconcile"
                            .to_owned(),
                    }),
                );
            }
        }
    }

    /// Adds already-exited outcomes for declared children with no active task.
    ///
    /// # Arguments
    ///
    /// - `wait_order`: Stable shutdown order for declared children.
    /// - `outcomes`: Output map for completed child outcomes.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    fn reconcile_shutdown_outcomes(
        &self,
        wait_order: &[ChildId],
        outcomes: &mut HashMap<ChildId, ChildShutdownOutcome>,
    ) {
        for child_id in wait_order {
            if outcomes.contains_key(child_id) {
                continue;
            }
            let Some(runtime) = self.registry.child(child_id) else {
                continue;
            };
            outcomes.insert(
                child_id.clone(),
                ChildShutdownOutcome::new(ChildShutdownOutcomeInput {
                    child_id: runtime.id.clone(),
                    path: runtime.path.clone(),
                    generation: runtime.generation,
                    attempt: runtime.attempt,
                    status: ChildShutdownStatus::AlreadyExited,
                    cancel_delivered: false,
                    exit: runtime.last_exit.clone(),
                    phase: ShutdownPhase::Reconcile,
                    reason: "child had no active attempt during shutdown".to_owned(),
                }),
            );
        }
    }

    /// Sets a child state and reports whether the operation was idempotent.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Target child identifier.
    /// - `next`: Requested managed child state.
    ///
    /// # Returns
    ///
    /// Returns a [`CommandResult::ChildState`] value.
    fn set_child_state(&mut self, child_id: ChildId, next: ManagedChildState) -> CommandResult {
        let previous = self.children.insert(child_id.clone(), next);
        CommandResult::ChildState {
            child_id,
            state: next,
            idempotent: previous == Some(next),
        }
    }

    /// Records the completed attempt in the registry.
    ///
    /// # Arguments
    ///
    /// - `report`: Completed child attempt report.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    fn record_child_exit(&mut self, report: ChildRunReport) {
        let child_id = report.runtime.id.clone();
        if let Some(runtime) = self.registry.child_mut(&child_id) {
            runtime.last_exit = Some(report.exit);
            runtime.status = ChildRuntimeStatus::Exited;
            runtime.generation = report.runtime.generation;
            runtime.attempt = report.runtime.attempt;
            runtime.restart_count = report.runtime.restart_count;
        }
    }

    /// Reports whether automatic policy may still act on a child.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Child whose latest exit is being evaluated.
    ///
    /// # Returns
    ///
    /// Returns `true` when the runtime may restart the child.
    fn should_apply_automatic_policy(&self, child_id: &ChildId) -> bool {
        if self.shutdown.phase() != crate::shutdown::stage::ShutdownPhase::Idle {
            return false;
        }
        !matches!(
            self.children.get(child_id),
            Some(ManagedChildState::Paused)
                | Some(ManagedChildState::Quarantined)
                | Some(ManagedChildState::Removed)
        )
    }

    /// Calculates a restart decision for the latest child exit.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Child whose latest exit is being evaluated.
    ///
    /// # Returns
    ///
    /// Returns a restart decision when the child is known.
    fn restart_decision(&self, child_id: &ChildId) -> Option<RestartDecision> {
        let runtime = self.registry.child(child_id)?;
        let exit = runtime.last_exit.as_ref()?;
        let policy_exit = policy_task_exit(exit);
        let restart_policy = restart_policy(runtime.spec.restart_policy);
        let backoff = backoff_policy(runtime.spec.backoff_policy);
        Some(self.policy_engine.decide(
            restart_policy,
            policy_exit,
            runtime.attempt.value,
            &backoff,
        ))
    }

    /// Executes a restart decision after a child exit.
    ///
    /// # Arguments
    ///
    /// - `failed_child`: Child whose exit triggered the decision.
    /// - `decision`: Restart decision returned by the policy engine.
    /// - `event_sender`: Event channel used for lifecycle text.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    fn execute_restart_decision(
        &mut self,
        failed_child: ChildId,
        decision: RestartDecision,
        event_sender: &broadcast::Sender<String>,
    ) {
        match decision {
            RestartDecision::RestartAfter { delay } => {
                self.restart_strategy_scope(failed_child, delay, event_sender);
            }
            RestartDecision::Quarantine => {
                let _result = self.set_child_state(failed_child, ManagedChildState::Quarantined);
            }
            RestartDecision::ShutdownTree => {
                let cause = ShutdownCause::new("runtime", "policy requested tree shutdown");
                let _result = self.shutdown.request_stop(cause);
            }
            RestartDecision::EscalateToParent | RestartDecision::DoNotRestart => {}
        }
    }

    /// Restarts every child selected by the current execution plan.
    ///
    /// # Arguments
    ///
    /// - `failed_child`: Child whose exit triggered the restart scope.
    /// - `delay`: Delay before every selected child is restarted.
    /// - `event_sender`: Event channel used for lifecycle text.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    fn restart_strategy_scope(
        &mut self,
        failed_child: ChildId,
        delay: Duration,
        event_sender: &broadcast::Sender<String>,
    ) {
        let plan = restart_execution_plan(&self.tree, &self.spec, &failed_child);
        let scope_label = child_scope_label(&plan.scope);
        let group_label = plan.group.as_deref().unwrap_or("supervisor");
        let _ignored = event_sender.send(format!(
            "restart_plan:{:?}:{group_label}:{scope_label}",
            plan.strategy
        ));
        for child_id in plan.scope {
            self.spawn_child_attempt(child_id, true, delay);
        }
    }

    /// Ensures that the dynamic supervisor accepts another child manifest.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` when another dynamic child can be added.
    fn ensure_dynamic_child_allowed(&self) -> Result<(), SupervisorError> {
        let current_child_count = self.dynamic_child_count();
        if self
            .spec
            .dynamic_supervisor_policy
            .allows_addition(current_child_count)
        {
            return Ok(());
        }
        Err(SupervisorError::InvalidTransition {
            message: "dynamic supervisor child limit reached".to_owned(),
        })
    }

    /// Counts declared and dynamic child records.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the number of declared children plus accepted dynamic manifests.
    fn dynamic_child_count(&self) -> usize {
        self.registry
            .declaration_order()
            .len()
            .saturating_add(self.manifests.len())
    }

    /// Spawns one child attempt and reports the exit back to this control loop.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Child that should run.
    /// - `is_restart`: Whether this attempt is a restart attempt.
    /// - `delay`: Delay before the attempt starts.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    fn spawn_child_attempt(&mut self, child_id: ChildId, is_restart: bool, delay: Duration) {
        let Some(runtime) = self.prepare_child_attempt(&child_id, is_restart) else {
            return;
        };
        let sender = self.command_sender.clone();
        if !delay.is_zero() {
            tokio::spawn(async move {
                tokio::time::sleep(delay).await;
                let child_id = runtime.id.clone();
                let result = ChildRunner::new().run_once(runtime).await;
                send_child_result(sender, child_id, result).await;
            });
            return;
        }

        let child_id = runtime.id.clone();
        let path = runtime.path.clone();
        let generation = runtime.generation;
        let attempt = runtime.attempt;
        if let Some(mut existing) = self.active_attempts.remove(&child_id) {
            existing.abort();
        }
        match ChildRunner::new().spawn_once(runtime) {
            Ok(handle) => {
                let mut completion_receiver = handle.completion_receiver.clone();
                self.active_attempts.insert(
                    child_id.clone(),
                    ActiveChildAttempt::new(child_id.clone(), path, generation, attempt, handle),
                );
                tokio::spawn(async move {
                    let result = wait_for_report(&mut completion_receiver).await;
                    send_child_result(sender, child_id, result).await;
                });
            }
            Err(error) => {
                tokio::spawn(async move {
                    send_child_result(sender, child_id, Err(error)).await;
                });
            }
        }
    }

    /// Prepares registry state for one child attempt.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Child that should run.
    /// - `is_restart`: Whether this attempt is a restart attempt.
    ///
    /// # Returns
    ///
    /// Returns a runtime record for the child runner.
    fn prepare_child_attempt(
        &mut self,
        child_id: &ChildId,
        is_restart: bool,
    ) -> Option<ChildRuntime> {
        let runtime = self.registry.child_mut(child_id)?;
        if is_restart {
            runtime.attempt = runtime.attempt.next();
            runtime.generation = runtime.generation.next();
            runtime.restart_count = runtime.restart_count.saturating_add(1);
        }
        runtime.status = ChildRuntimeStatus::Starting;
        self.children
            .insert(child_id.clone(), ManagedChildState::Running);
        Some(runtime.clone())
    }
}

/// Runs the control loop until all command senders are dropped.
///
/// # Arguments
///
/// - `state`: Runtime state initialized from the supervisor specification.
/// - `receiver`: Runtime command receiver.
/// - `event_sender`: Event channel used for audit text.
///
/// # Returns
///
/// Returns a [`RuntimeExitReport`] when the control loop ends.
pub async fn run_control_loop(
    mut state: RuntimeControlState,
    mut receiver: mpsc::Receiver<RuntimeLoopMessage>,
    event_sender: broadcast::Sender<String>,
) -> RuntimeExitReport {
    state.start_declared_children();
    while let Some(message) = receiver.recv().await {
        match message {
            RuntimeLoopMessage::Control {
                command,
                reply_sender,
            } => {
                let command_name = command_name(&command);
                let result = state.execute_control(command, &event_sender).await;
                let _ignored = event_sender.send(format!("control_command:{command_name}"));
                let _ignored = reply_sender.send(result);
            }
            RuntimeLoopMessage::ChildAttempt(ChildAttemptMessage::Exited { report }) => {
                state.handle_child_exit(*report, &event_sender);
            }
            RuntimeLoopMessage::ChildAttempt(ChildAttemptMessage::StartFailed {
                child_id,
                message,
            }) => {
                state.handle_child_start_failed(child_id, message, &event_sender);
            }
            RuntimeLoopMessage::ControlPlane(ControlPlaneMessage::Shutdown {
                meta,
                reply_sender,
            }) => {
                let _ignored = event_sender.send(format!(
                    "runtime_control_loop_shutdown_requested:{}:{}",
                    meta.requested_by, meta.reason
                ));
                match meta.validate() {
                    Ok(()) => {
                        let report = RuntimeExitReport::completed(
                            "shutdown",
                            format!("runtime control plane shutdown requested: {}", meta.reason),
                        );
                        let _ignored = reply_sender.send(Ok(report.clone()));
                        return report;
                    }
                    Err(error) => {
                        let _ignored = reply_sender.send(Err(error));
                        continue;
                    }
                }
            }
        }
    }
    RuntimeExitReport::completed("message_loop", "runtime command channel closed")
}

/// Returns a stable command name for audit text.
///
/// # Arguments
///
/// - `command`: Command being executed.
///
/// # Returns
///
/// Returns a static command name.
fn command_name(command: &ControlCommand) -> &'static str {
    match command {
        ControlCommand::AddChild { .. } => "add_child",
        ControlCommand::RemoveChild { .. } => "remove_child",
        ControlCommand::RestartChild { .. } => "restart_child",
        ControlCommand::PauseChild { .. } => "pause_child",
        ControlCommand::ResumeChild { .. } => "resume_child",
        ControlCommand::QuarantineChild { .. } => "quarantine_child",
        ControlCommand::ShutdownTree { .. } => "shutdown_tree",
        ControlCommand::CurrentState { .. } => "current_state",
    }
}

/// Sends a child run result back to the control loop.
///
/// # Arguments
///
/// - `sender`: Runtime command sender.
/// - `child_id`: Child identifier used when the run fails before reporting.
/// - `result`: Child run result.
///
/// # Returns
///
/// This function does not return a value.
async fn send_child_result(
    sender: mpsc::Sender<RuntimeLoopMessage>,
    child_id: ChildId,
    result: Result<ChildRunReport, SupervisorError>,
) {
    let message = match result {
        Ok(report) => RuntimeLoopMessage::ChildAttempt(ChildAttemptMessage::Exited {
            report: Box::new(report),
        }),
        Err(error) => RuntimeLoopMessage::ChildAttempt(ChildAttemptMessage::StartFailed {
            child_id,
            message: error.to_string(),
        }),
    };
    let _ignored = sender.send(message).await;
}

/// Maps child restart policy into policy-engine restart policy.
///
/// # Arguments
///
/// - `policy`: Restart policy stored on the child declaration.
///
/// # Returns
///
/// Returns the equivalent policy-engine value.
fn restart_policy(policy: ChildRestartPolicy) -> RestartPolicy {
    match policy {
        ChildRestartPolicy::Permanent => RestartPolicy::Permanent,
        ChildRestartPolicy::Transient => RestartPolicy::Transient,
        ChildRestartPolicy::Temporary => RestartPolicy::Temporary,
    }
}

/// Maps child backoff policy into policy-engine backoff policy.
///
/// # Arguments
///
/// - `policy`: Backoff policy stored on the child declaration.
///
/// # Returns
///
/// Returns the equivalent policy-engine value.
fn backoff_policy(policy: crate::spec::child::BackoffPolicy) -> BackoffPolicy {
    let jitter_percent = (policy.jitter_ratio * 100.0).round().clamp(0.0, 100.0) as u8;
    BackoffPolicy::new(
        policy.initial_delay,
        policy.max_delay,
        jitter_percent,
        policy.max_delay,
    )
}

/// Maps a child-runner exit into policy-engine task exit.
///
/// # Arguments
///
/// - `exit`: Exit reported by the child runner.
///
/// # Returns
///
/// Returns the policy-engine exit value.
fn policy_task_exit(exit: &TaskExit) -> PolicyTaskExit {
    match exit.failure_kind() {
        Some(kind) => PolicyTaskExit::Failed { kind: kind.into() },
        None => PolicyTaskExit::Succeeded,
    }
}

/// Formats a restart scope for lifecycle events.
///
/// # Arguments
///
/// - `scope`: Child identifiers selected by strategy.
///
/// # Returns
///
/// Returns a comma-separated child identifier list.
fn child_scope_label(scope: &[ChildId]) -> String {
    scope
        .iter()
        .map(|child_id| child_id.value.clone())
        .collect::<Vec<_>>()
        .join(",")
}

/// Builds a child shutdown outcome from a completed run report.
///
/// # Arguments
///
/// - `attempt`: Active attempt that produced the report.
/// - `report`: Completed child run report.
/// - `status`: Shutdown status assigned to the report.
/// - `phase`: Shutdown phase where the report was consumed.
/// - `reason`: Human-readable diagnostic reason.
///
/// # Returns
///
/// Returns a [`ChildShutdownOutcome`].
fn outcome_from_report(
    attempt: &ActiveChildAttempt,
    report: &ChildRunReport,
    status: ChildShutdownStatus,
    phase: ShutdownPhase,
    reason: impl Into<String>,
) -> ChildShutdownOutcome {
    ChildShutdownOutcome::new(ChildShutdownOutcomeInput {
        child_id: attempt.child_id.clone(),
        path: attempt.path.clone(),
        generation: attempt.generation,
        attempt: attempt.attempt,
        status,
        cancel_delivered: attempt.cancel_delivered,
        exit: Some(report.exit.clone()),
        phase,
        reason: reason.into(),
    })
}

/// Builds a child shutdown outcome from a run report error.
///
/// # Arguments
///
/// - `attempt`: Active attempt that produced the error.
/// - `status`: Shutdown status assigned to the error.
/// - `phase`: Shutdown phase where the error was consumed.
/// - `error`: Error returned by the child run observer.
///
/// # Returns
///
/// Returns a [`ChildShutdownOutcome`].
fn outcome_from_error(
    attempt: &ActiveChildAttempt,
    status: ChildShutdownStatus,
    phase: ShutdownPhase,
    error: SupervisorError,
) -> ChildShutdownOutcome {
    ChildShutdownOutcome::new(ChildShutdownOutcomeInput {
        child_id: attempt.child_id.clone(),
        path: attempt.path.clone(),
        generation: attempt.generation,
        attempt: attempt.attempt,
        status,
        cancel_delivered: attempt.cancel_delivered,
        exit: None,
        phase,
        reason: error.to_string(),
    })
}

/// Returns the remaining duration before a deadline.
///
/// # Arguments
///
/// - `deadline`: Monotonic deadline.
///
/// # Returns
///
/// Returns `None` when the deadline has already passed.
fn remaining_duration(deadline: Instant) -> Option<Duration> {
    deadline.checked_duration_since(Instant::now())
}

/// Returns the current Unix epoch timestamp in nanoseconds.
///
/// # Arguments
///
/// This function has no arguments.
///
/// # Returns
///
/// Returns a nanosecond timestamp, or zero if system time is before epoch.
fn unix_epoch_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos())
}
