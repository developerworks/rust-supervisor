//! Runtime control loop.
//!
//! This module executes control-plane commands, receives child child_start_count exits,
//! and applies supervisor restart strategy decisions.

use crate::child_runner::run_exit::TaskExit;
use crate::child_runner::runner::{ChildRunHandle, ChildRunReport, ChildRunner, wait_for_report};
use crate::control::command::{CommandMeta, CommandResult, ControlCommand, CurrentState};
use crate::control::outcome::{
    ChildAttemptStatus, ChildControlFailure, ChildControlFailurePhase, ChildControlOperation,
    ChildControlResult, ChildLivenessState, ChildStopState, GenerationFenceDecision,
    GenerationFenceOutcome, GenerationFencePhase, PendingRestart, RestartLimitState,
    StaleAttemptReport, StaleReportHandling,
};
use crate::error::types::SupervisorError;
use crate::event::payload::{ProtectionAction, SupervisorEvent, ThrottleGateOwner, What, Where};
use crate::event::time::{CorrelationId, EventSequenceSource, EventTime, When};
use crate::id::types::{ChildId, ChildStartCount, Generation, SupervisorPath};
use crate::observe::fairness::FairnessProbe;
use crate::observe::pipeline::{ObservabilityPipeline, PipelineStageDiagnostic};
use crate::policy::backoff::BackoffPolicy;
use crate::policy::budget::RestartBudgetConfig;
use crate::policy::decision::{
    PolicyEngine, RestartDecision, RestartPolicy, TaskExit as PolicyTaskExit,
};
use crate::policy::failure_window::{FailureWindow, FailureWindowConfig};
use crate::policy::meltdown::{MeltdownPolicy, MeltdownTracker};
use crate::policy::role_defaults::{EffectivePolicy, OnSuccessAction};
use crate::registry::entry::{ChildRuntime, ChildRuntimeStatus};
use crate::registry::store::RegistryStore;
use crate::runtime::admission::{AdmissionConflict, AdmissionSet};
use crate::runtime::child_slot::{
    ChildExitSummary, ChildSlot, DEFAULT_HEARTBEAT_TIMEOUT_SECS, RuntimeTimeBase,
};
use crate::runtime::lifecycle::RuntimeExitReport;
use crate::runtime::message::{ChildStartMessage, ControlPlaneMessage, RuntimeLoopMessage};
use crate::runtime::pipeline::{ExitClassification, PipelineContext, SupervisionPipeline};
use crate::runtime::shutdown::{reconcile_shutdown_slots, shutdown_tree_fanout};
use crate::runtime::shutdown_pipeline::ShutdownPipeline;
use crate::shutdown::coordinator::{ShutdownCoordinator, ShutdownResult};
use crate::shutdown::report::{
    ChildShutdownOutcome, ChildShutdownOutcomeInput, ChildShutdownStatus, ShutdownPipelineReport,
    ShutdownReconcileReport,
};
use crate::shutdown::stage::{ShutdownCause, ShutdownPhase, ShutdownPolicy};
use crate::spec::child::{ChildSpec, RestartPolicy as ChildRestartPolicy};
use crate::spec::supervisor::{RestartLimit, SupervisorSpec};
use crate::tree::builder::SupervisorTree;
use crate::tree::order::{restart_execution_plan, shutdown_order, startup_order};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, mpsc};
use tokio::time::{Instant, timeout};

/// Typed event waiting for emission after mutable state borrows end.
#[derive(Debug)]
struct PendingRuntimeEvent {
    /// Child task identifier related to the event.
    child_id: ChildId,
    /// Child task path attached to the event location.
    path: SupervisorPath,
    /// Generation number attached to event timing.
    generation: Option<Generation>,
    /// Attempt attached to event timing.
    attempt: Option<ChildStartCount>,
    /// Correlation identifier attached to the event.
    correlation_id: CorrelationId,
    /// Typed event payload.
    what: What,
}

/// Mutable state owned by the control loop.
#[derive(Debug)]
pub struct RuntimeControlState {
    /// Shutdown state machine used by tree-level shutdown commands.
    shutdown: ShutdownCoordinator,
    /// Runtime-owned shutdown pipeline state and cached report.
    shutdown_pipeline: ShutdownPipeline,
    /// Runtime slots for declared children.
    slots: HashMap<ChildId, ChildSlot>,
    /// Admission set that enforces at-most-one active attempt per child.
    #[allow(dead_code)]
    admission_set: AdmissionSet,
    /// Runtime time base used for public timestamps.
    time_base: RuntimeTimeBase,
    /// Event sequence source for typed observability facts.
    event_sequences: EventSequenceSource,
    /// Shared typed observability pipeline.
    observability: Arc<Mutex<ObservabilityPipeline>>,
    /// Six-stage supervision pipeline for failure processing.
    supervision_pipeline: SupervisionPipeline,
    /// Instance-global concurrent restart throttle gate (FR-003).
    concurrent_gate: crate::runtime::concurrent_gate::SupervisorInstanceGate,
    /// Fairness probe that detects scheduling starvation (US1).
    fairness_probe: FairnessProbe,
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
    /// Sender used by spawned child start_counts to report runtime messages.
    command_sender: mpsc::Sender<RuntimeLoopMessage>,
}

/// Builds initial [`ChildSlot`] records from the registry.
fn build_initial_slots(registry: &RegistryStore) -> HashMap<ChildId, ChildSlot> {
    registry
        .declaration_order()
        .iter()
        .filter_map(|child_id| {
            registry.child(child_id).map(|runtime| {
                let slot = ChildSlot::new_placeholder(runtime.id.clone(), runtime.path.clone());
                (child_id.clone(), slot)
            })
        })
        .collect::<HashMap<_, _>>()
}

#[allow(dead_code)]
impl RuntimeControlState {
    /// Creates control state from a supervisor specification.
    ///
    /// # Arguments
    ///
    /// - `spec`: Supervisor declaration that owns children and strategy.
    /// - `shutdown_policy`: Policy used by the shutdown coordinator.
    /// - `command_sender`: Sender used by child start_counts to report exits.
    ///
    /// # Returns
    ///
    /// Returns a [`RuntimeControlState`] value.
    pub fn new(
        spec: SupervisorSpec,
        shutdown_policy: ShutdownPolicy,
        command_sender: mpsc::Sender<RuntimeLoopMessage>,
        observability: Arc<Mutex<ObservabilityPipeline>>,
    ) -> Result<Self, SupervisorError> {
        let tree = SupervisorTree::build(&spec)?;
        let mut registry = RegistryStore::new();
        registry.register_tree(&tree)?;
        let time_base = RuntimeTimeBase::new();
        let slots = build_initial_slots(&registry);

        // Initialize six-stage supervision pipeline with default configuration
        let meltdown_policy = MeltdownPolicy::new(
            3,                        // child_max_restarts
            Duration::from_secs(10),  // child_window
            5,                        // group_max_failures
            Duration::from_secs(30),  // group_window
            10,                       // supervisor_max_failures
            Duration::from_secs(60),  // supervisor_window
            Duration::from_secs(120), // reset_after
        );
        let meltdown_tracker = MeltdownTracker::new(meltdown_policy);
        let failure_config = FailureWindowConfig::time_sliding(60, 5);
        let failure_window = FailureWindow::new(failure_config);
        let supervision_pipeline = SupervisionPipeline::new(
            100,
            10,
            meltdown_tracker,
            failure_window,
            RestartBudgetConfig::new(Duration::from_secs(60), 10, 0.5),
            vec![],
        );

        // Initialize concurrent restart throttle gate (FR-003)
        let concurrent_gate = crate::runtime::concurrent_gate::SupervisorInstanceGate::new(5);

        // Initialize fairness probe with current timestamp
        let now_unix_nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let fairness_probe = FairnessProbe::new(now_unix_nanos);

        Ok(Self {
            shutdown: ShutdownCoordinator::new(shutdown_policy),
            shutdown_pipeline: ShutdownPipeline::new(),
            slots,
            admission_set: AdmissionSet::new(),
            time_base,
            event_sequences: EventSequenceSource::new(),
            observability,
            supervision_pipeline,
            concurrent_gate,
            fairness_probe,
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
            self.spawn_child_start(child_id, false, Duration::ZERO);
        }
    }

    /// Records an active attempt after `spawn_once` so exit routing matches registry identities.
    ///
    /// Immediate spawns run this inline; delayed backoff spawns deliver the same handle through
    /// [`ChildStartMessage::DelayedSpawnAttached`] so `activate_instance` stays on the control loop.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Stable child owning the spawned attempt.
    /// - `path`: Supervisor path used when inserting placeholder runtime records.
    /// - `generation`: Generation pinned from the registry [`ChildRuntime`] passed to `spawn_once`.
    /// - `attempt`: Attempt counter pinned from the same registry record.
    /// - `handle`: Runner handle carrying cancellation and completion endpoints.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    fn attach_spawned_child_handle(
        &mut self,
        child_id: ChildId,
        path: SupervisorPath,
        generation: Generation,
        attempt: ChildStartCount,
        handle: ChildRunHandle,
    ) {
        let mut completion_receiver = handle.completion_receiver.clone();
        let sender = self.command_sender.clone();
        self.slots
            .entry(child_id.clone())
            .or_insert_with(|| ChildSlot::new_placeholder(child_id.clone(), path))
            .activate(generation, attempt, ChildAttemptStatus::Running, handle);
        tokio::spawn(async move {
            let result = wait_for_report(&mut completion_receiver).await;
            send_child_result(sender, child_id, result).await;
        });
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
        self.reconcile_stop_deadlines();
        match command {
            ControlCommand::AddChild { child_manifest, .. } => {
                self.ensure_dynamic_child_allowed()?;

                // Reject add_child when shutdown is in progress.
                if self.shutdown.phase() != ShutdownPhase::Idle {
                    return Err(SupervisorError::fatal_config(
                        "Cannot add child: supervisor is shutting down",
                    ));
                }

                // Parse manifest as ChildDeclaration.
                let declaration: crate::spec::child_declaration::ChildDeclaration =
                    serde_yaml::from_str(&child_manifest).map_err(|e| {
                        SupervisorError::fatal_config(format!(
                            "Failed to parse child manifest: {e}"
                        ))
                    })?;

                // Validate declaration against existing children.
                let all_names: std::collections::HashSet<String> =
                    self.spec.children.iter().map(|c| c.name.clone()).collect();
                let mut new_names = all_names.clone();
                new_names.insert(declaration.name.clone());

                crate::spec::child_declaration::validate_child_declaration(
                    &declaration,
                    &all_names,
                )
                .map_err(|e| {
                    SupervisorError::fatal_config(format!(
                        "Child validation failed at {}: {}",
                        e.field_path, e.reason
                    ))
                })?;

                // Staged via begin_transaction — for now register directly
                // since we operate inside the control loop's mutable state.
                let child_spec =
                    crate::spec::child::ChildSpec::try_from(declaration).map_err(|e| {
                        SupervisorError::fatal_config(format!("Child conversion failed: {e:?}"))
                    })?;

                self.manifests.push(child_manifest.clone());
                self.spec.children.push(child_spec);
                Ok(CommandResult::ChildAdded { child_manifest })
            }
            ControlCommand::RemoveChild { meta, child_id } => Ok(self.execute_stop_child_control(
                child_id,
                ChildControlOperation::Removed,
                "remove_child",
                &meta,
                event_sender,
            )),
            ControlCommand::RestartChild { meta, child_id } => {
                Ok(self.execute_restart_child_control(child_id, &meta, event_sender))
            }
            ControlCommand::PauseChild { meta, child_id } => Ok(self.execute_stop_child_control(
                child_id,
                ChildControlOperation::Paused,
                "pause_child",
                &meta,
                event_sender,
            )),
            ControlCommand::ResumeChild { child_id, .. } => {
                Ok(self.set_child_state(child_id, ChildControlOperation::Active))
            }
            ControlCommand::QuarantineChild { meta, child_id } => Ok(self
                .execute_stop_child_control(
                    child_id,
                    ChildControlOperation::Quarantined,
                    "quarantine_child",
                    &meta,
                    event_sender,
                )),
            ControlCommand::ShutdownTree { meta } => {
                let result = self
                    .execute_shutdown(meta.requested_by, meta.reason, event_sender)
                    .await?;
                Ok(CommandResult::Shutdown { result })
            }
            ControlCommand::CurrentState { .. } => {
                self.reconcile_stop_deadlines();
                Ok(CommandResult::CurrentState {
                    state: self.build_current_state(),
                })
            }
        }
    }

