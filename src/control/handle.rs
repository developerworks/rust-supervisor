//! Public runtime control handle.
//!
//! The handle owns the command sender side and exposes asynchronous control
//! methods. It keeps command construction separate from runtime execution.

use crate::child_runner::runner::ChildRunReport;
use crate::control::command::{CommandMeta, CommandResult, ControlCommand};
use crate::dashboard::runtime::DashboardIpcRuntimeGuard;
use crate::error::types::SupervisorError;
use crate::id::types::{ChildId, SupervisorPath};
use crate::observe::pipeline::{ObservabilityPipeline, TestRecorder};
use crate::runtime::lifecycle::{RuntimeControlPlane, RuntimeExitReport, RuntimeHealthReport};
use crate::runtime::message::{ControlPlaneMessage, RuntimeLoopMessage};
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::{broadcast, mpsc, oneshot};

/// Cloneable handle used to control a running supervisor.
#[derive(Debug, Clone)]
pub struct SupervisorHandle {
    /// Sender side used to submit runtime control commands.
    command_sender: mpsc::Sender<RuntimeLoopMessage>,
    /// Broadcast sender used to create lifecycle event subscriptions.
    event_sender: broadcast::Sender<String>,
    /// Runtime control plane lifecycle state.
    control_plane: RuntimeControlPlane,
    /// Shared typed observability pipeline.
    observability: Arc<Mutex<ObservabilityPipeline>>,
    /// Optional dashboard IPC runtime guard.
    dashboard_runtime: Option<Arc<DashboardIpcRuntimeGuard>>,
}

impl SupervisorHandle {
    /// Creates a runtime handle from channel senders.
    ///
    /// # Arguments
    ///
    /// - `command_sender`: Sender used to submit runtime commands.
    /// - `event_sender`: Sender used to subscribe to lifecycle events.
    ///
    /// # Returns
    ///
    /// Returns a [`SupervisorHandle`].
    pub fn new(
        command_sender: mpsc::Sender<RuntimeLoopMessage>,
        event_sender: broadcast::Sender<String>,
        control_plane: RuntimeControlPlane,
    ) -> Self {
        Self::new_with_observability(
            command_sender,
            event_sender,
            control_plane,
            Arc::new(Mutex::new(ObservabilityPipeline::new(16, 16))),
        )
    }

    /// Creates a runtime handle with a shared observability pipeline.
    ///
    /// # Arguments
    ///
    /// - `command_sender`: Sender used to submit runtime commands.
    /// - `event_sender`: Sender used to subscribe to lifecycle event text.
    /// - `control_plane`: Runtime control plane lifecycle state.
    /// - `observability`: Shared typed observability pipeline.
    ///
    /// # Returns
    ///
    /// Returns a [`SupervisorHandle`].
    pub(crate) fn new_with_observability(
        command_sender: mpsc::Sender<RuntimeLoopMessage>,
        event_sender: broadcast::Sender<String>,
        control_plane: RuntimeControlPlane,
        observability: Arc<Mutex<ObservabilityPipeline>>,
    ) -> Self {
        Self {
            command_sender,
            event_sender,
            control_plane,
            observability,
            dashboard_runtime: None,
        }
    }

    /// Attaches a dashboard IPC runtime guard to this handle.
    ///
    /// # Arguments
    ///
    /// - `dashboard_runtime`: Guard that owns dashboard IPC runtime tasks.
    ///
    /// # Returns
    ///
    /// Returns this handle with dashboard runtime lifecycle attached.
    pub(crate) fn with_dashboard_runtime(
        mut self,
        dashboard_runtime: Arc<DashboardIpcRuntimeGuard>,
    ) -> Self {
        self.dashboard_runtime = Some(dashboard_runtime);
        self
    }

