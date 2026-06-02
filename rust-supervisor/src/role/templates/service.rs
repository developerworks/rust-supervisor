//! Lightweight service role template entry.
//!
//! The template keeps a [`ServiceRole`] value available for direct access and
//! can consume it into a [`ServiceRoleAdapter`] or a ready child specification.

use crate::error::types::SupervisorError;
use crate::id::types::ChildId;
use crate::role::adapter::service::ServiceRoleAdapter;
use crate::role::traits::service::ServiceRole;
use crate::spec::child::{ChildSpec, TaskKind};
use crate::spec::child_builder::ChildSpecBuilder;
use std::sync::Arc;

/// Lightweight template that owns one service role instance.
pub struct ServiceTemplate<T>
where
    T: ServiceRole,
{
    /// Owned service role value used by adapter and child specification builders.
    inner: T,
}

impl<T> ServiceTemplate<T>
where
    T: ServiceRole,
{
    /// Creates a service template from a service role value.
    ///
    /// # Arguments
    ///
    /// - `inner`: Service role instance that will later be adapted or inspected.
    ///
    /// # Returns
    ///
    /// Returns a [`ServiceTemplate`] that owns the provided role value.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::role::context::service::ServiceContext;
    /// use rust_supervisor::role::result::service::ServiceResult;
    /// use rust_supervisor::role::templates::service::ServiceTemplate;
    /// use rust_supervisor::role::traits::service::ServiceRole;
    ///
    /// struct ExampleService {
    ///     runs: usize,
    /// }
    ///
    /// impl ServiceRole for ExampleService {
    ///     async fn run(&mut self, ctx: &ServiceContext) -> ServiceResult<()> {
    ///         let _ = ctx;
    ///         self.runs += 1;
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let template = ServiceTemplate::new(ExampleService { runs: 0 });
    /// assert_eq!(template.inner().runs, 0);
    /// ```
    pub fn new(inner: T) -> Self {
        Self { inner }
    }

    /// Returns a shared reference to the owned service role.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a shared reference that does not consume the template.
    pub fn inner(&self) -> &T {
        &self.inner
    }

    /// Returns an exclusive reference to the owned service role.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns an exclusive reference that allows caller-side role updates.
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    /// Consumes the template and returns the owned service role.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the original service role value.
    pub fn into_inner(self) -> T {
        self.inner
    }

    /// Consumes the template and creates a service role adapter.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a [`ServiceRoleAdapter`] that implements the task factory
    /// service contract for the owned role value.
    pub fn adapter(self) -> ServiceRoleAdapter<T> {
        ServiceRoleAdapter::new(self.inner)
    }

    /// Consumes the template and creates a service child specification.
    ///
    /// # Arguments
    ///
    /// - `id`: Stable child identifier assigned to the generated child.
    /// - `name`: Human-readable child name stored in the generated child.
    ///
    /// # Returns
    ///
    /// Returns a validated [`ChildSpec`] with service role defaults and an async
    /// worker task kind.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::id::types::ChildId;
    /// use rust_supervisor::role::context::service::ServiceContext;
    /// use rust_supervisor::role::result::service::ServiceResult;
    /// use rust_supervisor::role::templates::service::ServiceTemplate;
    /// use rust_supervisor::role::traits::service::ServiceRole;
    /// use rust_supervisor::spec::child::TaskKind;
    ///
    /// struct ExampleService;
    ///
    /// impl ServiceRole for ExampleService {
    ///     async fn run(&mut self, ctx: &ServiceContext) -> ServiceResult<()> {
    ///         let _ = ctx;
    ///         Ok(())
    ///     }
    /// }
    ///
    /// # fn example() -> Result<(), rust_supervisor::error::types::SupervisorError> {
    /// let spec = ServiceTemplate::new(ExampleService)
    ///     .child_spec(ChildId::new("api-service"), "API Service")?;
    /// assert_eq!(spec.kind, TaskKind::AsyncWorker);
    /// # Ok(())
    /// # }
    /// ```
    pub fn child_spec(
        self,
        id: ChildId,
        name: impl Into<String>,
    ) -> Result<ChildSpec, SupervisorError> {
        ChildSpecBuilder::service(id, name, TaskKind::AsyncWorker, Arc::new(self.adapter())).build()
    }
}
