//! Role lifecycle phase labels.
//!
//! Lifecycle phase values are used in role errors and diagnostics so adapters
//! can report where a contract failed.

/// Lifecycle phase reached by a role contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoleLifecyclePhase {
    /// Role initialization before the primary body starts.
    Init,
    /// Long-running or one-shot role body.
    Run,
    /// Bounded worker body.
    Work,
    /// Nested supervisor tree construction.
    BuildTree,
    /// Completion hook after bounded or one-shot work succeeds.
    Complete,
    /// Shutdown hook after runtime cancellation.
    Shutdown,
}

impl RoleLifecyclePhase {
    /// Returns a stable lowercase phase label.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a stable lifecycle phase label for diagnostics.
    ///
    /// # Examples
    ///
    /// ```
    /// let phase = rust_supervisor::role::lifecycle::RoleLifecyclePhase::Run;
    /// assert_eq!(phase.as_str(), "run");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Init => "init",
            Self::Run => "run",
            Self::Work => "work",
            Self::BuildTree => "build_tree",
            Self::Complete => "complete",
            Self::Shutdown => "shutdown",
        }
    }
}