    /// Adds a child manifest under a supervisor path.
    ///
    /// # Arguments
    ///
    /// - `target`: Supervisor path that should receive the child.
    /// - `child_manifest`: Child manifest text supplied by the caller.
    /// - `requested_by`: Caller that requested the command.
    /// - `reason`: Human-readable command reason.
    ///
    /// # Child Manifest Example
    ///
    /// The runtime expects a YAML child declaration. The smallest useful
    /// manifest names the child and selects a task kind:
    ///
    /// ```yaml
    /// name: worker
    /// kind: async_worker
    /// ```
    ///
    /// Optional fields can be added when the child needs dependencies,
    /// lifecycle policy, resource limits, command permissions, environment
    /// variables, or secret references:
    ///
    /// ```yaml
    /// name: worker
    /// kind: async_worker
    /// criticality: optional
    /// restart_policy: transient
    /// dependencies:
    ///   - cache
    /// health_check:
    ///   check_interval_secs: 10
    ///   timeout_secs: 5
    ///   max_retries: 3
    /// readiness:
    ///   check_interval_secs: 5
    ///   timeout_secs: 3
    /// resource_limits:
    ///   max_memory_mb: 256
    ///   max_cpu_percent: 80
    ///   max_file_descriptors: 1024
    /// command_permissions:
    ///   allow_shutdown: false
    ///   allow_restart: true
    ///   allowed_signals:
    ///     - SIGTERM
    /// environment:
    ///   - name: WORKER_MODE
    ///     value: queue
    ///   - name: API_TOKEN
    ///     secret_ref: ${API_TOKEN}
    /// secrets:
    ///   - name: API_TOKEN
    ///     key: workers/api_token
    ///     required: true
    /// ```
    ///
    /// # Example
    ///
    /// ```no_run
    /// # async fn add_child_example() -> Result<(), rust_supervisor::error::types::SupervisorError> {
    /// use rust_supervisor::control::command::CommandResult;
    /// use rust_supervisor::id::types::SupervisorPath;
    /// use rust_supervisor::runtime::supervisor::Supervisor;
    /// use rust_supervisor::spec::supervisor::SupervisorSpec;
    ///
    /// let handle = Supervisor::start(SupervisorSpec::root(Vec::new())).await?;
    /// let result = handle
    ///     .add_child(
    ///         SupervisorPath::root(),
    ///         "name: worker\nkind: async_worker\n",
    ///         "operator",
    ///         "attach worker during runtime update",
    ///     )
    ///     .await?;
    ///
    /// assert!(matches!(result, CommandResult::ChildAdded { .. }));
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Returns
    ///
    /// Returns a [`CommandResult`] after the runtime accepts the command.
    pub async fn add_child(
        &self,
        target: SupervisorPath,
        child_manifest: impl Into<String>,
        requested_by: impl Into<String>,
        reason: impl Into<String>,
    ) -> Result<CommandResult, SupervisorError> {
        self.send(ControlCommand::AddChild {
            meta: CommandMeta::new(requested_by, reason),
            target,
            child_manifest: child_manifest.into(),
        })
        .await
    }

    /// Removes a child from runtime governance.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Target child identifier.
    /// - `requested_by`: Caller that requested the command.
    /// - `reason`: Human-readable command reason.
    ///
    /// # Returns
    ///
    /// Returns a [`CommandResult`] after removal or idempotent reuse.
    pub async fn remove_child(
        &self,
        child_id: ChildId,
        requested_by: impl Into<String>,
        reason: impl Into<String>,
    ) -> Result<CommandResult, SupervisorError> {
        self.child_command(child_id, requested_by, reason, |meta, child_id| {
            ControlCommand::RemoveChild { meta, child_id }
        })
        .await
    }

    /// Restarts a child explicitly.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Target child identifier.
    /// - `requested_by`: Caller that requested the command.
    /// - `reason`: Human-readable command reason.
    ///
    /// # Returns
    ///
    /// Returns a [`CommandResult`] after restart dispatch.
    pub async fn restart_child(
        &self,
        child_id: ChildId,
        requested_by: impl Into<String>,
        reason: impl Into<String>,
    ) -> Result<CommandResult, SupervisorError> {
        self.child_command(child_id, requested_by, reason, |meta, child_id| {
            ControlCommand::RestartChild { meta, child_id }
        })
        .await
    }

    /// Pauses a child idempotently.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Target child identifier.
    /// - `requested_by`: Caller that requested the command.
    /// - `reason`: Human-readable command reason.
    ///
    /// # Returns
    ///
    /// Returns the current child state after the command.
    pub async fn pause_child(
        &self,
        child_id: ChildId,
        requested_by: impl Into<String>,
        reason: impl Into<String>,
    ) -> Result<CommandResult, SupervisorError> {
        self.child_command(child_id, requested_by, reason, |meta, child_id| {
            ControlCommand::PauseChild { meta, child_id }
        })
        .await
    }

