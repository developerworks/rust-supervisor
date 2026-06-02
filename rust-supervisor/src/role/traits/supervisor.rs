//! Supervisor role trait contract.
//!
//! Implement this trait when a nested supervisor tree should be adapted into
//! the supervisor task factory runtime.

use crate::control::handle::SupervisorHandle;
use crate::role::context::supervisor::SupervisorContext;
use crate::role::lifecycle::RoleLifecyclePhase;
use crate::role::result::supervisor::{SupervisorResult, SupervisorRoleError};
use crate::spec::supervisor::SupervisorSpec;
use std::future::Future;

/// Explicit lifecycle contract for a nested supervisor role.
pub trait SupervisorRole: Send + 'static {
    /// Builds the nested supervisor tree specification.
    ///
    /// # Arguments
    ///
    /// - `ctx`: Supervisor-specific context for identity, readiness, and heartbeat reporting.
    ///
    /// # Returns
    ///
    /// Returns a future that resolves to the nested [`SupervisorSpec`].
    fn build_tree<'a>(
        &'a mut self,
        ctx: &'a SupervisorContext,
    ) -> impl Future<Output = SupervisorResult<SupervisorSpec>> + Send + 'a;

    /// Runs the supervisor role while the nested supervisor is alive.
    ///
    /// # Arguments
    ///
    /// - `ctx`: Supervisor-specific context for shutdown observation.
    /// - `handle`: Runtime handle for the nested supervisor.
    ///
    /// # Returns
    ///
    /// Returns a future that resolves when the role body exits.
    fn run<'a>(
        &'a mut self,
        ctx: &'a SupervisorContext,
        handle: &'a SupervisorHandle,
    ) -> impl Future<Output = SupervisorResult<()>> + Send + 'a {
        async move {
            let _ = handle;
            ctx.wait_shutdown().await;
            Ok(())
        }
    }

    /// Runs the supervisor role shutdown hook.
    ///
    /// # Arguments
    ///
    /// - `ctx`: Supervisor-specific context for final shutdown work.
    /// - `handle`: Runtime handle for the nested supervisor.
    ///
    /// # Returns
    ///
    /// Returns a future that resolves when nested supervisor shutdown succeeds
    /// or fails.
    fn shutdown<'a>(
        &'a mut self,
        ctx: &'a SupervisorContext,
        handle: &'a SupervisorHandle,
    ) -> impl Future<Output = SupervisorResult<()>> + Send + 'a {
        async move {
            let shutdown_tree_result = handle
                .shutdown_tree("role-adapter", "nested supervisor shutdown")
                .await
                .map_err(|error| {
                    SupervisorRoleError::from_supervisor_error(
                        ctx.child_id().clone(),
                        RoleLifecyclePhase::Shutdown,
                        error,
                    )
                });
            let control_plane_result = handle
                .shutdown("role-adapter", "nested supervisor shutdown")
                .await
                .map(|_report| ())
                .map_err(|error| {
                    SupervisorRoleError::from_supervisor_error(
                        ctx.child_id().clone(),
                        RoleLifecyclePhase::Shutdown,
                        error,
                    )
                });
            let join_result = handle.join().await.map(|_report| ()).map_err(|error| {
                SupervisorRoleError::from_supervisor_error(
                    ctx.child_id().clone(),
                    RoleLifecyclePhase::Shutdown,
                    error,
                )
            });

            shutdown_tree_result?;
            control_plane_result?;
            join_result?;
            Ok(())
        }
    }
}
