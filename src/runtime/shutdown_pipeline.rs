//! Runtime-owned shutdown pipeline helpers.
//!
//! This module stores active child attempt handles and cached shutdown reports.
//! It deliberately depends on public shutdown report types instead of moving
//! task handles into the shutdown module.

use crate::child_runner::runner::{ChildRunHandle, ChildRunReport, wait_for_report};
use crate::error::types::SupervisorError;
use crate::id::types::{Attempt, ChildId, Generation, SupervisorPath};
use crate::shutdown::report::ShutdownPipelineReport;
use tokio::sync::watch::Receiver;
use tokio::task::AbortHandle;
use tokio_util::sync::CancellationToken;

/// Running child attempt observed by the shutdown pipeline.
#[derive(Debug)]
pub(crate) struct ActiveChildAttempt {
    /// Stable child identifier.
    pub child_id: ChildId,
    /// Child path in the supervisor tree.
    pub path: SupervisorPath,
    /// Runtime slot generation.
    pub generation: Generation,
    /// Runtime attempt number.
    pub attempt: Attempt,
    /// Cancellation token shared with the task context.
    pub cancellation_token: CancellationToken,
    /// Abort handle attached to the real child future.
    pub abort_handle: AbortHandle,
    /// Completion receiver for the child run report.
    pub completion_receiver: Receiver<Option<Result<ChildRunReport, SupervisorError>>>,
    /// Whether runtime delivered cancellation during shutdown.
    pub cancel_delivered: bool,
    /// Whether runtime requested abort during shutdown.
    pub abort_requested: bool,
}

impl ActiveChildAttempt {
    /// Builds an active attempt from a child run handle.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Stable child identifier.
    /// - `path`: Child path in the supervisor tree.
    /// - `generation`: Runtime slot generation.
    /// - `attempt`: Runtime attempt number.
    /// - `handle`: Child run handle returned by the runner.
    ///
    /// # Returns
    ///
    /// Returns an [`ActiveChildAttempt`].
    pub(crate) fn new(
        child_id: ChildId,
        path: SupervisorPath,
        generation: Generation,
        attempt: Attempt,
        handle: ChildRunHandle,
    ) -> Self {
        Self {
            child_id,
            path,
            generation,
            attempt,
            cancellation_token: handle.cancellation_token,
            abort_handle: handle.abort_handle,
            completion_receiver: handle.completion_receiver,
            cancel_delivered: false,
            abort_requested: false,
        }
    }

    /// Delivers cancellation to the running child attempt.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    pub(crate) fn cancel(&mut self) {
        self.cancellation_token.cancel();
        self.cancel_delivered = true;
    }

    /// Requests abort for the running child attempt.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    pub(crate) fn abort(&mut self) {
        self.abort_handle.abort();
        self.abort_requested = true;
    }

    /// Waits for the child attempt report.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the completed child run report.
    pub(crate) async fn wait_for_report(&mut self) -> Result<ChildRunReport, SupervisorError> {
        wait_for_report(&mut self.completion_receiver).await
    }
}

/// Shutdown pipeline state stored by the runtime control loop.
#[derive(Debug, Default)]
pub(crate) struct ShutdownPipeline {
    /// Cached report after the first completed shutdown.
    cached_report: Option<ShutdownPipelineReport>,
}

impl ShutdownPipeline {
    /// Creates an empty shutdown pipeline cache.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a [`ShutdownPipeline`].
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Returns the cached shutdown report.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the cached shutdown report when shutdown already completed.
    pub(crate) fn cached_report(&self) -> Option<&ShutdownPipelineReport> {
        self.cached_report.as_ref()
    }

    /// Stores the completed shutdown report.
    ///
    /// # Arguments
    ///
    /// - `report`: Completed report to cache.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    pub(crate) fn cache_report(&mut self, report: ShutdownPipelineReport) {
        self.cached_report = Some(report);
    }
}
