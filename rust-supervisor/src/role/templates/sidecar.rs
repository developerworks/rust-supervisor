//! Sidecar role template entry.
//!
//! This module provides a small owned template that can turn a [`SidecarRole`]
//! into a [`SidecarRoleAdapter`] or a validated sidecar [`ChildSpec`].

use crate::error::types::SupervisorError;
use crate::id::types::ChildId;
use crate::policy::task_role_defaults::SidecarConfig;
use crate::role::adapter::sidecar::SidecarRoleAdapter;
use crate::role::traits::sidecar::SidecarRole;
use crate::spec::child::{ChildSpec, TaskKind};
use crate::spec::child_builder::ChildSpecBuilder;
use std::sync::Arc;

/// Lightweight owned template for a sidecar role.
///
/// The template keeps the role instance and the primary child identifier
/// together until callers request an adapter or a child specification.
pub struct SidecarTemplate<T>
where
    T: SidecarRole,
{
    /// Sidecar role instance that owns user lifecycle state.
    inner: T,
    /// Stable child identifier for the primary child linked to the sidecar.
    primary_child_id: ChildId,
}

impl<T> SidecarTemplate<T>
where
    T: SidecarRole,
{
    /// Creates a sidecar template.
    ///
    /// # Arguments
    ///
    /// - `inner`: Sidecar role instance that owns user lifecycle state.
    /// - `primary_child_id`: Stable child identifier for the primary child.
    ///
    /// # Returns
    ///
    /// Returns a template that can create a sidecar adapter or child specification.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::id::types::ChildId;
    /// use rust_supervisor::role::context::sidecar::SidecarContext;
    /// use rust_supervisor::role::result::sidecar::SidecarResult;
    /// use rust_supervisor::role::templates::sidecar::SidecarTemplate;
    /// use rust_supervisor::role::traits::sidecar::SidecarRole;
    ///
    /// struct ExampleSidecar;
    ///
    /// impl SidecarRole for ExampleSidecar {
    ///     async fn run(&mut self, ctx: &SidecarContext) -> SidecarResult<()> {
    ///         let _ = ctx;
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let template = SidecarTemplate::new(
    ///     ExampleSidecar,
    ///     ChildId::new("primary"),
    /// );
    /// assert_eq!(template.primary_child_id().value.as_str(), "primary");
    /// ```
    pub fn new(inner: T, primary_child_id: ChildId) -> Self {
        Self {
            inner,
            primary_child_id,
        }
    }

    /// Returns a shared reference to the inner sidecar role.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the stored sidecar role by shared reference.
    pub fn inner(&self) -> &T {
        &self.inner
    }

    /// Returns an exclusive reference to the inner sidecar role.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the stored sidecar role by exclusive reference.
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    /// Returns the primary child identifier.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the stable child identifier for the primary child.
    pub fn primary_child_id(&self) -> &ChildId {
        &self.primary_child_id
    }

    /// Consumes the template and returns the inner sidecar role.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the stored sidecar role instance.
    pub fn into_inner(self) -> T {
        self.inner
    }

    /// Consumes the template and returns a sidecar role adapter.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns an adapter that implements the task factory service contract.
    pub fn adapter(self) -> SidecarRoleAdapter<T> {
        SidecarRoleAdapter::new(self.inner, self.primary_child_id)
    }

    /// Consumes the template and builds a sidecar child specification.
    ///
    /// # Arguments
    ///
    /// - `id`: Stable child identifier for the sidecar child.
    /// - `name`: Human-readable child name for the sidecar child.
    ///
    /// # Returns
    ///
    /// Returns a validated [`ChildSpec`] configured as an async sidecar worker.
    ///
    /// # Errors
    ///
    /// Returns [`SupervisorError`] when child specification validation fails.
    pub fn child_spec(
        self,
        id: ChildId,
        name: impl Into<String>,
    ) -> Result<ChildSpec, SupervisorError> {
        let primary = self.primary_child_id.clone();
        ChildSpecBuilder::sidecar(
            id,
            name,
            TaskKind::AsyncWorker,
            Arc::new(self.adapter()),
            SidecarConfig::new(primary, true),
        )
        .build()
    }
}
