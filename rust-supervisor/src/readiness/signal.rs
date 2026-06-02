//! Readiness policy and signal primitives.
//!
//! This module owns the small synchronization boundary that allows a task to
//! report readiness without exposing runtime internals.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::sync::watch;

/// Readiness observation state for one child.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessState {
    /// The child has not reported readiness.
    Unreported,
    /// The child reported readiness.
    Ready,
    /// The child reported that it is not ready.
    NotReady,
}

/// Policy that decides when a child becomes ready.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessPolicy {
    /// The child becomes ready as soon as its task starts running.
    Immediate,
    /// The child becomes ready only after its task reports readiness.
    Explicit,
}

impl ReadinessPolicy {
    /// Returns whether this policy marks a task ready immediately.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns `true` for [`ReadinessPolicy::Immediate`].
    ///
    /// # Examples
    ///
    /// ```
    /// let policy = rust_supervisor::readiness::signal::ReadinessPolicy::Immediate;
    /// assert!(policy.is_immediate());
    /// ```
    pub fn is_immediate(self) -> bool {
        matches!(self, Self::Immediate)
    }
}

/// Sender side used by a task context to publish readiness.
#[derive(Debug, Clone)]
pub struct ReadySignal {
    /// Watch channel that stores the latest readiness flag.
    sender: watch::Sender<ReadinessState>,
}

impl ReadySignal {
    /// Creates a readiness signal pair.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the signal handle and a receiver for readiness observers.
    ///
    /// # Examples
    ///
    /// ```
    /// let (signal, receiver) = rust_supervisor::readiness::signal::ReadySignal::new();
    /// signal.mark_ready();
    /// assert_eq!(
    ///     *receiver.borrow(),
    ///     rust_supervisor::readiness::signal::ReadinessState::Ready
    /// );
    /// ```
    pub fn new() -> (Self, watch::Receiver<ReadinessState>) {
        let (sender, receiver) = watch::channel(ReadinessState::Unreported);
        (Self { sender }, receiver)
    }

    /// Marks the child as ready.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    pub fn mark_ready(&self) {
        self.set_readiness(ReadinessState::Ready);
    }

    /// Sets the latest readiness state.
    ///
    /// # Arguments
    ///
    /// - `state`: Readiness state that observers should see.
    ///
    /// # Returns
    ///
    /// This function does not return a value.
    pub fn set_readiness(&self, state: ReadinessState) {
        let _ignored = self.sender.send(state);
    }

    /// Creates another receiver for readiness observers.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a receiver subscribed to the latest readiness value.
    pub fn subscribe(&self) -> watch::Receiver<ReadinessState> {
        self.sender.subscribe()
    }
}