    /// Applies policy to a completed child child_start_count.
    ///
    /// # Arguments
    ///
    /// - `report`: Completed child child_start_count report.
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
        // FR-003: Release concurrent gate slot when child exits (only if gate has active slots)
        if self.concurrent_gate.get_active_count() > 0 {
            self.concurrent_gate.release();
        }

        let child_id = report.runtime.id.clone();
        let generation = report.runtime.generation;
        let attempt = report.runtime.child_start_count;
        let exit_kind = report.exit.clone();
        let mut pending_events = Vec::new();
        let was_active = self
            .slots
            .get(&child_id)
            .is_some_and(ChildSlot::has_active_attempt);
        let matches_pending_fence = self
            .slots
            .get(&child_id)
            .and_then(|state| state.generation_fence.pending_restart.as_ref())
            .is_some_and(|pending_restart| {
                pending_restart.old_generation == generation
                    && pending_restart.old_attempt == attempt
            });
        let matches_active_attempt = self.slots.get(&child_id).is_some_and(|state| {
            state.has_active_attempt()
                && state.generation == Some(generation)
                && state.attempt == Some(attempt)
        });
        let manual_stop_requested = self
            .slots
            .get(&child_id)
            .is_some_and(|state| state.stop_state == ChildStopState::CancelDelivered);
        let mut stale_idle_report = false;
        let count_restart_failure = self.slots.get(&child_id).is_some_and(|state| {
            state.operation == ChildControlOperation::Active
                && restart_limit_counts_exit(&exit_kind)
        });
        let late_report = !was_active && self.shutdown.phase() == ShutdownPhase::Completed;
        let mut fence_pending_release = None::<PendingRestart>;

        if let Some(runtime_state) = self.slots.get_mut(&child_id) {
            if matches_pending_fence {
                if runtime_state.stop_state == ChildStopState::CancelDelivered {
                    runtime_state.stop_state = ChildStopState::Completed;
                    pending_events.push(PendingRuntimeEvent {
                        child_id: child_id.clone(),
                        path: runtime_state.path.clone(),
                        generation: Some(generation),
                        attempt: Some(attempt),
                        correlation_id: CorrelationId::from_uuid(
                            runtime_state
                                .generation_fence
                                .pending_restart
                                .as_ref()
                                .expect("matches pending implies Some")
                                .command_id,
                        ),
                        what: What::ChildControlStopCompleted {
                            child_id: child_id.clone(),
                            generation,
                            attempt,
                            exit_kind: exit_kind.clone(),
                        },
                    });
                }
                fence_pending_release = runtime_state.generation_fence.pending_restart.take();
                if let Some(pending_release) = fence_pending_release.as_ref() {
                    let drained_correlation_id =
                        CorrelationId::from_uuid(pending_release.command_id);
                    pending_events.push(PendingRuntimeEvent {
                        child_id: child_id.clone(),
                        path: runtime_state.path.clone(),
                        generation: Some(generation),
                        attempt: Some(attempt),
                        correlation_id: drained_correlation_id,
                        what: What::ChildRestartFencePendingDrained {
                            child_id: child_id.clone(),
                        },
                    });
                }
                runtime_state.generation_fence.phase = GenerationFencePhase::ReadyToStart;
                runtime_state.status = ChildAttemptStatus::Stopped;
                runtime_state.clear_instance();
            } else if matches_active_attempt
                || late_report
                || self.shutdown.phase() != ShutdownPhase::Idle
            {
                if runtime_state.stop_state == ChildStopState::CancelDelivered {
                    runtime_state.stop_state = ChildStopState::Completed;
                    pending_events.push(PendingRuntimeEvent {
                        child_id: child_id.clone(),
                        path: runtime_state.path.clone(),
                        generation: Some(generation),
                        attempt: Some(attempt),
                        correlation_id: CorrelationId::new(),
                        what: What::ChildControlStopCompleted {
                            child_id: child_id.clone(),
                            generation,
                            attempt,
                            exit_kind: exit_kind.clone(),
                        },
                    });
                }
                runtime_state.status = ChildAttemptStatus::Stopped;
                runtime_state.clear_instance();
            } else {
                stale_idle_report = true;
                let observed_at_unix_nanos = self.time_base.now_unix_nanos();
                let current_generation = runtime_state.generation;
                let current_attempt = runtime_state.attempt;
                let stale_fact = StaleAttemptReport::new(
                    child_id.clone(),
                    generation,
                    attempt,
                    current_generation,
                    current_attempt,
                    exit_kind.clone(),
                    StaleReportHandling::RecordedForAudit,
                    observed_at_unix_nanos,
                );
                runtime_state.generation_fence.last_stale_report = Some(stale_fact);
                pending_events.push(PendingRuntimeEvent {
                    child_id: child_id.clone(),
                    path: runtime_state.path.clone(),
                    generation: Some(generation),
                    attempt: Some(attempt),
                    correlation_id: CorrelationId::new(),
                    what: What::ChildAttemptStaleReport {
                        child_id: child_id.clone(),
                        reported_generation: generation,
                        reported_attempt: attempt,
                        current_generation,
                        current_attempt,
                        exit_kind: exit_kind.clone(),
                        handled_as: StaleReportHandling::RecordedForAudit,
                    },
                });
            }
        }

        if stale_idle_report {
            let _ignored = event_sender.send(format!("child_exit:{child_id}"));
            for event in pending_events {
                self.emit_pending_event(event);
            }
            self.reconcile_stop_deadlines();
            return;
        }

        self.record_child_exit(report);
        let restart_limit_refreshed =
            self.refresh_restart_limit_for_child(&child_id, count_restart_failure);
        if let Some((path, restart_limit)) = restart_limit_refreshed.clone() {
            pending_events.push(PendingRuntimeEvent {
                child_id: child_id.clone(),
                path,
                generation: Some(generation),
                attempt: Some(attempt),
                correlation_id: CorrelationId::new(),
                what: What::ChildRuntimeRestartLimitUpdated {
                    child_id: child_id.clone(),
                    restart_limit,
                },
            });
        }
        let _ignored = event_sender.send(format!("child_exit:{child_id}"));
        if late_report {
            let _ignored = event_sender.send(format!("child_shutdown_late_report:{child_id}"));
        }

        // Execute six-stage supervision pipeline for failure processing.
        let sequence = self.event_sequences.next().value;
        // T037: Generate a real CorrelationId to link budget→meltdown→escalation events.
        let correlation_id_str = format!("{}", uuid::Uuid::new_v4());
        let supervisor_path = self
            .slots
            .get(&child_id)
            .map(|state| state.path.clone())
            .unwrap_or_else(|| SupervisorPath::root().join(child_id.value.clone()));

        let mut pipeline_ctx = PipelineContext::new(
            child_id.clone(),
            supervisor_path,
            sequence,
            correlation_id_str,
        );
        pipeline_ctx.exit_classification = Some(classify_exit_for_pipeline(
            &exit_kind,
            manual_stop_requested,
        ));
        pipeline_ctx.effective_policy = self
            .registry
            .child(&child_id)
            .map(|runtime| prepare_effective_policy(&runtime.spec));

        // Convert TaskExit to PolicyTaskExit for pipeline.
        let policy_exit = policy_task_exit(&exit_kind);

        // Execute the complete six-stage pipeline.
        let pipeline_result = self.supervision_pipeline.execute_pipeline(
            pipeline_ctx,
            policy_exit,
            &self.spec,
            &self.tree,
        );
        self.record_pipeline_stage_diagnostics(&pipeline_result.stage_diagnostics);

        // T019: Record scheduling opportunity for fairness probe after each child exit.
        self.fairness_probe.record_opportunity(&child_id);

        // T019: Periodically check fairness probe for scheduling starvation.
        self.check_fairness_probe(event_sender);

        // T029: Reflect group_fuse_active state when meltdown triggers group-level fuse.
        if let Some(ref budget_eval) = pipeline_result.budget_evaluation
            && matches!(
                budget_eval.meltdown_outcome,
                crate::policy::meltdown::MeltdownOutcome::GroupFuse
            )
            && let Some(ref group_id) = pipeline_result.group_id
        {
            let _ignored = event_sender.send(format!("group_fuse_active:{group_id}:{}", child_id));
            // Mark all children in the affected group as non-restartable.
            for (_cid, slot) in self.slots.iter_mut() {
                if slot.path.to_string().contains(group_id) {
                    slot.last_control_failure = Some(ChildControlFailure::new(
                        ChildControlFailurePhase::WaitCompletion,
                        format!("group_fuse_active:{group_id}"),
                        false,
                    ));
                }
            }
        }

        if let Some(pending) = fence_pending_release {
            for event in pending_events {
                self.emit_pending_event(event);
            }
            self.spawn_pending_restart_target(child_id.clone(), pending, exit_kind.clone());
            self.reconcile_stop_deadlines();
            return;
        }

        if !self.should_apply_automatic_policy(&child_id) {
            if self
                .slots
                .get(&child_id)
                .is_some_and(|state| state.operation == ChildControlOperation::Removed)
                && let Some(removed) = self.slots.remove(&child_id)
            {
                pending_events.push(PendingRuntimeEvent {
                    child_id: child_id.clone(),
                    path: removed.path.clone(),
                    generation: Some(generation),
                    attempt: Some(attempt),
                    correlation_id: CorrelationId::new(),
                    what: What::ChildRuntimeStateRemoved {
                        child_id: child_id.clone(),
                        path: removed.path,
                        final_status: Some(ChildAttemptStatus::Stopped),
                    },
                });
            }
            for event in pending_events {
                self.emit_pending_event(event);
            }
            self.reconcile_stop_deadlines();
            return;
        }

        // Extract action decision from pipeline result.
        let action_decision = pipeline_result.action_decision.as_ref();

        // Map pipeline protection action to restart decision.
        let pipeline_driven_decision = if let Some(decision) = action_decision {
            match decision.action {
                ProtectionAction::RestartAllowed => {
                    if role_policy_restarts_success(&pipeline_result) {
                        Some(RestartDecision::RestartAfter {
                            delay: Duration::ZERO,
                        })
                    } else {
                        self.restart_decision(&child_id)
                    }
                }
                ProtectionAction::RestartQueued => {
                    // Queue the restart - for now treat as no immediate restart.
                    None
                }
                ProtectionAction::RestartDenied
                | ProtectionAction::SupervisionPaused
                | ProtectionAction::Escalated
                | ProtectionAction::SupervisedStop => {
                    // Do not restart - respect pipeline decision.
                    None
                }
            }
        } else {
            // Fallback to existing policy engine if pipeline didn't produce a decision.
            self.restart_decision(&child_id)
        };

        let Some(decision) = pipeline_driven_decision else {
            for event in pending_events {
                self.emit_pending_event(event);
            }
            self.reconcile_stop_deadlines();
            return;
        };

