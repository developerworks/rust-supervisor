//! Worker role template entry.
//!
//! This module owns a lightweight wrapper that turns a [`WorkerRole`] value
//! into the existing adapter and child specification pipeline.

use crate::error::types::SupervisorError;
use crate::id::types::ChildId;
use crate::role::adapter::worker::WorkerRoleAdapter;
use crate::role::traits::worker::WorkerRole;
use crate::spec::child::{ChildSpec, TaskKind};
use crate::spec::child_builder::ChildSpecBuilder;
use std::sync::Arc;

/// Lightweight template wrapper for worker roles.
pub struct WorkerTemplate<T>
where
    T: WorkerRole,
{
    /// Wrapped worker role instance.
    inner: T,
}

impl<T> WorkerTemplate<T>
where
    T: WorkerRole,
{
    /// Creates a worker template around an inner role.
    ///
    /// # Arguments
    ///
    /// - `inner`: Worker role instance that owns user lifecycle state.
    ///
    /// # Returns
    ///
    /// Returns a [`WorkerTemplate`] that can expose the role, create an
    /// adapter, or build a child specification.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::role::context::worker::WorkerContext;
    /// use rust_supervisor::role::result::worker::WorkerResult;
    /// use rust_supervisor::role::templates::worker::WorkerTemplate;
    /// use rust_supervisor::role::traits::worker::WorkerRole;
    ///
    /// struct ExampleWorker;
    ///
    /// impl WorkerRole for ExampleWorker {
    ///     async fn work(&mut self, ctx: &WorkerContext) -> WorkerResult<()> {
    ///         let _ = ctx;
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let template = WorkerTemplate::new(ExampleWorker);
    /// let _worker = template.into_inner();
    /// ```
    pub fn new(inner: T) -> Self {
        Self { inner }
    }

    /// Borrows the wrapped worker role.
    ///
    /// # Arguments
    ///
    /// This function has no named arguments.
    ///
    /// # Returns
    ///
    /// Returns a shared reference to the inner worker role.
    pub fn inner(&self) -> &T {
        &self.inner
    }

    /// Mutably borrows the wrapped worker role.
    ///
    /// # Arguments
    ///
    /// This function has no named arguments.
    ///
    /// # Returns
    ///
    /// Returns a mutable reference to the inner worker role.
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    /// Consumes the template and returns the wrapped worker role.
    ///
    /// # Arguments
    ///
    /// This function has no named arguments beyond the consumed template.
    ///
    /// # Returns
    ///
    /// Returns the inner worker role.
    pub fn into_inner(self) -> T {
        self.inner
    }

    /// Consumes the template and creates a worker role adapter.
    ///
    /// # Arguments
    ///
    /// This function has no named arguments beyond the consumed template.
    ///
    /// # Returns
    ///
    /// Returns a [`WorkerRoleAdapter`] that implements the task factory
    /// service contract.
    pub fn adapter(self) -> WorkerRoleAdapter<T> {
        WorkerRoleAdapter::new(self.inner)
    }

    /// Consumes the template and builds a worker child specification.
    ///
    /// # Arguments
    ///
    /// - `id`: Stable child identifier.
    /// - `name`: Human-readable child name.
    ///
    /// # Returns
    ///
    /// Returns a validated [`ChildSpec`] that runs the worker through
    /// [`TaskKind::AsyncWorker`].
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::id::types::ChildId;
    /// use rust_supervisor::role::context::worker::WorkerContext;
    /// use rust_supervisor::role::result::worker::WorkerResult;
    /// use rust_supervisor::role::templates::worker::WorkerTemplate;
    /// use rust_supervisor::role::traits::worker::WorkerRole;
    ///
    /// # fn example() -> Result<(), rust_supervisor::error::types::SupervisorError> {
    /// struct ExampleWorker;
    ///
    /// impl WorkerRole for ExampleWorker {
    ///     async fn work(&mut self, ctx: &WorkerContext) -> WorkerResult<()> {
    ///         let _ = ctx;
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let spec = WorkerTemplate::new(ExampleWorker)
    ///     .child_spec(ChildId::new("worker"), "worker")?;
    /// assert_eq!(spec.name, "worker");
    /// # Ok(())
    /// # }
    /// ```
    pub fn child_spec(
        self,
        id: ChildId,
        name: impl Into<String>,
    ) -> Result<ChildSpec, SupervisorError> {
        ChildSpecBuilder::worker(id, name, TaskKind::AsyncWorker, Arc::new(self.adapter())).build()
    }
}