    /// Resumes a child idempotently.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Target child identifier.
    /// - `requested_by`: Caller that requested the command.
    /// - `reason`: Human-readable command reason.
    ///
    /// # Returns
    ///
    /// Returns the current child state after the command.
    pub async fn resume_child(
        &self,
        child_id: ChildId,
        requested_by: impl Into<String>,
        reason: impl Into<String>,
    ) -> Result<CommandResult, SupervisorError> {
        self.child_command(child_id, requested_by, reason, |meta, child_id| {
            ControlCommand::ResumeChild { meta, child_id }
        })
        .await
    }

    /// Quarantines a child idempotently.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Target child identifier.
    /// - `requested_by`: Caller that requested the command.
    /// - `reason`: Human-readable command reason.
    ///
    /// # Returns
    ///
    /// Returns the current child state after the command.
    pub async fn quarantine_child(
        &self,
        child_id: ChildId,
        requested_by: impl Into<String>,
        reason: impl Into<String>,
    ) -> Result<CommandResult, SupervisorError> {
        self.child_command(child_id, requested_by, reason, |meta, child_id| {
            ControlCommand::QuarantineChild { meta, child_id }
        })
        .await
    }

    /// Shuts down the supervisor tree idempotently.
    ///
    /// # Arguments
    ///
    /// - `requested_by`: Caller that requested shutdown.
    /// - `reason`: Human-readable shutdown reason.
    ///
    /// # Returns
    ///
    /// Returns the current shutdown result.
    pub async fn shutdown_tree(
        &self,
        requested_by: impl Into<String>,
        reason: impl Into<String>,
    ) -> Result<CommandResult, SupervisorError> {
        self.send(ControlCommand::ShutdownTree {
            meta: CommandMeta::new(requested_by, reason),
        })
        .await
    }

    /// Queries the current runtime state.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a [`CommandResult::CurrentState`] value.
    pub async fn current_state(&self) -> Result<CommandResult, SupervisorError> {
        self.send(ControlCommand::CurrentState {
            meta: CommandMeta::new("system", "current_state"),
        })
        .await
    }

    /// Reports whether the runtime control loop is alive.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns `true` when ordinary control commands may still be accepted.
    pub fn is_alive(&self) -> bool {
        self.control_plane.is_alive()
    }

    /// Returns a runtime control plane health report.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a [`RuntimeHealthReport`] value for the current observation.
    pub fn health(&self) -> RuntimeHealthReport {
        self.control_plane.health()
    }

    /// Waits until the runtime control plane reaches a final state.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the cached [`RuntimeExitReport`].
    pub async fn join(&self) -> Result<RuntimeExitReport, SupervisorError> {
        let report = self.control_plane.join().await;
        let _ignored = self.event_sender.send(format!(
            "runtime_control_loop_join_completed:{}:{}:{}",
            report.state.as_str(),
            report.phase,
            report.reason
        ));
        Ok(report)
    }

    /// Requests shutdown for the runtime control plane itself.
    ///
    /// # Arguments
    ///
    /// - `requested_by`: Caller that requested shutdown.
    /// - `reason`: Human-readable shutdown reason.
    ///
    /// # Returns
    ///
    /// Returns the final [`RuntimeExitReport`].
    pub async fn shutdown(
        &self,
        requested_by: impl Into<String>,
        reason: impl Into<String>,
    ) -> Result<RuntimeExitReport, SupervisorError> {
        let meta = CommandMeta::new(requested_by, reason);
        if let Some(report) = self
            .control_plane
            .mark_shutdown_requested(meta.requested_by.clone(), meta.reason.clone())?
        {
            return Ok(report);
        }

        let (reply_sender, reply_receiver) = oneshot::channel();
        if self
            .command_sender
            .send(RuntimeLoopMessage::ControlPlane(
                ControlPlaneMessage::Shutdown { meta, reply_sender },
            ))
            .await
            .is_err()
        {
            return Err(self
                .closed_runtime_error_after_join("runtime control loop is closed")
                .await);
        }
        match reply_receiver.await {
            Ok(result) => {
                result?;
            }
            Err(_) => {
                return Err(self
                    .closed_runtime_error_after_join("runtime control loop dropped shutdown reply")
                    .await);
            }
        }
        self.join().await
    }

