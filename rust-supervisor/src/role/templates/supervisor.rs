//! Supervisor role template entry.
//!
//! This module provides a lightweight owned template that can turn a
//! [`SupervisorRole`] into a [`SupervisorRoleAdapter`] or a validated supervisor
//! child specification backed by an async worker task.

use crate::error::types::SupervisorError;
use crate::id::types::ChildId;
use crate::policy::task_role_defaults::TaskRole;
use crate::role::adapter::supervisor::SupervisorRoleAdapter;
use crate::role::traits::supervisor::SupervisorRole;
use crate::spec::child::{ChildSpec, Criticality, TaskKind};
use crate::spec::child_builder::ChildSpecBuilder;
use std::sync::Arc;

/// Lightweight template that owns one supervisor role instance.
pub struct SupervisorTemplate<T>
where
    T: SupervisorRole,
{
    /// Owned supervisor role value used by adapter and child specification builders.
    inner: T,
}

impl<T> SupervisorTemplate<T>
where
    T: SupervisorRole,
{
    /// Creates a supervisor template from a supervisor role value.
    ///
    /// # Arguments
    ///
    /// - `inner`: Supervisor role instance that owns nested supervisor state.
    ///
    /// # Returns
    ///
    /// Returns a [`SupervisorTemplate`] that owns the provided role value.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::role::context::supervisor::SupervisorContext;
    /// use rust_supervisor::role::result::supervisor::SupervisorResult;
    /// use rust_supervisor::role::templates::supervisor::SupervisorTemplate;
    /// use rust_supervisor::role::traits::supervisor::SupervisorRole;
    /// use rust_supervisor::spec::supervisor::SupervisorSpec;
    ///
    /// struct ExampleSupervisor;
    ///
    /// impl SupervisorRole for ExampleSupervisor {
    ///     async fn build_tree(
    ///         &mut self,
    ///         ctx: &SupervisorContext,
    ///     ) -> SupervisorResult<SupervisorSpec> {
    ///         let _ = ctx;
    ///         Ok(SupervisorSpec::root(Vec::new()))
    ///     }
    /// }
    ///
    /// let template = SupervisorTemplate::new(ExampleSupervisor);
    /// let _supervisor = template.into_inner();
    /// ```
    pub fn new(inner: T) -> Self {
        Self { inner }
    }

    /// Returns a shared reference to the owned supervisor role.
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

    /// Returns an exclusive reference to the owned supervisor role.
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

    /// Consumes the template and returns the owned supervisor role.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the original supervisor role value.
    pub fn into_inner(self) -> T {
        self.inner
    }

    /// Consumes the template and creates a supervisor role adapter.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a [`SupervisorRoleAdapter`] that implements the task factory
    /// service contract for the owned role value.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::role::adapter::supervisor::SupervisorRoleAdapter;
    /// use rust_supervisor::role::context::supervisor::SupervisorContext;
    /// use rust_supervisor::role::result::supervisor::SupervisorResult;
    /// use rust_supervisor::role::templates::supervisor::SupervisorTemplate;
    /// use rust_supervisor::role::traits::supervisor::SupervisorRole;
    /// use rust_supervisor::spec::supervisor::SupervisorSpec;
    ///
    /// struct ExampleSupervisor;
    ///
    /// impl SupervisorRole for ExampleSupervisor {
    ///     async fn build_tree(
    ///         &mut self,
    ///         ctx: &SupervisorContext,
    ///     ) -> SupervisorResult<SupervisorSpec> {
    ///         let _ = ctx;
    ///         Ok(SupervisorSpec::root(Vec::new()))
    ///     }
    /// }
    ///
    /// let _adapter: SupervisorRoleAdapter<ExampleSupervisor> =
    ///     SupervisorTemplate::new(ExampleSupervisor).adapter();
    /// ```
    pub fn adapter(self) -> SupervisorRoleAdapter<T> {
        SupervisorRoleAdapter::new(self.inner)
    }

    /// Consumes the template and creates a supervisor child specification.
    ///
    /// # Arguments
    ///
    /// - `id`: Stable child identifier assigned to the generated child.
    /// - `name`: Human-readable child name stored in the generated child.
    ///
    /// # Returns
    ///
    /// Returns a validated [`ChildSpec`] with supervisor role classification,
    /// critical child settings, and an async worker task kind.
    ///
    /// # Errors
    ///
    /// Returns [`SupervisorError`] when child specification validation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::id::types::ChildId;
    /// use rust_supervisor::policy::task_role_defaults::TaskRole;
    /// use rust_supervisor::role::context::supervisor::SupervisorContext;
    /// use rust_supervisor::role::result::supervisor::SupervisorResult;
    /// use rust_supervisor::role::templates::supervisor::SupervisorTemplate;
    /// use rust_supervisor::role::traits::supervisor::SupervisorRole;
    /// use rust_supervisor::spec::child::{Criticality, TaskKind};
    /// use rust_supervisor::spec::supervisor::SupervisorSpec;
    ///
    /// struct ExampleSupervisor;
    ///
    /// impl SupervisorRole for ExampleSupervisor {
    ///     async fn build_tree(
    ///         &mut self,
    ///         ctx: &SupervisorContext,
    ///     ) -> SupervisorResult<SupervisorSpec> {
    ///         let _ = ctx;
    ///         Ok(SupervisorSpec::root(Vec::new()))
    ///     }
    /// }
    ///
    /// # fn example() -> Result<(), rust_supervisor::error::types::SupervisorError> {
    /// let spec = SupervisorTemplate::new(ExampleSupervisor)
    ///     .child_spec(ChildId::new("nested-supervisor"), "Nested Supervisor")?;
    /// assert_eq!(spec.kind, TaskKind::AsyncWorker);
    /// assert_eq!(spec.task_role, Some(TaskRole::Supervisor));
    /// assert_eq!(spec.criticality, Criticality::Critical);
    /// # Ok(())
    /// # }
    /// ```
    pub fn child_spec(
        self,
        id: ChildId,
        name: impl Into<String>,
    ) -> Result<ChildSpec, SupervisorError> {
        ChildSpecBuilder::worker(id, name, TaskKind::AsyncWorker, Arc::new(self.adapter()))
            .task_role(TaskRole::Supervisor)
            .criticality(Criticality::Critical)
            .build()
    }
}