        if restart_limit_refreshed
            .as_ref()
            .is_some_and(|(_path, restart_limit)| {
                restart_limit.used > restart_limit.limit
                    && matches!(decision, RestartDecision::RestartAfter { .. })
            })
        {
            let _ignored = event_sender.send(format!("child_restart_limit_exhausted:{child_id}"));
            for event in pending_events {
                self.emit_pending_event(event);
            }
            self.reconcile_stop_deadlines();
            return;
        }
        self.execute_restart_decision(child_id, decision, event_sender);
        for event in pending_events {
            self.emit_pending_event(event);
        }
        self.reconcile_stop_deadlines();
    }

    /// Records a failed child start.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Child identifier whose child_start_count failed.
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

        let mut fenced_spawn_recovery = Option::<(Generation, ChildStartCount, u64)>::None;
        let mut repaired_fenced_spawn = false;

        if let Some(runtime_state) = self.slots.get_mut(&child_id)
            && runtime_state.generation_fence.phase == GenerationFencePhase::ReadyToStart
        {
            fenced_spawn_recovery = runtime_state
                .registry_identity_anchor_for_spawn_attempt
                .take();
            repaired_fenced_spawn = true;
            runtime_state.generation_fence.phase = GenerationFencePhase::Open;
            runtime_state.last_control_failure = Some(ChildControlFailure::new(
                ChildControlFailurePhase::WaitCompletion,
                message,
                true,
            ));
        }

        if repaired_fenced_spawn {
            if let Some((generation, attempt, restart_count)) = fenced_spawn_recovery
                && let Some(registry_runtime) = self.registry.child_mut(&child_id)
            {
                registry_runtime.generation = generation;
                registry_runtime.child_start_count = attempt;
                registry_runtime.restart_count = restart_count;
            }
            return;
        }

        let _result = self.set_child_state(child_id, ChildControlOperation::Quarantined);
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

    /// Delivers cancellation to every active child child_start_count.
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
            let Some(runtime_state) = self.slots.get_mut(child_id) else {
                continue;
            };
            if runtime_state.operation == ChildControlOperation::Removed {
                continue;
            }
            if !runtime_state.has_active_attempt() {
                continue;
            };
            runtime_state.cancel();
            let _ignored = event_sender.send(format!(
                "child_shutdown_cancel_delivered:{}:{}:{}",
                runtime_state.child_id,
                runtime_state
                    .generation
                    .map_or(0, |generation| generation.value),
                runtime_state.attempt.map_or(0, |attempt| attempt.value)
            ));
        }
    }

    /// Drains cooperative child start_counts within the graceful timeout budget.
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
            let Some(mut runtime_state) = self.slots.remove(child_id) else {
                continue;
            };
            if runtime_state.operation == ChildControlOperation::Removed {
                outcomes.insert(
                    child_id.clone(),
                    removed_runtime_state_shutdown_outcome(
                        &runtime_state,
                        ShutdownPhase::GracefulDrain,
                    ),
                );
                self.slots.insert(child_id.clone(), runtime_state);
                continue;
            }
            if !runtime_state.has_active_attempt() {
                self.slots.insert(child_id.clone(), runtime_state);
                continue;
            };
            let completed = match remaining_duration(deadline) {
                Some(remaining) => timeout(remaining, runtime_state.wait_for_report())
                    .await
                    .ok(),
                None => None,
            };
            match completed {
                Some(Ok(report)) => {
                    let outcome = outcome_from_report(
                        &runtime_state,
                        &report,
                        ChildShutdownStatus::Graceful,
                        ShutdownPhase::GracefulDrain,
                        "child completed during graceful drain",
                    );
                    self.record_child_exit(report);
                    runtime_state.clear_instance();
                    self.slots.insert(child_id.clone(), runtime_state);
                    let _ignored = event_sender.send(format!("child_shutdown_graceful:{child_id}"));
                    outcomes.insert(child_id.clone(), outcome);
                }
                Some(Err(error)) => {
                    outcomes.insert(
                        child_id.clone(),
                        outcome_from_error(
                            &runtime_state,
                            ChildShutdownStatus::Graceful,
                            ShutdownPhase::GracefulDrain,
                            error,
                        ),
                    );
                    self.slots.insert(child_id.clone(), runtime_state);
                }
                None => {
                    self.slots.insert(child_id.clone(), runtime_state);
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
            let Some(mut runtime_state) = self.slots.remove(child_id) else {
                continue;
            };
            if runtime_state.operation == ChildControlOperation::Removed {
                outcomes.insert(
                    child_id.clone(),
                    removed_runtime_state_shutdown_outcome(
                        &runtime_state,
                        ShutdownPhase::AbortStragglers,
                    ),
                );
                self.slots.insert(child_id.clone(), runtime_state);
                continue;
            }
            if !runtime_state.has_active_attempt() {
                self.slots.insert(child_id.clone(), runtime_state);
                continue;
            };
            if !policy.abort_after_timeout {
                self.wait_for_late_report(
                    child_id,
                    runtime_state,
                    policy.abort_wait,
                    outcomes,
                    event_sender,
                )
                .await;
                continue;
            }
            runtime_state.abort();
            let _ignored = event_sender.send(format!(
                "child_shutdown_abort_requested:{}",
                runtime_state.child_id
            ));
            match timeout(policy.abort_wait, runtime_state.wait_for_report()).await {
                Ok(Ok(report)) => {
                    let outcome = outcome_from_report(
                        &runtime_state,
                        &report,
                        ChildShutdownStatus::Aborted,
                        ShutdownPhase::AbortStragglers,
                        "child completed after abort request",
                    );
                    self.record_child_exit(report);
                    runtime_state.clear_instance();
                    self.slots.insert(child_id.clone(), runtime_state);
                    let _ignored = event_sender.send(format!("child_shutdown_aborted:{child_id}"));
                    outcomes.insert(child_id.clone(), outcome);
                }
                Ok(Err(error)) => {
                    outcomes.insert(
                        child_id.clone(),
                        outcome_from_error(
                            &runtime_state,
                            ChildShutdownStatus::AbortFailed,
                            ShutdownPhase::AbortStragglers,
                            error,
                        ),
                    );
                    self.slots.insert(child_id.clone(), runtime_state);
                }
                Err(_elapsed) => {
                    outcomes.insert(
                        child_id.clone(),
                        ChildShutdownOutcome::new(ChildShutdownOutcomeInput {
                            child_id: runtime_state.child_id.clone(),
                            path: runtime_state.path.clone(),
                            generation: runtime_state
                                .generation
                                .unwrap_or_else(Generation::initial),
                            child_start_count: runtime_state
                                .attempt
                                .unwrap_or_else(ChildStartCount::first),
                            status: ChildShutdownStatus::AbortFailed,
                            cancel_delivered: runtime_state.attempt_cancel_delivered,
                            exit: None,
                            phase: ShutdownPhase::AbortStragglers,
                            reason: "child did not complete after abort request".to_owned(),
                        }),
                    );
                    self.slots.insert(child_id.clone(), runtime_state);
                }
            }
        }
    }

    /// Waits for a late report when abort is disabled by policy.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Child whose child_start_count is being reconciled.
    /// - `runtime_state`: Runtime state removed from runtime tracking.
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
        mut runtime_state: ChildSlot,
        wait: Duration,
        outcomes: &mut HashMap<ChildId, ChildShutdownOutcome>,
        event_sender: &broadcast::Sender<String>,
    ) {
        match timeout(wait, runtime_state.wait_for_report()).await {
            Ok(Ok(report)) => {
                let outcome = outcome_from_report(
                    &runtime_state,
                    &report,
                    ChildShutdownStatus::LateReport,
                    ShutdownPhase::AbortStragglers,
                    "child reported after graceful timeout",
                );
                self.record_child_exit(report);
                runtime_state.clear_instance();
                self.slots.insert(child_id.clone(), runtime_state);
                let _ignored = event_sender.send(format!("child_shutdown_late_report:{child_id}"));
                outcomes.insert(child_id.clone(), outcome);
            }
            Ok(Err(error)) => {
                outcomes.insert(
                    child_id.clone(),
                    outcome_from_error(
                        &runtime_state,
                        ChildShutdownStatus::LateReport,
                        ShutdownPhase::AbortStragglers,
                        error,
                    ),
                );
                self.slots.insert(child_id.clone(), runtime_state);
            }
            Err(_elapsed) => {
                outcomes.insert(
                    child_id.clone(),
                    ChildShutdownOutcome::new(ChildShutdownOutcomeInput {
                        child_id: runtime_state.child_id.clone(),
                        path: runtime_state.path.clone(),
                        generation: runtime_state.generation.unwrap_or_else(Generation::initial),
                        child_start_count: runtime_state
                            .attempt
                            .unwrap_or_else(ChildStartCount::first),
                        status: ChildShutdownStatus::AbortFailed,
                        cancel_delivered: runtime_state.attempt_cancel_delivered,
                        exit: None,
                        phase: ShutdownPhase::AbortStragglers,
                        reason: "abort disabled and child did not report before reconcile"
                            .to_owned(),
                    }),
                );
                self.slots.insert(child_id.clone(), runtime_state);
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
                    child_start_count: runtime.child_start_count,
                    status: ChildShutdownStatus::AlreadyExited,
                    cancel_delivered: false,
                    exit: runtime.last_exit.clone(),
                    phase: ShutdownPhase::Reconcile,
                    reason: "child had no active child_start_count during shutdown".to_owned(),
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
    /// Returns a [`CommandResult::ChildControl`] value.
    fn set_child_state(
        &mut self,
        child_id: ChildId,
        operation: ChildControlOperation,
    ) -> CommandResult {
        if !self.slots.contains_key(&child_id) {
            let placeholder = self
                .registry
                .child(&child_id)
                .map(|runtime| ChildSlot::new_placeholder(runtime.id.clone(), runtime.path.clone()))
                .unwrap_or_else(|| {
                    ChildSlot::new_placeholder(
                        child_id.clone(),
                        crate::id::types::SupervisorPath::root().join(child_id.value.clone()),
                    )
                });
            self.slots.insert(child_id.clone(), placeholder);
        }
        let runtime_state = self
            .slots
            .get_mut(&child_id)
            .expect("child runtime state should exist after insertion");
        let operation_before = runtime_state.operation;
        runtime_state.operation = operation;
        let outcome = ChildControlResult::new(
            child_id,
            runtime_state.attempt,
            runtime_state.generation,
            operation_before,
            runtime_state.operation,
            Some(runtime_state.status),
            false,
            if runtime_state.has_active_attempt() {
                runtime_state.stop_state
            } else {
                ChildStopState::NoActiveAttempt
            },
            runtime_state.restart_limit.clone(),
            runtime_state.observe_liveness(self.time_base.now_unix_nanos()),
            operation_before == operation,
            runtime_state.last_control_failure.clone(),
            None,
        );
        CommandResult::ChildControl { outcome }
    }

    /// Executes a stop-style child control command.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Target child identifier.
    /// - `target_operation`: Operation requested by the command.
    /// - `command_name`: Stable command name used in lifecycle text.
    /// - `meta`: Audit metadata attached to the command.
    /// - `event_sender`: Event channel used for lifecycle text.
    ///
    /// # Returns
    ///
    /// Returns a [`CommandResult::ChildControl`] value.
    fn execute_stop_child_control(
        &mut self,
        child_id: ChildId,
        target_operation: ChildControlOperation,
        command_name: &'static str,
        meta: &CommandMeta,
        event_sender: &broadcast::Sender<String>,
    ) -> CommandResult {
        if !self.slots.contains_key(&child_id) {
            let placeholder = self
                .registry
                .child(&child_id)
                .map(|runtime| ChildSlot::new_placeholder(runtime.id.clone(), runtime.path.clone()))
                .unwrap_or_else(|| {
                    ChildSlot::new_placeholder(
                        child_id.clone(),
                        crate::id::types::SupervisorPath::root().join(child_id.value.clone()),
                    )
                });
            self.slots.insert(child_id.clone(), placeholder);
        }

        let remove_after_outcome;
        let correlation_id = CorrelationId::from_uuid(meta.command_id.value);
        let mut pending_events = Vec::new();
        let outcome = {
            let runtime_state = self
                .slots
                .get_mut(&child_id)
                .expect("child runtime state should exist after insertion");
            let stop = apply_stop_control_to_runtime_state(
                runtime_state,
                target_operation,
                command_name,
                &meta.command_id.value.to_string(),
                correlation_id,
                self.time_base
                    .now_unix_nanos()
                    .saturating_add(self.shutdown.policy.graceful_timeout.as_nanos()),
                event_sender,
                &mut pending_events,
            );
            remove_after_outcome = stop.remove_after_outcome;
            build_child_control_outcome(
                stop.operation_before,
                stop.cancel_delivered,
                stop.idempotent,
                runtime_state.last_control_failure.clone(),
                runtime_state,
                &self.time_base,
                None,
            )
        };

        for event in pending_events {
            self.emit_pending_event(event);
        }

        let outcome_path = self
            .slots
            .get(&child_id)
            .map(|state| state.path.clone())
            .or_else(|| {
                self.registry
                    .child(&child_id)
                    .map(|runtime| runtime.path.clone())
            })
            .unwrap_or_else(|| SupervisorPath::root().join(child_id.value.clone()));
        self.emit_pending_event(PendingRuntimeEvent {
            child_id: child_id.clone(),
            path: outcome_path,
            generation: outcome.generation,
            attempt: outcome.attempt,
            correlation_id,
            what: What::ChildControlCommandCompleted {
                child_id: child_id.clone(),
                command: command_name.to_owned(),
                command_id: meta.command_id.value.to_string(),
                requested_by: meta.requested_by.clone(),
                reason: meta.reason.clone(),
                result: child_control_result_label(&outcome).to_owned(),
                outcome: Box::new(outcome.clone()),
            },
        });

        if remove_after_outcome {
            if let Some(removed) = self.slots.remove(&child_id) {
                self.emit_pending_event(PendingRuntimeEvent {
                    child_id: child_id.clone(),
                    path: removed.path.clone(),
                    generation: removed.generation,
                    attempt: removed.attempt,
                    correlation_id,
                    what: What::ChildRuntimeStateRemoved {
                        child_id: child_id.clone(),
                        path: removed.path,
                        final_status: None,
                    },
                });
            }
            let _ignored = event_sender.send(format!("child_runtime_state_removed:{child_id}"));
        }

        CommandResult::ChildControl { outcome }
    }

    /// Records the completed child_start_count in the registry.
    ///
    /// # Arguments
    ///
    /// - `report`: Completed child child_start_count report.
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
            runtime.child_start_count = report.runtime.child_start_count;
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
            self.slots.get(child_id).map(|state| state.operation),
            Some(
                ChildControlOperation::Paused
                    | ChildControlOperation::Quarantined
                    | ChildControlOperation::Removed
            )
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
            runtime.child_start_count.value,
            &backoff,
        ))
    }

    /// Refreshes restart limit state for one child after an exit.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Child whose accounting should be refreshed.
    /// - `count_failure`: Whether this exit consumes the restart limit.
    ///
    /// # Returns
    ///
    /// Returns the child path and updated restart limit state when the child is tracked.
    fn refresh_restart_limit_for_child(
        &mut self,
        child_id: &ChildId,
        count_failure: bool,
    ) -> Option<(SupervisorPath, crate::control::outcome::RestartLimitState)> {
        let restart_limit = restart_limit_for_child_in_spec(&self.tree, &self.spec, child_id);
        let runtime_state = self.slots.get_mut(child_id)?;
        let updated = runtime_state.refresh_restart_limit(
            restart_limit.window,
            restart_limit.max_restarts,
            count_failure,
            &self.time_base,
        );
        Some((runtime_state.path.clone(), updated))
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
                let _result =
                    self.set_child_state(failed_child, ChildControlOperation::Quarantined);
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

        // FR-003: Check concurrent restart gate before spawning
        if !self.concurrent_gate.try_acquire() {
            // Gate saturated - emit throttle event and skip restart
            let _ignored = event_sender.send(format!(
                "restart_throttled:concurrent_gate_saturated:{group_label}:{scope_label}"
            ));
            self.emit_throttle_gate_event(
                &failed_child,
                plan.group.as_deref(),
                ThrottleGateOwner::SupervisorInstance,
            );
            return;
        }

        let _ignored = event_sender.send(format!(
            "restart_plan:{:?}:{group_label}:{scope_label}",
            plan.strategy
        ));
        for child_id in plan.scope {
            self.spawn_child_start(child_id, true, delay);
        }
        self.concurrent_gate.release();
    }

    /// Emits a typed event for a restart throttle gate hit.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Child whose restart was throttled.
    /// - `group_id`: Optional restart execution group.
    /// - `owner`: Gate owner that limited the restart.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    fn emit_throttle_gate_event(
        &mut self,
        child_id: &ChildId,
        group_id: Option<&str>,
        owner: ThrottleGateOwner,
    ) {
        let now = Instant::now();
        let uptime = now
            .duration_since(self.time_base.base_instant)
            .as_millis()
            .min(u128::from(u64::MAX)) as u64;
        let monotonic_nanos = now.duration_since(self.time_base.base_instant).as_nanos();
        let path = self
            .slots
            .get(child_id)
            .map(|state| state.path.clone())
            .unwrap_or_else(|| SupervisorPath::root().join(child_id.value.clone()));
        let child_name = self
            .registry
            .child(child_id)
            .map(|runtime| runtime.spec.name.clone())
            .unwrap_or_else(|| child_id.to_string());
        let mut event = SupervisorEvent::new(
            When::new(EventTime::from_parts(
                monotonic_nanos,
                uptime,
                Generation::initial(),
                ChildStartCount::first(),
            )),
            Where::new(path).with_child(child_id.clone(), child_name),
            What::ChildFailed {
                failure: crate::error::types::TaskFailure::new(
                    crate::error::types::TaskFailureKind::Error,
                    "restart_throttled",
                    format!(
                        "restart denied by throttle gate {} for group {}",
                        owner,
                        group_id.unwrap_or("supervisor")
                    ),
                ),
            },
            self.event_sequences.next(),
            CorrelationId::new(),
            1,
        );
        event.effective_protective_action = Some(ProtectionAction::RestartDenied);
        event.throttle_gate_owner = owner;
        if let Some(runtime) = self.registry.child(child_id) {
            let effective_policy = prepare_effective_policy(&runtime.spec);
            event.work_role = Some(effective_policy.work_role);
            event.used_fallback_default = effective_policy.used_fallback;
            event.effective_policy_source = Some(effective_policy.source);
        }
        if let Ok(mut observability) = self.observability.lock() {
            let _lagged = observability.emit(event);
        }
    }

    /// Records six-stage pipeline diagnostics in shared observability.
    ///
    /// # Arguments
    ///
    /// - `diagnostics`: Diagnostics produced by the supervision pipeline.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    fn record_pipeline_stage_diagnostics(&self, diagnostics: &[PipelineStageDiagnostic]) {
        if let Ok(mut observability) = self.observability.lock() {
            observability.record_pipeline_stage_diagnostics(diagnostics);
        }
    }

    /// Checks the fairness probe and emits starvation alerts (T019).
    ///
    /// # Arguments
    ///
    /// - `event_sender`: Event channel used for lifecycle text.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    fn check_fairness_probe(&mut self, event_sender: &broadcast::Sender<String>) {
        let now_unix_nanos = self.time_base.now_unix_nanos();
        let all_child_ids: Vec<ChildId> = self.slots.keys().cloned().collect();
        if let Some(alert) = self.fairness_probe.check(now_unix_nanos, &all_child_ids) {
            // Emit typed event for structured observability (ALIGN-003).
            let path = self
                .slots
                .get(&alert.starved_child_id)
                .map(|slot| slot.path.clone())
                .unwrap_or_else(|| {
                    SupervisorPath::root().join(alert.starved_child_id.value.clone())
                });
            let generation = self
                .slots
                .get(&alert.starved_child_id)
                .and_then(|slot| slot.generation);
            let attempt = self
                .slots
                .get(&alert.starved_child_id)
                .and_then(|slot| slot.attempt);
            let pending = PendingRuntimeEvent {
                child_id: alert.starved_child_id.clone(),
                path,
                generation,
                attempt,
                correlation_id: CorrelationId::new(),
                what: What::FairnessProbeStarvation {
                    starved_child_id: alert.starved_child_id.clone(),
                    skip_count: alert.skip_count,
                    probe_start_unix_nanos: alert.probe_start_unix_nanos,
                    probe_end_unix_nanos: alert.probe_end_unix_nanos,
                },
            };
            self.emit_pending_event(pending);

            // Keep text-based log for backward compatibility.
            let _ignored = event_sender.send(format!(
                "fairness_starvation:{}:skip_count={}:window_start={}:window_end={}",
                alert.starved_child_id,
                alert.skip_count,
                alert.probe_start_unix_nanos,
                alert.probe_end_unix_nanos,
            ));
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

    /// Handles `RestartChild` with generation fencing semantics.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Stable child targeted by restart.
    /// - `meta`: Audit metadata forwarded from the caller.
    /// - `event_sender`: Lifecycle text broadcaster.
    ///
    /// # Returns
    ///
    /// Returns a structured [`CommandResult`] that always uses [`CommandResult::ChildControl`].
    fn execute_restart_child_control(
        &mut self,
        child_id: ChildId,
        meta: &CommandMeta,
        event_sender: &broadcast::Sender<String>,
    ) -> CommandResult {
        let correlation_id = CorrelationId::from_uuid(meta.command_id.value);

        if self.registry.child(&child_id).is_none() {
            let outcome = restart_child_unknown_outcome(child_id.clone());
            self.emit_restart_child_completed(
                outcome.clone(),
                meta,
                correlation_id,
                event_sender,
                Vec::new(),
            );
            return CommandResult::ChildControl { outcome };
        }

        if self.shutdown.phase() != ShutdownPhase::Idle {
            return self.restart_child_blocked_by_shutdown(
                &child_id,
                meta,
                correlation_id,
                event_sender,
            );
        }

        if !self.slots.contains_key(&child_id) {
            let placeholder = self
                .registry
                .child(&child_id)
                .map(|runtime| ChildSlot::new_placeholder(runtime.id.clone(), runtime.path.clone()))
                .unwrap_or_else(|| {
                    ChildSlot::new_placeholder(
                        child_id.clone(),
                        SupervisorPath::root().join(child_id.value.clone()),
                    )
                });
            self.slots.insert(child_id.clone(), placeholder);
        }

        let mut pending_events = Vec::new();

        // Records which restart branch matched before optional immediate spawn bookkeeping.
        enum RestartPrep {
            // Outcome resolved without visiting `spawn_child_start`.
            Completed(Box<ChildControlResult>),
            // Child had no activity and should restart immediately via the shared spawn helper.
            DeferredImmediate {
                // Operation captured before spawning.
                operation_before: ChildControlOperation,
            },
        }

        let restart_prep = {
            let runtime_state = self
                .slots
                .get_mut(&child_id)
                .expect("runtime state exists after insertion");
            if runtime_state.generation_fence.pending_restart.is_some() {
                let pending = runtime_state
                    .generation_fence
                    .pending_restart
                    .as_mut()
                    .expect("checked pending restart");
                pending.duplicate_request_count = pending.duplicate_request_count.saturating_add(1);
                let pending_for_conflict = pending.clone();
                pending_events.push(PendingRuntimeEvent {
                    child_id: child_id.clone(),
                    path: runtime_state.path.clone(),
                    generation: Some(pending_for_conflict.old_generation),
                    attempt: Some(pending_for_conflict.old_attempt),
                    correlation_id: CorrelationId::from_uuid(meta.command_id.value),
                    what: What::ChildRestartConflict {
                        child_id: child_id.clone(),
                        current_generation: Some(pending_for_conflict.old_generation),
                        current_attempt: Some(pending_for_conflict.old_attempt),
                        target_generation: Some(pending_for_conflict.target_generation),
                        command_id: meta.command_id.value.to_string(),
                        decision: "already_pending".to_owned(),
                        reason: "duplicate restart merged into pending restart".to_owned(),
                    },
                });
                let fence = GenerationFenceOutcome::new(
                    GenerationFenceDecision::AlreadyPending,
                    Some(pending_for_conflict.old_generation),
                    Some(pending_for_conflict.old_attempt),
                    Some(pending_for_conflict.target_generation),
                    false,
                    pending_for_conflict.abort_requested,
                    None,
                );
                let operation_before = runtime_state.operation;
                RestartPrep::Completed(Box::new(build_child_control_outcome(
                    operation_before,
                    false,
                    false,
                    runtime_state.last_control_failure.clone(),
                    runtime_state,
                    &self.time_base,
                    Some(fence),
                )))
            } else if !runtime_state.has_active_attempt() {
                RestartPrep::DeferredImmediate {
                    operation_before: runtime_state.operation,
                }
            } else {
                let old_generation = runtime_state
                    .generation
                    .expect("active attempt owns a generation");
                let old_attempt = runtime_state
                    .attempt
                    .expect("active attempt owns an attempt counter");
                let cancel_delivered = runtime_state.cancel();
                let deadline = self
                    .time_base
                    .now_unix_nanos()
                    .saturating_add(self.shutdown.policy.graceful_timeout.as_nanos());
                runtime_state.stop_deadline_at_unix_nanos = Some(deadline);
                let target_generation = old_generation.next();
                let requested_at = self.time_base.now_unix_nanos();
                let pending = PendingRestart::new(
                    meta.command_id.value,
                    meta.requested_by.clone(),
                    meta.reason.clone(),
                    old_generation,
                    old_attempt,
                    target_generation,
                    requested_at,
                    deadline,
                    false,
                    0,
                );
                runtime_state.generation_fence.pending_restart = Some(pending.clone());
                runtime_state.generation_fence.phase = GenerationFencePhase::WaitingForOldStop;

                if cancel_delivered {
                    pending_events.push(PendingRuntimeEvent {
                        child_id: child_id.clone(),
                        path: runtime_state.path.clone(),
                        generation: Some(old_generation),
                        attempt: Some(old_attempt),
                        correlation_id,
                        what: What::ChildControlCancelDelivered {
                            child_id: child_id.clone(),
                            generation: old_generation,
                            attempt: old_attempt,
                            command: "restart_child".to_owned(),
                            command_id: meta.command_id.value.to_string(),
                        },
                    });
                    let _ignored = event_sender.send(format!(
                        "child_control_cancel_delivered:{child_id}:restart_child"
                    ));
                }

                pending_events.push(PendingRuntimeEvent {
                    child_id: child_id.clone(),
                    path: runtime_state.path.clone(),
                    generation: Some(old_generation),
                    attempt: Some(old_attempt),
                    correlation_id,
                    what: What::ChildRestartFenceEntered {
                        child_id: child_id.clone(),
                        old_generation,
                        old_attempt,
                        target_generation,
                        command_id: meta.command_id.value.to_string(),
                        requested_by: meta.requested_by.clone(),
                        reason: meta.reason.clone(),
                        stop_deadline_at_unix_nanos: deadline,
                    },
                });

                let operation_before = runtime_state.operation;
                let fence = GenerationFenceOutcome::new(
                    GenerationFenceDecision::QueuedAfterStop,
                    Some(old_generation),
                    Some(old_attempt),
                    Some(target_generation),
                    cancel_delivered,
                    false,
                    None,
                );
                RestartPrep::Completed(Box::new(build_child_control_outcome(
                    operation_before,
                    cancel_delivered,
                    false,
                    None,
                    runtime_state,
                    &self.time_base,
                    Some(fence),
                )))
            }
        };

        let outcome = match restart_prep {
            RestartPrep::Completed(outcome) => *outcome,
            RestartPrep::DeferredImmediate { operation_before } => {
                self.spawn_child_start(child_id.clone(), true, Duration::ZERO);
                let runtime_state = self.slots.get_mut(&child_id).expect("runtime state exists");
                let target_generation = self
                    .registry
                    .child(&child_id)
                    .map(|runtime| runtime.generation);
                let fence = GenerationFenceOutcome::new(
                    GenerationFenceDecision::StartedImmediately,
                    None,
                    None,
                    target_generation,
                    false,
                    false,
                    None,
                );
                build_child_control_outcome(
                    operation_before,
                    false,
                    false,
                    runtime_state.last_control_failure.clone(),
                    runtime_state,
                    &self.time_base,
                    Some(fence),
                )
            }
        };

        self.emit_restart_child_completed(
            outcome.clone(),
            meta,
            correlation_id,
            event_sender,
            pending_events,
        );

        CommandResult::ChildControl { outcome }
    }

    /// Emits [`What::ChildControlCommandCompleted`] for an explicit restart command.
    ///
    /// # Arguments
    ///
    /// - `outcome`: Command outcome returned to the caller.
    /// - `meta`: Audit metadata carried from the command.
    /// - `correlation_id`: Correlation shared with related fence events.
    /// - `event_sender`: Legacy text broadcaster.
    /// - `pending_events`: Fence or cancellation events that must publish first.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    fn emit_restart_child_completed(
        &mut self,
        outcome: ChildControlResult,
        meta: &CommandMeta,
        correlation_id: CorrelationId,
        event_sender: &broadcast::Sender<String>,
        mut pending_events: Vec<PendingRuntimeEvent>,
    ) {
        for event in pending_events.drain(..) {
            self.emit_pending_event(event);
        }
        let outcome_identifier = outcome.child_id.clone();
        let outcome_path = self
            .slots
            .get(&outcome.child_id)
            .map(|state| state.path.clone())
            .or_else(|| {
                self.registry
                    .child(&outcome.child_id)
                    .map(|runtime| runtime.path.clone())
            })
            .unwrap_or_else(|| SupervisorPath::root().join(outcome.child_id.value.clone()));
        self.emit_pending_event(PendingRuntimeEvent {
            child_id: outcome.child_id.clone(),
            path: outcome_path,
            generation: outcome.generation,
            attempt: outcome.attempt,
            correlation_id,
            what: What::ChildControlCommandCompleted {
                child_id: outcome.child_id.clone(),
                command: "restart_child".to_owned(),
                command_id: meta.command_id.value.to_string(),
                requested_by: meta.requested_by.clone(),
                reason: meta.reason.clone(),
                result: child_control_result_label(&outcome).to_owned(),
                outcome: Box::new(outcome),
            },
        });
        let _ignored = event_sender.send(format!(
            "child_control_command_completed:{}:restart_child",
            outcome_identifier
        ));
    }

    /// Blocks restart while the supervisor tree is not idle.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Target child identifier.
    /// - `meta`: Audit metadata from the command.
    /// - `correlation_id`: Correlation binding typed events.
    /// - `event_sender`: Legacy text broadcaster.
    ///
    /// # Returns
    ///
    /// Returns [`CommandResult::ChildControl`] with [`GenerationFenceDecision::BlockedByShutdown`].
    fn restart_child_blocked_by_shutdown(
        &mut self,
        child_id: &ChildId,
        meta: &CommandMeta,
        correlation_id: CorrelationId,
        event_sender: &broadcast::Sender<String>,
    ) -> CommandResult {
        if !self.slots.contains_key(child_id) {
            let placeholder = self
                .registry
                .child(child_id)
                .map(|runtime| ChildSlot::new_placeholder(runtime.id.clone(), runtime.path.clone()))
                .unwrap_or_else(|| {
                    ChildSlot::new_placeholder(
                        child_id.clone(),
                        SupervisorPath::root().join(child_id.value.clone()),
                    )
                });
            self.slots.insert(child_id.clone(), placeholder);
        }

        let outcome = {
            let runtime_state = self.slots.get_mut(child_id).expect("runtime state exists");
            runtime_state.generation_fence.phase = GenerationFencePhase::Closed;
            let failure = ChildControlFailure::new(
                ChildControlFailurePhase::WaitCompletion,
                "supervisor tree is shutting down",
                false,
            );
            let fence = GenerationFenceOutcome::new(
                GenerationFenceDecision::BlockedByShutdown,
                runtime_state.generation,
                runtime_state.attempt,
                None,
                false,
                false,
                Some(failure.clone()),
            );
            let operation_before = runtime_state.operation;
            runtime_state.last_control_failure = Some(failure);
            build_child_control_outcome(
                operation_before,
                false,
                false,
                runtime_state.last_control_failure.clone(),
                runtime_state,
                &self.time_base,
                Some(fence),
            )
        };

        let blocked_events = match self.slots.get(child_id).map(|runtime_state| {
            (
                runtime_state.path.clone(),
                runtime_state.generation,
                runtime_state.attempt,
            )
        }) {
            Some((path, current_generation, current_attempt)) => {
                vec![PendingRuntimeEvent {
                    child_id: child_id.clone(),
                    path,
                    generation: current_generation,
                    attempt: current_attempt,
                    correlation_id,
                    what: What::ChildRestartConflict {
                        child_id: child_id.clone(),
                        current_generation,
                        current_attempt,
                        target_generation: None,
                        command_id: meta.command_id.value.to_string(),
                        decision: "rejected".to_owned(),
                        reason: "restart rejected while supervisor tree is shutting down"
                            .to_owned(),
                    },
                }]
            }
            None => Vec::new(),
        };

        self.emit_restart_child_completed(
            outcome.clone(),
            meta,
            correlation_id,
            event_sender,
            blocked_events,
        );

        CommandResult::ChildControl { outcome }
    }

    /// Builds the current runtime state report.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a [`CurrentState`] value.
    fn build_current_state(&mut self) -> CurrentState {
        let mut child_runtime_records = Vec::new();
        let mut pending_events = Vec::new();
        let declaration_order = self.registry.declaration_order().to_vec();
        for child_id in declaration_order {
            if let Some(runtime_state) = self.slots.get_mut(&child_id) {
                let liveness = runtime_state.observe_liveness(self.time_base.now_unix_nanos());
                if let Some(event) = heartbeat_stale_event(runtime_state, &liveness) {
                    pending_events.push(event);
                }
                child_runtime_records.push(runtime_state.to_record(liveness));
            }
        }
        for (child_id, runtime_state) in &mut self.slots {
            if self.registry.child(child_id).is_some() {
                continue;
            }
            let liveness = runtime_state.observe_liveness(self.time_base.now_unix_nanos());
            if let Some(event) = heartbeat_stale_event(runtime_state, &liveness) {
                pending_events.push(event);
            }
            child_runtime_records.push(runtime_state.to_record(liveness));
        }
        for event in pending_events {
            self.emit_pending_event(event);
        }
        CurrentState {
            child_count: self.dynamic_child_count(),
            shutdown_completed: self.shutdown.phase()
                == crate::shutdown::stage::ShutdownPhase::Completed,
            child_runtime_records,
        }
    }

    /// Spawns the target generation queued by a pending manual restart once the old attempt exits.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Stable child undergoing a fenced restart.
    /// - `pending`: Accepted restart bookkeeping that pins the identity triple transition.
    /// - `old_exit`: Exit classification observed for the old attempt.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    fn spawn_pending_restart_target(
        &mut self,
        child_id: ChildId,
        pending: PendingRestart,
        old_exit: TaskExit,
    ) {
        let Some(registry_identity_anchor) = self.registry.child(&child_id).map(|runtime| {
            (
                runtime.generation,
                runtime.child_start_count,
                runtime.restart_count,
            )
        }) else {
            return;
        };
        let path = self
            .slots
            .get(&child_id)
            .map(|state| state.path.clone())
            .unwrap_or_else(|| SupervisorPath::root().join(child_id.value.clone()));
        let correlation_id = CorrelationId::from_uuid(pending.command_id);

        if let Some(runtime_state) = self.slots.get_mut(&child_id) {
            runtime_state.registry_identity_anchor_for_spawn_attempt =
                Some(registry_identity_anchor);
        }

        {
            let Some(registry_runtime) = self.registry.child_mut(&child_id) else {
                return;
            };
            registry_runtime.generation = pending.target_generation;
            registry_runtime.child_start_count = registry_runtime.child_start_count.next();
            registry_runtime.restart_count = registry_runtime.restart_count.saturating_add(1);
            registry_runtime.status = ChildRuntimeStatus::Starting;
        }

        let Some(runtime) = self.registry.child(&child_id).cloned() else {
            return;
        };
        let new_generation = runtime.generation;
        let new_attempt = runtime.child_start_count;

        let path_for_handles = path.clone();

        match ChildRunner::new().spawn_once(runtime) {
            Ok(handle) => {
                let sender = self.command_sender.clone();
                self.emit_pending_event(PendingRuntimeEvent {
                    child_id: child_id.clone(),
                    path,
                    generation: Some(new_generation),
                    attempt: Some(new_attempt),
                    correlation_id,
                    what: What::ChildRestartFenceReleased {
                        child_id: child_id.clone(),
                        old_generation: pending.old_generation,
                        old_attempt: pending.old_attempt,
                        target_generation: pending.target_generation,
                        exit_kind: old_exit.clone(),
                    },
                });
                let mut completion_receiver = handle.completion_receiver.clone();
                self.slots
                    .entry(child_id.clone())
                    .or_insert_with(|| {
                        ChildSlot::new_placeholder(child_id.clone(), path_for_handles)
                    })
                    .activate(
                        new_generation,
                        new_attempt,
                        ChildAttemptStatus::Running,
                        handle,
                    );
                tokio::spawn(async move {
                    let result = wait_for_report(&mut completion_receiver).await;
                    send_child_result(sender, child_id, result).await;
                });
            }
            Err(error) => {
                let message = error.to_string();
                if let Some(runtime_state) = self.slots.get_mut(&child_id) {
                    let identity_anchor_triple_opt = runtime_state
                        .registry_identity_anchor_for_spawn_attempt
                        .take();
                    if let Some((generation, attempt, restart_count)) = identity_anchor_triple_opt {
                        if let Some(registry_runtime) = self.registry.child_mut(&child_id) {
                            registry_runtime.generation = generation;
                            registry_runtime.child_start_count = attempt;
                            registry_runtime.restart_count = restart_count;
                        }
                        // Keep the superseded `(generation, attempt)` identity visible alongside the queued target spawn failure diagnostics.
                        runtime_state.generation = Some(generation);
                        runtime_state.attempt = Some(attempt);
                        runtime_state.status = ChildAttemptStatus::Stopped;
                    }
                    runtime_state.generation_fence.phase = GenerationFencePhase::Open;
                    runtime_state.last_control_failure = Some(ChildControlFailure::new(
                        ChildControlFailurePhase::WaitCompletion,
                        message,
                        true,
                    ));
                }
                // Avoid enqueueing a second asynchronous start-failure loop message because `last_control_failure` already records the deterministic spawn diagnostics.
            }
        }
    }

    /// Reconciles expired stop deadlines without blocking the control loop.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    fn reconcile_stop_deadlines(&mut self) {
        let now = self.time_base.now_unix_nanos();
        let mut pending_events = Vec::new();
        for runtime_state in self.slots.values_mut() {
            let fence_escalation = if let Some(pending_restart) =
                runtime_state.generation_fence.pending_restart.as_ref()
            {
                if pending_restart.abort_requested {
                    None
                } else if runtime_state.generation_fence.phase
                    == GenerationFencePhase::WaitingForOldStop
                    && runtime_state.stop_state == ChildStopState::CancelDelivered
                    && runtime_state.has_active_attempt()
                    && now >= pending_restart.stop_deadline_at_unix_nanos
                {
                    match (runtime_state.generation, runtime_state.attempt) {
                        (Some(old_generation), Some(old_attempt)) => Some((
                            pending_restart.command_id,
                            pending_restart.target_generation,
                            pending_restart.stop_deadline_at_unix_nanos,
                            runtime_state.child_id.clone(),
                            runtime_state.path.clone(),
                            old_generation,
                            old_attempt,
                        )),
                        _ => None,
                    }
                } else {
                    None
                }
            } else {
                None
            };

            if let Some((
                command_id,
                target_generation,
                deadline_ns,
                fence_child_id,
                fence_path,
                old_generation,
                old_attempt,
            )) = fence_escalation
            {
                let delivered = runtime_state.abort();
                if delivered {
                    if let Some(pending_mut) = &mut runtime_state.generation_fence.pending_restart {
                        pending_mut.abort_requested = true;
                    }
                    runtime_state.generation_fence.phase = GenerationFencePhase::AbortingOld;
                    pending_events.push(PendingRuntimeEvent {
                        child_id: fence_child_id.clone(),
                        path: fence_path,
                        generation: Some(old_generation),
                        attempt: Some(old_attempt),
                        correlation_id: CorrelationId::from_uuid(command_id),
                        what: What::ChildRestartFenceAbortRequested {
                            child_id: fence_child_id,
                            old_generation,
                            old_attempt,
                            target_generation,
                            command_id: command_id.to_string(),
                            deadline_unix_nanos: deadline_ns,
                        },
                    });
                }
            }

            if runtime_state.generation_fence.pending_restart.is_some() {
                continue;
            }

            if matches!(
                runtime_state.generation_fence.phase,
                GenerationFencePhase::WaitingForOldStop | GenerationFencePhase::AbortingOld
            ) {
                continue;
            }

            if runtime_state.stop_state != ChildStopState::CancelDelivered {
                continue;
            }
            let Some(deadline) = runtime_state.stop_deadline_at_unix_nanos else {
                continue;
            };
            if deadline > now || !runtime_state.has_active_attempt() {
                continue;
            }
            let Some(generation) = runtime_state.generation else {
                continue;
            };
            let Some(attempt) = runtime_state.attempt else {
                continue;
            };
            let status = runtime_state.status;
            let failure = ChildControlFailure::new(
                ChildControlFailurePhase::WaitCompletion,
                "child did not complete before stop deadline",
                true,
            );
            runtime_state.status = status;
            runtime_state.stop_state = ChildStopState::Failed;
            runtime_state.last_control_failure = Some(failure.clone());
            pending_events.push(PendingRuntimeEvent {
                child_id: runtime_state.child_id.clone(),
                path: runtime_state.path.clone(),
                generation: Some(generation),
                attempt: Some(attempt),
                correlation_id: CorrelationId::new(),
                what: What::ChildControlStopFailed {
                    child_id: runtime_state.child_id.clone(),
                    generation,
                    attempt,
                    status,
                    stop_state: ChildStopState::Failed,
                    phase: failure.phase,
                    reason: failure.reason,
                    recoverable: failure.recoverable,
                },
            });
        }
        for event in pending_events {
            self.emit_pending_event(event);
        }
    }

    /// Emits one pending typed runtime event.
    ///
    /// # Arguments
    ///
    /// - `pending`: Event data collected while runtime state was borrowed.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    fn emit_pending_event(&mut self, pending: PendingRuntimeEvent) {
        let now = Instant::now();
        let uptime = now
            .duration_since(self.time_base.base_instant)
            .as_millis()
            .min(u128::from(u64::MAX)) as u64;
        let monotonic_nanos = now.duration_since(self.time_base.base_instant).as_nanos();
        let child_name = self
            .registry
            .child(&pending.child_id)
            .map(|runtime| runtime.spec.name.clone())
            .unwrap_or_else(|| pending.child_id.to_string());
        let event = SupervisorEvent::new(
            When::new(EventTime::from_parts(
                monotonic_nanos,
                uptime,
                pending.generation.unwrap_or_else(Generation::initial),
                pending.attempt.unwrap_or_else(ChildStartCount::first),
            )),
            Where::new(pending.path).with_child(pending.child_id, child_name),
            pending.what,
            self.event_sequences.next(),
            pending.correlation_id,
            1,
        );
        if let Ok(mut observability) = self.observability.lock() {
            let _lagged = observability.emit(event);
        }
    }

    /// Spawns one child child_start_count and reports the exit back to this control loop.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Child that should run.
    /// - `is_restart`: Whether this child_start_count is a restart child_start_count.
    /// - `delay`: Delay before the child_start_count starts.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    fn spawn_child_start(&mut self, child_id: ChildId, is_restart: bool, delay: Duration) {
        if self.shutdown.phase() != ShutdownPhase::Idle {
            return;
        }
        if let Some(runtime_state) = self.slots.get(&child_id) {
            if runtime_state.generation_fence.pending_restart.is_some() {
                if is_restart {
                    let path = runtime_state.path.clone();
                    let generation = runtime_state.generation;
                    let attempt = runtime_state.attempt;
                    let pending_target = runtime_state
                        .generation_fence
                        .pending_restart
                        .as_ref()
                        .map(|pending| pending.target_generation);
                    self.emit_pending_event(PendingRuntimeEvent {
                        child_id: child_id.clone(),
                        path,
                        generation,
                        attempt,
                        correlation_id: CorrelationId::new(),
                        what: What::ChildRestartConflict {
                            child_id: child_id.clone(),
                            current_generation: generation,
                            current_attempt: attempt,
                            target_generation: pending_target,
                            command_id: "runtime-policy".to_owned(),
                            decision: "rejected".to_owned(),
                            reason: "automatic restart suppressed while pending manual restart holds the fence".to_owned(),
                        },
                    });
                }
                return;
            }
            if matches!(
                runtime_state.generation_fence.phase,
                GenerationFencePhase::WaitingForOldStop
                    | GenerationFencePhase::AbortingOld
                    | GenerationFencePhase::Closed
                    | GenerationFencePhase::ReadyToStart
            ) {
                return;
            }
        }
        let Some(runtime) = self.prepare_child_start(&child_id, is_restart) else {
            return;
        };
        let sender = self.command_sender.clone();
        if !delay.is_zero() {
            tokio::spawn(async move {
                tokio::time::sleep(delay).await;
                let child_id_for_msg = runtime.id.clone();
                let path = runtime.path.clone();
                let generation = runtime.generation;
                let attempt = runtime.child_start_count;
                match ChildRunner::new().spawn_once(runtime) {
                    Ok(handle) => {
                        let _ignored = sender
                            .send(RuntimeLoopMessage::ChildStart(
                                ChildStartMessage::DelayedSpawnAttached {
                                    child_id: child_id_for_msg,
                                    path,
                                    generation,
                                    attempt,
                                    handle,
                                },
                            ))
                            .await;
                    }
                    Err(error) => {
                        tokio::spawn(async move {
                            send_child_result(sender, child_id_for_msg, Err(error)).await;
                        });
                    }
                }
            });
            return;
        }

        let child_id_cloned = runtime.id.clone();
        let path = runtime.path.clone();
        let generation = runtime.generation;
        let child_start_count = runtime.child_start_count;
        match ChildRunner::new().spawn_once(runtime) {
            Ok(handle) => {
                self.attach_spawned_child_handle(
                    child_id_cloned,
                    path,
                    generation,
                    child_start_count,
                    handle,
                );
            }
            Err(error) => {
                tokio::spawn(async move {
                    send_child_result(sender, child_id_cloned, Err(error)).await;
                });
            }
        }
    }

    /// Prepares registry state for one child child_start_count.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Child that should run.
    /// - `bump_restart_counters`: Whether this spawn should bump generation accounting like a restart.
    ///
    /// # Returns
    ///
    /// Returns a runtime record for the child runner.
    fn prepare_child_start(
        &mut self,
        child_id: &ChildId,
        bump_restart_counters: bool,
    ) -> Option<ChildRuntime> {
        let runtime = self.registry.child_mut(child_id)?;
        if bump_restart_counters {
            runtime.child_start_count = runtime.child_start_count.next();
            runtime.generation = runtime.generation.next();
            runtime.restart_count = runtime.restart_count.saturating_add(1);
        }
        runtime.status = ChildRuntimeStatus::Starting;
        if let Some(runtime_state) = self.slots.get_mut(child_id) {
            runtime_state.operation = ChildControlOperation::Active;
        }
        Some(runtime.clone())
    }

    // ------------------------------------------------------------------
    // Slot-based lifecycle operations (migration from child_runtime_states)
    // ------------------------------------------------------------------

    /// Executes a shutdown on all slots using the real cancellation+join
    /// pipeline.
    ///
    /// # Arguments
    ///
    /// - `event_sender`: Event channel used for lifecycle text.
    ///
    /// # Returns
    ///
    /// Returns a shutdown result.
    pub(crate) async fn handle_shutdown_tree(
        &mut self,
        requested_by: String,
        reason: String,
        event_sender: &broadcast::Sender<String>,
    ) -> Result<ShutdownResult, SupervisorError> {
        let policy = self.shutdown.policy;
        let reason_copy = reason.clone();
        let cause = ShutdownCause::new(requested_by, reason);
        let _started = self.shutdown.request_stop(cause);
        let _ignored = event_sender.send(format!(
            "shutdown_phase_changed:{:?}:{:?}",
            ShutdownPhase::Idle,
            self.shutdown.phase()
        ));
        self.advance_shutdown_phase(event_sender);
        self.advance_shutdown_phase(event_sender);

        let outcomes =
            shutdown_tree_fanout(&mut self.slots, &policy, &mut self.admission_set).await;
        let reconcile = reconcile_shutdown_slots(&self.slots);

        // Emit orphan warning when residual handles remain after shutdown.
        if !reconcile.verified_clean {
            let _ignored = event_sender.send(format!(
                "shutdown_reconcile_warning: orphan_slots={:?}",
                reconcile.orphan_slots
            ));
        }

        self.advance_shutdown_phase(event_sender);
        self.advance_shutdown_phase(event_sender);
        let _completed = self.shutdown.complete();

        let report = ShutdownPipelineReport {
            cause: ShutdownCause::new("slot-shutdown", reason_copy),
            started_at_unix_nanos: unix_epoch_nanos(),
            completed_at_unix_nanos: unix_epoch_nanos(),
            phase: ShutdownPhase::Completed,
            outcomes,
            reconcile: ShutdownReconcileReport::core_runtime_completed(),
            idempotent: false,
        };
        self.shutdown_pipeline.cache_report(report.clone());
        let _ignored = event_sender.send(format!("shutdown_completed:{}", report.outcomes.len()));
        Ok(self.shutdown.result_with_report(report, false))
    }

    /// Applies a control operation to the slot for the given child.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Target child identifier.
    /// - `operation`: Desired control operation.
    ///
    /// # Returns
    ///
    /// Returns `true` when the slot was found and modified.
    pub(crate) fn handle_command_on_slot(
        &mut self,
        child_id: &ChildId,
        operation: ChildControlOperation,
    ) -> bool {
        let Some(slot) = self.slots.get_mut(child_id) else {
            return false;
        };
        slot.operation = operation;
        if matches!(
            operation,
            ChildControlOperation::Quarantined | ChildControlOperation::Removed
        ) && slot.has_active_attempt()
        {
            slot.cancel();
        }
        true
    }

    /// Processes a completed child exit through the slot system.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Child that exited.
    /// - `report`: Completed child run report.
    ///
    /// # Returns
    ///
    /// Returns the exit summary stored in the slot, or `None` when no slot
    /// exists for this child.
    pub(crate) fn process_child_exit_on_slot(
        &mut self,
        child_id: &ChildId,
        report: &ChildRunReport,
    ) -> Option<ChildExitSummary> {
        let slot = self.slots.get_mut(child_id)?;
        let now_nanos = self.time_base.now_unix_nanos();
        let summary = ChildExitSummary::from_report(report, now_nanos);
        slot.deactivate(summary.clone());
        self.admission_set.release(child_id);
        Some(summary)
    }

    /// Observes liveness for every active slot and emits stale heartbeat
    /// events.
    ///
    /// # Arguments
    ///
    /// - `event_sender`: Event channel used for lifecycle text.
    ///
    /// # Returns
    ///
    /// Returns the count of slots with stale heartbeats.
    pub(crate) fn observe_slot_liveness(
        &mut self,
        event_sender: &broadcast::Sender<String>,
    ) -> usize {
        let mut stale_count = 0usize;
        let threshold_nanos = Duration::from_secs(DEFAULT_HEARTBEAT_TIMEOUT_SECS).as_nanos();
        let now_nanos = self.time_base.now_unix_nanos();

        for (child_id, slot) in self.slots.iter_mut() {
            if !slot.has_active_attempt() {
                continue;
            }
            if let Some(last_hb) = slot.last_heartbeat_at
                && now_nanos.saturating_sub(last_hb) >= threshold_nanos
            {
                stale_count += 1;
                let _ignored = event_sender.send(format!(
                    "child_liveness_stale: child_id={} last_heartbeat_at={}",
                    child_id, last_hb
                ));
            }
        }
        stale_count
    }

    /// Checks whether a child slot is eligible for restart.
    ///
    /// Returns `Ok(())` when restart may proceed, or `Err(AdmissionConflict)`
    /// when the slot has a pending restart or an active attempt.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Child to check.
    /// - `request_generation`: Generation claimed by the restart request.
    /// - `request_attempt`: Attempt number claimed by the restart request.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` or `Err(AdmissionConflict)`.
    pub(crate) fn check_slot_restart_eligibility(
        &self,
        child_id: &ChildId,
        request_generation: Generation,
        request_attempt: ChildStartCount,
    ) -> Result<(), AdmissionConflict> {
        let Some(slot) = self.slots.get(child_id) else {
            return Ok(());
        };
        if slot.pending_restart {
            return Err(AdmissionConflict::new(
                child_id.clone(),
                slot.generation.unwrap_or(Generation::initial()),
                slot.attempt.unwrap_or(ChildStartCount::first()),
                "restart rejected: pending restart already exists",
            ));
        }
        if let (Some(active_gen), Some(active_att)) = (slot.generation, slot.attempt)
            && (request_generation != active_gen || request_attempt != active_att)
        {
            return Err(AdmissionConflict::new(
                child_id.clone(),
                active_gen,
                active_att,
                "restart conflicts with active attempt",
            ));
        }
        Ok(())
    }

    /// Copies a `ChildRuntimeState` entry into a `ChildSlot` when a slot
    /// does not yet exist for the child.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Child to ensure has a slot.
    /// - `path`: Supervisor path for the child.
    ///
    /// # Returns
    ///
    /// Returns `true` when a new slot was created.
    pub(crate) fn ensure_slot_exists(&mut self, child_id: ChildId, path: SupervisorPath) -> bool {
        if self.slots.contains_key(&child_id) {
            return false;
        }
        let slot = ChildSlot::new(
            child_id.clone(),
            path,
            Duration::from_secs(60), // Default restart window.
        );
        self.slots.insert(child_id, slot);
        true
    }
}

