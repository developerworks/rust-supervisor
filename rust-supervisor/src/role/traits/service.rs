//! Service role trait contract.
//!
//! Implement this trait when a long-running service should be adapted into the
//! supervisor task factory runtime.

use crate::role::context::service::ServiceContext;
use crate::role::result::service::ServiceResult;
use std::future::Future;

/// Explicit lifecycle contract for a service role.
pub trait ServiceRole: Send + 'static {
    /// Runs initialization before the service body starts.
    ///
    /// # Arguments
    ///
    /// - `ctx`: Service-specific context for readiness and heartbeat reporting.
    ///
    /// # Returns
    ///
    /// Returns a future that resolves when initialization succeeds or fails.
    fn init<'a>(
        &'a mut self,
        ctx: &'a ServiceContext,
    ) -> impl Future<Output = ServiceResult<()>> + Send + 'a {
        async move {
            let _ = ctx;
            Ok(())
        }
    }

    /// Runs the service body.
    ///
    /// # Arguments
    ///
    /// - `ctx`: Service-specific context for shutdown observation.
    ///
    /// # Returns
    ///
    /// Returns a future that resolves when the service exits.
    fn run<'a>(
        &'a mut self,
        ctx: &'a ServiceContext,
    ) -> impl Future<Output = ServiceResult<()>> + Send + 'a;

    /// Runs the service shutdown hook.
    ///
    /// # Arguments
    ///
    /// - `ctx`: Service-specific context for final shutdown work.
    ///
    /// # Returns
    ///
    /// Returns a future that resolves when shutdown succeeds or fails.
    fn shutdown<'a>(
        &'a mut self,
        ctx: &'a ServiceContext,
    ) -> impl Future<Output = ServiceResult<()>> + Send + 'a {
        async move {
            let _ = ctx;
            Ok(())
        }
    }
}
