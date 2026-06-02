//! Sidecar role trait contract.
//!
//! Implement this trait when an auxiliary sidecar should be adapted into the
//! supervisor task factory runtime while keeping the primary child identifier
//! visible.

use crate::role::context::sidecar::SidecarContext;
use crate::role::result::sidecar::SidecarResult;
use std::future::Future;

/// Explicit lifecycle contract for a sidecar role.
pub trait SidecarRole: Send + 'static {
    /// Runs initialization before the sidecar body starts.
    ///
    /// # Arguments
    ///
    /// - `ctx`: Sidecar-specific context for readiness, heartbeat, and primary child access.
    ///
    /// # Returns
    ///
    /// Returns a future that resolves when initialization succeeds or fails.
    fn init<'a>(
        &'a mut self,
        ctx: &'a SidecarContext,
    ) -> impl Future<Output = SidecarResult<()>> + Send + 'a {
        async move {
            let _ = ctx;
            Ok(())
        }
    }

    /// Runs the sidecar body.
    ///
    /// # Arguments
    ///
    /// - `ctx`: Sidecar-specific context for shutdown and primary child observation.
    ///
    /// # Returns
    ///
    /// Returns a future that resolves when the sidecar exits.
    fn run<'a>(
        &'a mut self,
        ctx: &'a SidecarContext,
    ) -> impl Future<Output = SidecarResult<()>> + Send + 'a;

    /// Runs the sidecar shutdown hook.
    ///
    /// # Arguments
    ///
    /// - `ctx`: Sidecar-specific context for final shutdown work.
    ///
    /// # Returns
    ///
    /// Returns a future that resolves when shutdown succeeds or fails.
    fn shutdown<'a>(
        &'a mut self,
        ctx: &'a SidecarContext,
    ) -> impl Future<Output = SidecarResult<()>> + Send + 'a {
        async move {
            let _ = ctx;
            Ok(())
        }
    }
}
