//! Runtime-owned shutdown pipeline helpers.
//!
//! This module stores cached shutdown reports. Active child handles live in
//! child runtime state records owned by the control loop.

use crate::shutdown::report::ShutdownPipelineReport;

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