/// Builds a child control command outcome from the latest runtime state.
///
/// # Arguments
///
/// - `operation_before`: Operation observed before command handling.
/// - `cancel_delivered`: Whether this command delivered cancellation.
/// - `idempotent`: Whether this command reused existing state.
/// - `failure`: Failure observed during command handling.
/// - `runtime_state`: Runtime state used as the source of truth.
/// - `time_base`: Runtime time base used for liveness timestamps.
/// - `generation_fence`: Optional fencing metadata for restart commands only.
///
/// # Returns
///
/// Returns a [`ChildControlResult`] value.
fn build_child_control_outcome(
    operation_before: ChildControlOperation,
    cancel_delivered: bool,
    idempotent: bool,
    failure: Option<ChildControlFailure>,
    runtime_state: &mut ChildSlot,
    time_base: &RuntimeTimeBase,
    generation_fence: Option<GenerationFenceOutcome>,
) -> ChildControlResult {
    let liveness = runtime_state.observe_liveness(time_base.now_unix_nanos());
    ChildControlResult::new(
        runtime_state.child_id.clone(),
        runtime_state.attempt,
        runtime_state.generation,
        operation_before,
        runtime_state.operation,
        Some(runtime_state.status),
        cancel_delivered,
        runtime_state.stop_state,
        runtime_state.restart_limit.clone(),
        liveness,
        idempotent,
        failure,
        generation_fence,
    )
}

