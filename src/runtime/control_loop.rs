//! Runtime control loop.
//!
//! This module executes control-plane commands, receives child attempt exits,
//! and applies supervisor restart strategy decisions.

use crate::child_runner::attempt::TaskExit;
use crate::child_runner::runner::{ChildRunReport, ChildRunner};
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
use crate::shutdown::coordinator::ShutdownCoordinator;
use crate::shutdown::stage::{ShutdownCause, ShutdownPolicy};
use crate::spec::child::RestartPolicy as ChildRestartPolicy;
use crate::spec::supervisor::SupervisorSpec;
use crate::tree::builder::SupervisorTree;
use crate::tree::order::{restart_execution_plan, startup_order};
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};

/// Mutable state owned by the control loop.
#[derive(Debug)]
pub struct RuntimeControlState {
    /// Shutdown state machine used by tree-level shutdown commands.
    shutdown: ShutdownCoordinator,
    /// Runtime child states set by explicit control commands.
    children: HashMap<ChildId, ManagedChildState>,
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
            children: HashMap::new(),
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
    ///
    /// # Returns
    ///
    /// Returns a command result.
    pub fn execute_control(
        &mut self,
        command: ControlCommand,
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
                let cause = ShutdownCause::new(meta.requested_by, meta.reason);
                let result = self.shutdown.request_stop(cause);
                self.shutdown.advance();
                self.shutdown.advance();
                self.shutdown.advance();
                self.shutdown.advance();
                self.shutdown.complete();
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
        self.record_child_exit(report);
        let _ignored = event_sender.send(format!("child_exit:{child_id}"));
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
        tokio::spawn(async move {
            if !delay.is_zero() {
                tokio::time::sleep(delay).await;
            }
            let child_id = runtime.id.clone();
            let result = ChildRunner::new().run_once(runtime).await;
            send_child_result(sender, child_id, result).await;
        });
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
                let result = state.execute_control(command);
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
