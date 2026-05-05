//! Shutdown coordination state for four-stage runtime shutdown.
//!
//! This module owns idempotent phase transitions. Runtime code supplies concrete
//! task cancellation and join behavior around these transitions.

use crate::shutdown::stage::{ShutdownCause, ShutdownPhase, ShutdownPolicy};
use serde::{Deserialize, Serialize};

/// Result returned after a shutdown transition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShutdownResult {
    /// Current shutdown phase after the transition.
    pub phase: ShutdownPhase,
    /// Shutdown cause recorded for the first accepted request.
    pub cause: ShutdownCause,
    /// Whether this result reused an existing shutdown request.
    pub idempotent: bool,
}

/// Coordinates idempotent shutdown phases.
#[derive(Debug, Clone)]
pub struct ShutdownCoordinator {
    /// Policy that defines shutdown timing and abort behavior.
    pub policy: ShutdownPolicy,
    phase: ShutdownPhase,
    cause: Option<ShutdownCause>,
}

impl ShutdownCoordinator {
    /// Creates a coordinator in the idle phase.
    ///
    /// # Arguments
    ///
    /// - `policy`: Shutdown timing policy.
    ///
    /// # Returns
    ///
    /// Returns a [`ShutdownCoordinator`].
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    ///
    /// let policy = rust_supervisor::shutdown::stage::ShutdownPolicy::new(
    ///     Duration::from_secs(1),
    ///     Duration::from_secs(1),
    ///     true,
    /// );
    /// let coordinator = rust_supervisor::shutdown::coordinator::ShutdownCoordinator::new(policy);
    /// assert_eq!(coordinator.phase(), rust_supervisor::shutdown::stage::ShutdownPhase::Idle);
    /// ```
    pub fn new(policy: ShutdownPolicy) -> Self {
        Self {
            policy,
            phase: ShutdownPhase::Idle,
            cause: None,
        }
    }

    /// Requests shutdown and records the first cause.
    ///
    /// # Arguments
    ///
    /// - `cause`: Caller and reason for shutdown.
    ///
    /// # Returns
    ///
    /// Returns a [`ShutdownResult`] for the current phase.
    pub fn request_stop(&mut self, cause: ShutdownCause) -> ShutdownResult {
        if let Some(existing) = self.cause.clone() {
            return self.result(existing, true);
        }
        self.phase = ShutdownPhase::RequestStop;
        self.cause = Some(cause.clone());
        self.result(cause, false)
    }

    /// Advances to the next shutdown phase.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the updated phase.
    pub fn advance(&mut self) -> ShutdownPhase {
        if let Some(next) = self.phase.next() {
            self.phase = next;
        }
        self.phase
    }

    /// Marks shutdown as completed.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the completed phase.
    pub fn complete(&mut self) -> ShutdownPhase {
        self.phase = ShutdownPhase::Completed;
        self.phase
    }

    /// Returns the current phase.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the current [`ShutdownPhase`].
    pub fn phase(&self) -> ShutdownPhase {
        self.phase
    }

    /// Builds a shutdown result.
    ///
    /// # Arguments
    ///
    /// - `cause`: Recorded shutdown cause.
    /// - `idempotent`: Whether the caller reused an existing shutdown request.
    ///
    /// # Returns
    ///
    /// Returns a [`ShutdownResult`].
    fn result(&self, cause: ShutdownCause, idempotent: bool) -> ShutdownResult {
        ShutdownResult {
            phase: self.phase,
            cause,
            idempotent,
        }
    }
}
