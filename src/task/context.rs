//! Task execution context.
//!
//! This module provides the per-attempt handles that a task uses to observe
//! cancellation, emit heartbeats, and report readiness.

use crate::id::types::{Attempt, ChildId, Generation, SupervisorPath};
use crate::readiness::signal::ReadySignal;
use tokio::sync::watch;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

/// Context passed to a task for a single attempt.
#[derive(Debug, Clone)]
pub struct TaskContext {
    /// Stable child identifier for the task attempt.
    pub child_id: ChildId,
    /// Full path of the child in the supervisor tree.
    pub path: SupervisorPath,
    /// Generation for the runtime slot that owns this attempt.
    pub generation: Generation,
    /// Attempt number for the running task.
    pub attempt: Attempt,
    /// Token that tells the task when cancellation was requested.
    cancellation_token: CancellationToken,
    /// Sender used to report readiness.
    ready_signal: ReadySignal,
    /// Sender used to publish the latest heartbeat time.
    heartbeat_sender: watch::Sender<Option<Instant>>,
}

impl TaskContext {
    /// Creates a task context for a child attempt.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Stable child identifier.
    /// - `path`: Full supervisor tree path for this child.
    /// - `generation`: Runtime slot generation.
    /// - `attempt`: Attempt number for this execution.
    ///
    /// # Returns
    ///
    /// Returns the context and a heartbeat receiver for runtime observers.
    ///
    /// # Examples
    ///
    /// ```
    /// let (ctx, _heartbeat) = rust_supervisor::task::context::TaskContext::new(
    ///     rust_supervisor::id::types::ChildId::new("worker"),
    ///     rust_supervisor::id::types::SupervisorPath::root().join("worker"),
    ///     rust_supervisor::id::types::Generation::initial(),
    ///     rust_supervisor::id::types::Attempt::first(),
    /// );
    /// assert!(!ctx.is_cancelled());
    /// ```
    pub fn new(
        child_id: ChildId,
        path: SupervisorPath,
        generation: Generation,
        attempt: Attempt,
    ) -> (Self, watch::Receiver<Option<Instant>>) {
        let (ready_signal, _ready_receiver) = ReadySignal::new();
        let (heartbeat_sender, heartbeat_receiver) = watch::channel(None);
        (
            Self {
                child_id,
                path,
                generation,
                attempt,
                cancellation_token: CancellationToken::new(),
                ready_signal,
                heartbeat_sender,
            },
            heartbeat_receiver,
        )
    }

    /// Creates a task context with an existing readiness signal.
    ///
    /// # Arguments
    ///
    /// - `child_id`: Stable child identifier.
    /// - `path`: Full supervisor tree path for this child.
    /// - `generation`: Runtime slot generation.
    /// - `attempt`: Attempt number for this execution.
    /// - `ready_signal`: Signal used to publish readiness.
    ///
    /// # Returns
    ///
    /// Returns the context and a heartbeat receiver for runtime observers.
    pub fn with_ready_signal(
        child_id: ChildId,
        path: SupervisorPath,
        generation: Generation,
        attempt: Attempt,
        ready_signal: ReadySignal,
    ) -> (Self, watch::Receiver<Option<Instant>>) {
        let (heartbeat_sender, heartbeat_receiver) = watch::channel(None);
        (
            Self {
                child_id,
                path,
                generation,
                attempt,
                cancellation_token: CancellationToken::new(),
                ready_signal,
                heartbeat_sender,
            },
            heartbeat_receiver,
        )
    }

    /// Reports that the task is ready.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    pub fn mark_ready(&self) {
        self.ready_signal.mark_ready();
    }

    /// Emits a heartbeat with the current monotonic time.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    pub fn heartbeat(&self) {
        let _ignored = self.heartbeat_sender.send(Some(Instant::now()));
    }

    /// Requests cancellation for this task attempt.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    pub fn cancel(&self) {
        self.cancellation_token.cancel();
    }

    /// Returns whether cancellation was requested.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns `true` when cancellation was requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancellation_token.is_cancelled()
    }

    /// Returns a clone of the cancellation token.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the cancellation token for asynchronous selection.
    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation_token.clone()
    }

    /// Subscribes to readiness updates.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a receiver that observes readiness changes.
    pub fn readiness_receiver(&self) -> watch::Receiver<bool> {
        self.ready_signal.subscribe()
    }
}