/// Result of applying a stop-style control command to a runtime state record.
#[derive(Debug, Clone, Copy)]
struct StopControlApplication {
    /// Operation observed before command handling.
    operation_before: ChildControlOperation,
    /// Whether this command delivered cancellation.
    cancel_delivered: bool,
    /// Whether this command reused existing state.
    idempotent: bool,
    /// Whether the caller should remove the record after building the outcome.
    remove_after_outcome: bool,
}

/// Applies a stop-style child control command to one runtime state record.
///
/// # Arguments
///
/// - `runtime_state`: Runtime state that owns the target child.
/// - `target_operation`: Operation requested by the command.
/// - `command_name`: Stable command name.
/// - `command_id`: Stable command identifier.
/// - `correlation_id`: Correlation identifier for emitted events.
/// - `stop_deadline_at_unix_nanos`: Deadline written when cancellation is delivered.
/// - `event_sender`: Event channel used for lifecycle text.
/// - `pending_events`: Typed events collected until mutable borrows end.
///
/// # Returns
///
/// Returns the applied stop control facts.
#[allow(clippy::too_many_arguments)]
fn apply_stop_control_to_runtime_state(
    runtime_state: &mut ChildSlot,
    target_operation: ChildControlOperation,
    command_name: &'static str,
    command_id: &str,
    correlation_id: CorrelationId,
    stop_deadline_at_unix_nanos: u128,
    event_sender: &broadcast::Sender<String>,
    pending_events: &mut Vec<PendingRuntimeEvent>,
) -> StopControlApplication {
    let child_id = runtime_state.child_id.clone();
    let operation_before = runtime_state.operation;
    let had_active_attempt = runtime_state.has_active_attempt();
    let already_cancelled_for_target = had_active_attempt
        && operation_before == target_operation
        && runtime_state.attempt_cancel_delivered;
    let idempotent = if had_active_attempt {
        already_cancelled_for_target
    } else {
        operation_before == target_operation && target_operation != ChildControlOperation::Removed
    };

    if operation_before != target_operation {
        runtime_state.operation = target_operation;
        pending_events.push(PendingRuntimeEvent {
            child_id: child_id.clone(),
            path: runtime_state.path.clone(),
            generation: runtime_state.generation,
            attempt: runtime_state.attempt,
            correlation_id,
            what: What::ChildControlOperationChanged {
                child_id: child_id.clone(),
                from: operation_before,
                to: target_operation,
                command: command_name.to_owned(),
                command_id: command_id.to_owned(),
            },
        });
        let _ignored = event_sender.send(format!(
            "child_control_operation_changed:{child_id}:{operation_before:?}:{target_operation:?}"
        ));
    }

    let cancel_delivered = if had_active_attempt && !already_cancelled_for_target {
        let delivered = runtime_state.cancel();
        if delivered {
            runtime_state.stop_deadline_at_unix_nanos = Some(stop_deadline_at_unix_nanos);
            if let (Some(generation), Some(attempt)) =
                (runtime_state.generation, runtime_state.attempt)
            {
                pending_events.push(PendingRuntimeEvent {
                    child_id: child_id.clone(),
                    path: runtime_state.path.clone(),
                    generation: Some(generation),
                    attempt: Some(attempt),
                    correlation_id,
                    what: What::ChildControlCancelDelivered {
                        child_id: child_id.clone(),
                        generation,
                        attempt,
                        command: command_name.to_owned(),
                        command_id: command_id.to_owned(),
                    },
                });
            }
            let _ignored = event_sender.send(format!(
                "child_control_cancel_delivered:{child_id}:{command_name}"
            ));
        }
        delivered
    } else {
        if !had_active_attempt {
            runtime_state.stop_state = ChildStopState::NoActiveAttempt;
        }
        false
    };

    StopControlApplication {
        operation_before,
        cancel_delivered,
        idempotent,
        remove_after_outcome: target_operation == ChildControlOperation::Removed
            && !had_active_attempt,
    }
}