    /// Subscribes to runtime event text emitted by the control loop.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a broadcast receiver for event text.
    pub fn subscribe_events(&self) -> broadcast::Receiver<String> {
        let receiver = self.event_sender.subscribe();
        if self.control_plane.is_alive() {
            let _ignored = self
                .event_sender
                .send("runtime_control_loop_started:startup".to_owned());
        }
        receiver
    }

    /// Returns a copy of the typed observability test recorder.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the currently retained [`TestRecorder`] contents.
    pub fn observability_recorder(&self) -> TestRecorder {
        self.observability
            .lock()
            .map(|pipeline| pipeline.test_recorder.clone())
            .unwrap_or_default()
    }

    /// Hidden integration-test hook that feeds a synthetic [`ChildRunReport`] through the mailbox.
    ///
    /// Production callers must not rely on this hook.
    #[doc(hidden)]
    pub async fn generation_fencing_replay_child_exit_for_test(
        &self,
        report: ChildRunReport,
    ) -> Result<(), SupervisorError> {
        if let Some(report_final) = self.control_plane.final_report() {
            return Err(runtime_exit_error(&report_final));
        }
        if !self.control_plane.is_alive() {
            return Err(SupervisorError::InvalidTransition {
                message: format!(
                    "runtime control loop is not alive: state={}",
                    self.control_plane.health().state.as_str()
                ),
            });
        }
        self.command_sender
            .send(RuntimeLoopMessage::ControlPlane(
                ControlPlaneMessage::ReplayChildExitForTest {
                    report: Box::new(report),
                },
            ))
            .await
            .map_err(|_| SupervisorError::InvalidTransition {
                message: "runtime control loop is closed".to_owned(),
            })
    }

    /// Sends one control command and waits for the result.
    ///
    /// # Arguments
    ///
    /// - `command`: Command that should be executed by the runtime loop.
    ///
    /// # Returns
    ///
    /// Returns a command result or a supervisor error when the runtime is gone.
    async fn send(&self, command: ControlCommand) -> Result<CommandResult, SupervisorError> {
        if let Some(report) = self.control_plane.final_report() {
            return Err(runtime_exit_error(&report));
        }
        if !self.control_plane.is_alive() {
            return Err(SupervisorError::InvalidTransition {
                message: format!(
                    "runtime control loop is not alive: state={}",
                    self.control_plane.health().state.as_str()
                ),
            });
        }
        command.validate_audit_metadata()?;
        let (reply_sender, reply_receiver) = oneshot::channel();
        if self
            .command_sender
            .send(RuntimeLoopMessage::Control {
                command,
                reply_sender,
            })
            .await
            .is_err()
        {
            return Err(self
                .closed_runtime_error_after_join("runtime control loop is closed")
                .await);
        }
        match reply_receiver.await {
            Ok(result) => result,
            Err(_) => Err(self
                .closed_runtime_error_after_join("runtime control loop dropped command reply")
                .await),
        }
    }

    /// Builds and sends a child-targeted command.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Target child identifier.
    /// - `requested_by`: Caller that requested the command.
    /// - `reason`: Human-readable command reason.
    /// - `builder`: Command builder for the child operation.
    ///
    /// # Returns
    ///
    /// Returns a command result from the runtime loop.
    async fn child_command<F>(
        &self,
        child_id: ChildId,
        requested_by: impl Into<String>,
        reason: impl Into<String>,
        builder: F,
    ) -> Result<CommandResult, SupervisorError>
    where
        F: FnOnce(CommandMeta, ChildId) -> ControlCommand,
    {
        let meta = CommandMeta::new(requested_by, reason);
        self.send(builder(meta, child_id)).await
    }

    /// Builds an error after waiting for the runtime exit report when possible.
    async fn closed_runtime_error_after_join(&self, fallback: &str) -> SupervisorError {
        if let Some(report) = self.control_plane.final_report() {
            return runtime_exit_error(&report);
        }
        if self.command_sender.is_closed() {
            let report = self.control_plane.join().await;
            return runtime_exit_error(&report);
        }
        SupervisorError::InvalidTransition {
            message: fallback.to_owned(),
        }
    }
}

/// Builds a control command error from a runtime exit report.
fn runtime_exit_error(report: &RuntimeExitReport) -> SupervisorError {
    SupervisorError::InvalidTransition {
        message: format!(
            "runtime control loop already exited: state={}, phase={}, reason={}",
            report.state.as_str(),
            report.phase,
            report.reason
        ),
    }
}
