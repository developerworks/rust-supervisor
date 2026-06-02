//! Job role trait contract.
//!
//! Implement this trait when a one-shot job should be adapted into the
//! supervisor task factory runtime.

use crate::role::context::job::JobContext;
use crate::role::result::job::JobResult;
use std::future::Future;

/// Explicit lifecycle contract for a job role.
pub trait JobRole: Send + 'static {
    /// Runs initialization before the job body starts.
    ///
    /// # Arguments
    ///
    /// - `ctx`: Job-specific context for readiness and heartbeat reporting.
    ///
    /// # Returns
    ///
    /// Returns a future that resolves when initialization succeeds or fails.
    fn init<'a>(
        &'a mut self,
        ctx: &'a JobContext,
    ) -> impl Future<Output = JobResult<()>> + Send + 'a {
        async move {
            let _ = ctx;
            Ok(())
        }
    }

    /// Runs the one-shot job body.
    ///
    /// # Arguments
    ///
    /// - `ctx`: Job-specific context for cancellation observation.
    ///
    /// # Returns
    ///
    /// Returns a future that resolves when the job exits.
    fn run<'a>(
        &'a mut self,
        ctx: &'a JobContext,
    ) -> impl Future<Output = JobResult<()>> + Send + 'a;

    /// Runs the job completion hook after the body succeeds.
    ///
    /// # Arguments
    ///
    /// - `ctx`: Job-specific context for final completion work.
    ///
    /// # Returns
    ///
    /// Returns a future that resolves when completion succeeds or fails.
    fn complete<'a>(
        &'a mut self,
        ctx: &'a JobContext,
    ) -> impl Future<Output = JobResult<()>> + Send + 'a {
        async move {
            let _ = ctx;
            Ok(())
        }
    }
}