/// Builds a stale heartbeat event when suppression allows emission.
///
/// # Arguments
///
/// - `runtime_state`: Runtime state whose heartbeat was observed.
/// - `liveness`: Latest liveness state.
///
/// # Returns
///
/// Returns a pending event when the stale heartbeat should be emitted.
fn heartbeat_stale_event(
    runtime_state: &mut ChildSlot,
    liveness: &ChildLivenessState,
) -> Option<PendingRuntimeEvent> {
    let Some(attempt) = runtime_state.attempt else {
        runtime_state.stale_event_attempt = None;
        return None;
    };
    if !liveness.heartbeat_stale {
        runtime_state.stale_event_attempt = None;
        return None;
    }
    if runtime_state.stale_event_attempt == Some(attempt) {
        return None;
    }
    let since_unix_nanos = liveness.last_heartbeat_at_unix_nanos?;
    runtime_state.stale_event_attempt = Some(attempt);
    Some(PendingRuntimeEvent {
        child_id: runtime_state.child_id.clone(),
        path: runtime_state.path.clone(),
        generation: runtime_state.generation,
        attempt: Some(attempt),
        correlation_id: CorrelationId::new(),
        what: What::ChildHeartbeatStale {
            child_id: runtime_state.child_id.clone(),
            attempt,
            since_unix_nanos,
        },
    })
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
            RuntimeLoopMessage::ChildStart(ChildStartMessage::Exited { report }) => {
                state.handle_child_exit(*report, &event_sender);
            }
            RuntimeLoopMessage::ChildStart(ChildStartMessage::StartFailed {
                child_id,
                message,
            }) => {
                state.handle_child_start_failed(child_id, message, &event_sender);
            }
            RuntimeLoopMessage::ChildStart(ChildStartMessage::DelayedSpawnAttached {
                child_id,
                path,
                generation,
                attempt,
                handle,
            }) => {
                state.attach_spawned_child_handle(child_id, path, generation, attempt, handle);
            }
            RuntimeLoopMessage::ControlPlane(ControlPlaneMessage::ReplayChildExitForTest {
                report,
            }) => {
                state.handle_child_exit(*report, &event_sender);
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
        Ok(report) => RuntimeLoopMessage::ChildStart(ChildStartMessage::Exited {
            report: Box::new(report),
        }),
        Err(error) => RuntimeLoopMessage::ChildStart(ChildStartMessage::StartFailed {
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
/// Maps a child spec backoff policy into the policy-engine equivalent.
///
/// # Arguments
///
/// - `policy`: Backoff policy stored on the child declaration.
///
/// # Returns
///
/// Returns the equivalent policy-engine value with full jitter enabled.
fn backoff_policy(policy: crate::spec::child::BackoffPolicy) -> BackoffPolicy {
    let jitter_percent = (policy.jitter_ratio * 100.0).round().clamp(0.0, 100.0) as u8;
    BackoffPolicy::new(
        policy.initial_delay,
        policy.max_delay,
        jitter_percent,
        policy.max_delay,
    )
    .with_full_jitter(42) // Enable full jitter mode per FR-003
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

/// Classifies a child-runner exit into pipeline exit classification.
///
/// This function maps all six minimum required exit kinds from the specification:
/// success, nonzero_exit, panic, timeout, external_cancel, manual_stop.
///
/// # Arguments
///
/// - `exit`: Exit reported by the child runner.
///
/// # Returns
///
/// Returns the pipeline exit classification value.
fn classify_exit_for_pipeline(exit: &TaskExit, manual_stop_requested: bool) -> ExitClassification {
    match exit {
        TaskExit::Succeeded => ExitClassification::Success,
        TaskExit::Cancelled if manual_stop_requested => ExitClassification::ManualStop,
        TaskExit::Cancelled => ExitClassification::ExternalCancel,
        TaskExit::Failed(failure) => {
            // Check if this is an external cancel or timeout based on failure kind.
            match failure.kind {
                crate::error::types::TaskFailureKind::Cancelled if manual_stop_requested => {
                    ExitClassification::ManualStop
                }
                crate::error::types::TaskFailureKind::Cancelled => {
                    ExitClassification::ExternalCancel
                }
                crate::error::types::TaskFailureKind::Timeout => ExitClassification::Timeout,
                _ => ExitClassification::NonZeroExit { exit_code: -1 },
            }
        }
        TaskExit::Panicked(_) => ExitClassification::Crash {
            reason: "panic".to_string(),
        },
        TaskExit::TimedOut => ExitClassification::Timeout,
    }
}

/// Reports whether the role policy should restart a successful exit.
///
/// # Arguments
///
/// - `pipeline_result`: Completed supervision pipeline context.
///
/// # Returns
///
/// Returns `true` when the effective role treats success as a restartable exit.
fn role_policy_restarts_success(pipeline_result: &PipelineContext) -> bool {
    pipeline_result.exit_classification == Some(ExitClassification::Success)
        && pipeline_result
            .effective_policy
            .as_ref()
            .is_some_and(|policy| policy.policy_pack.on_success_exit == OnSuccessAction::Restart)
}

/// Builds the effective policy for a child before budget evaluation.
///
/// # Arguments
///
/// - `child_spec`: Child specification whose declared role and overrides should be merged.
///
/// # Returns
///
/// Returns an [`EffectivePolicy`] ready for the supervision pipeline.
fn prepare_effective_policy(child_spec: &ChildSpec) -> EffectivePolicy {
    EffectivePolicy::for_child(child_spec)
}

/// Reports whether an exit should consume restart limit accounting.
///
/// # Arguments
///
/// - `exit`: Exit reported by the child runner.
///
/// # Returns
///
/// Returns `true` when the exit is an unplanned failure.
fn restart_limit_counts_exit(exit: &TaskExit) -> bool {
    matches!(
        exit,
        TaskExit::Failed(_) | TaskExit::Panicked(_) | TaskExit::TimedOut
    )
}

/// Resolves the restart limit for a child from the supervisor strategy layers.
///
/// # Arguments
///
/// - `tree`: Supervisor tree used for child group lookup.
/// - `spec`: Supervisor specification that owns restart limit layers.
/// - `child_id`: Child whose restart limit should be resolved.
///
/// # Returns
///
/// Returns the selected restart limit or the runtime default.
fn restart_limit_for_child_in_spec(
    tree: &SupervisorTree,
    spec: &SupervisorSpec,
    child_id: &ChildId,
) -> RestartLimit {
    restart_execution_plan(tree, spec, child_id)
        .restart_limit
        .unwrap_or_else(default_restart_limit)
}

/// Returns the runtime default restart limit.
///
/// # Arguments
///
/// This function has no arguments.
///
/// # Returns
///
/// Returns a conservative effectively-unbounded restart limit.
fn default_restart_limit() -> RestartLimit {
    RestartLimit::new(u32::MAX, Duration::from_secs(60))
}

/// Builds a deterministic restart outcome for unknown identifiers.
///
/// # Arguments
///
/// - `child_id`: Stable child referenced by the command.
///
/// # Returns
///
/// Returns a rejection [`ChildControlResult`] with structured fencing metadata.
fn restart_child_unknown_outcome(child_id: ChildId) -> ChildControlResult {
    let conflict = ChildControlFailure::new(
        ChildControlFailurePhase::WaitCompletion,
        "unknown child",
        false,
    );
    let fence = GenerationFenceOutcome::new(
        GenerationFenceDecision::Rejected,
        None,
        None,
        None,
        false,
        false,
        Some(conflict.clone()),
    );
    ChildControlResult::new(
        child_id,
        None,
        None,
        ChildControlOperation::Active,
        ChildControlOperation::Active,
        None,
        false,
        ChildStopState::NoActiveAttempt,
        RestartLimitState::default(),
        ChildLivenessState::new(
            None,
            false,
            crate::readiness::signal::ReadinessState::Unreported,
        ),
        false,
        Some(conflict),
        Some(fence),
    )
}

/// Classifies a child control command outcome for metrics.
///
/// # Arguments
///
/// - `outcome`: Child control command outcome.
///
/// # Returns
///
/// Returns `accepted`, `idempotent`, or `failed`.
fn child_control_result_label(outcome: &ChildControlResult) -> &'static str {
    if outcome.failure.is_some() || outcome.stop_state == ChildStopState::Failed {
        "failed"
    } else if outcome.idempotent {
        "idempotent"
    } else {
        "accepted"
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

/// Maps managed child state into a control operation.
///
/// # Arguments
///
/// - `state`: Managed child state used by the current control loop.
///
/// # Returns
///
/// Returns the equivalent child control operation.
/// Builds a child shutdown outcome from a completed run report.
///
/// # Arguments
///
/// - `runtime_state`: Runtime state that produced the report.
/// - `report`: Completed child run report.
/// - `status`: Shutdown status assigned to the report.
/// - `phase`: Shutdown phase where the report was consumed.
/// - `reason`: Human-readable diagnostic reason.
///
/// # Returns
///
/// Returns a [`ChildShutdownOutcome`].
fn outcome_from_report(
    runtime_state: &ChildSlot,
    report: &ChildRunReport,
    status: ChildShutdownStatus,
    phase: ShutdownPhase,
    reason: impl Into<String>,
) -> ChildShutdownOutcome {
    ChildShutdownOutcome::new(ChildShutdownOutcomeInput {
        child_id: runtime_state.child_id.clone(),
        path: runtime_state.path.clone(),
        generation: runtime_state.generation.unwrap_or_else(Generation::initial),
        child_start_count: runtime_state.attempt.unwrap_or_else(ChildStartCount::first),
        status,
        cancel_delivered: runtime_state.attempt_cancel_delivered,
        exit: Some(report.exit.clone()),
        phase,
        reason: reason.into(),
    })
}

/// Builds a shutdown outcome for a removed runtime state record.
///
/// # Arguments
///
/// - `runtime_state`: Removed runtime state skipped by shutdown.
/// - `phase`: Shutdown phase that observed the removed state.
///
/// # Returns
///
/// Returns a [`ChildShutdownOutcome`] marked as already exited.
fn removed_runtime_state_shutdown_outcome(
    runtime_state: &ChildSlot,
    phase: ShutdownPhase,
) -> ChildShutdownOutcome {
    ChildShutdownOutcome::new(ChildShutdownOutcomeInput {
        child_id: runtime_state.child_id.clone(),
        path: runtime_state.path.clone(),
        generation: runtime_state.generation.unwrap_or_else(Generation::initial),
        child_start_count: runtime_state.attempt.unwrap_or_else(ChildStartCount::first),
        status: ChildShutdownStatus::AlreadyExited,
        cancel_delivered: false,
        exit: None,
        phase,
        reason: "child runtime state was already removed before shutdown".to_owned(),
    })
}

/// Builds a child shutdown outcome from a run report error.
///
/// # Arguments
///
/// - `runtime_state`: Runtime state that produced the error.
/// - `status`: Shutdown status assigned to the error.
/// - `phase`: Shutdown phase where the error was consumed.
/// - `error`: Error returned by the child run observer.
///
/// # Returns
///
/// Returns a [`ChildShutdownOutcome`].
fn outcome_from_error(
    runtime_state: &ChildSlot,
    status: ChildShutdownStatus,
    phase: ShutdownPhase,
    error: SupervisorError,
) -> ChildShutdownOutcome {
    ChildShutdownOutcome::new(ChildShutdownOutcomeInput {
        child_id: runtime_state.child_id.clone(),
        path: runtime_state.path.clone(),
        generation: runtime_state.generation.unwrap_or_else(Generation::initial),
        child_start_count: runtime_state.attempt.unwrap_or_else(ChildStartCount::first),
        status,
        cancel_delivered: runtime_state.attempt_cancel_delivered,
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
