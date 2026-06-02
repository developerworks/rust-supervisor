//! Worker role trait contract.
//!
//! Implement this trait when a bounded background worker should be adapted into
//! the supervisor task factory runtime.

use crate::role::context::worker::WorkerContext;
use crate::role::result::worker::WorkerResult;
use std::future::Future;

/// Explicit lifecycle contract for a worker role.
pub trait WorkerRole: Send + 'static {
    /// Runs initialization before the worker body starts.
    ///
    /// # Arguments
    ///
    /// - `ctx`: Worker-specific context for readiness and heartbeat reporting.
    ///
    /// # Returns
    ///
    /// Returns a future that resolves when initialization succeeds or fails.
    fn init<'a>(
        &'a mut self,
        ctx: &'a WorkerContext,
    ) -> impl Future<Output = WorkerResult<()>> + Send + 'a {
        async move {
            let _ = ctx;
            Ok(())
        }
    }

    /// Runs the worker body.
    ///
    /// # Arguments
    ///
    /// - `ctx`: Worker-specific context for cancellation observation.
    ///
    /// # Returns
    ///
    /// Returns a future that resolves when the worker exits.
    fn work<'a>(
        &'a mut self,
        ctx: &'a WorkerContext,
    ) -> impl Future<Output = WorkerResult<()>> + Send + 'a;

    /// Runs the worker completion hook after successful work.
    ///
    /// # Arguments
    ///
    /// - `ctx`: Worker-specific context for final completion work.
    ///
    /// # Returns
    ///
    /// Returns a future that resolves when completion succeeds or fails.
    fn complete<'a>(
        &'a mut self,
        ctx: &'a WorkerContext,
    ) -> impl Future<Output = WorkerResult<()>> + Send + 'a {
        async move {
            let _ = ctx;
            Ok(())
        }
    }
}
