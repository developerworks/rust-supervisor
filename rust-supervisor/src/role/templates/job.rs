//! Job role template entry point.
//!
//! This module keeps job role construction close to the role adapter while
//! leaving lifecycle behavior in [`JobRole`] implementations.

use crate::error::types::SupervisorError;
use crate::id::types::ChildId;
use crate::role::adapter::job::JobRoleAdapter;
use crate::role::traits::job::JobRole;
use crate::spec::child::{ChildSpec, TaskKind};
use crate::spec::child_builder::ChildSpecBuilder;
use std::sync::Arc;

/// Lightweight template that wraps one job role implementation.
pub struct JobTemplate<T>
where
    T: JobRole,
{
    /// Wrapped job role implementation.
    inner: T,
}

impl<T> JobTemplate<T>
where
    T: JobRole,
{
    /// Creates a job template from a role implementation.
    ///
    /// # Arguments
    ///
    /// - `inner`: Job role implementation stored by this template.
    ///
    /// # Returns
    ///
    /// Returns a template that can expose the inner role, create an adapter,
    /// or build a child specification.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::role::context::job::JobContext;
    /// use rust_supervisor::role::result::job::JobResult;
    /// use rust_supervisor::role::templates::job::JobTemplate;
    /// use rust_supervisor::role::traits::job::JobRole;
    ///
    /// struct ExampleJob;
    ///
    /// impl JobRole for ExampleJob {
    ///     async fn run(&mut self, ctx: &JobContext) -> JobResult<()> {
    ///         let _ = ctx;
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let _template = JobTemplate::new(ExampleJob);
    /// ```
    pub fn new(inner: T) -> Self {
        Self { inner }
    }

    /// Borrows the wrapped job role implementation.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a shared reference to the wrapped job role implementation.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::role::context::job::JobContext;
    /// use rust_supervisor::role::result::job::JobResult;
    /// use rust_supervisor::role::templates::job::JobTemplate;
    /// use rust_supervisor::role::traits::job::JobRole;
    ///
    /// struct ExampleJob {
    ///     value: usize,
    /// }
    ///
    /// impl JobRole for ExampleJob {
    ///     async fn run(&mut self, ctx: &JobContext) -> JobResult<()> {
    ///         let _ = ctx;
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let template = JobTemplate::new(ExampleJob { value: 7 });
    /// assert_eq!(template.inner().value, 7);
    /// ```
    pub fn inner(&self) -> &T {
        &self.inner
    }

    /// Mutably borrows the wrapped job role implementation.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a mutable reference to the wrapped job role implementation.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::role::context::job::JobContext;
    /// use rust_supervisor::role::result::job::JobResult;
    /// use rust_supervisor::role::templates::job::JobTemplate;
    /// use rust_supervisor::role::traits::job::JobRole;
    ///
    /// struct ExampleJob {
    ///     value: usize,
    /// }
    ///
    /// impl JobRole for ExampleJob {
    ///     async fn run(&mut self, ctx: &JobContext) -> JobResult<()> {
    ///         let _ = ctx;
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let mut template = JobTemplate::new(ExampleJob { value: 7 });
    /// template.inner_mut().value = 8;
    /// assert_eq!(template.inner().value, 8);
    /// ```
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    /// Consumes the template and returns the wrapped job role implementation.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the wrapped job role implementation.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::role::context::job::JobContext;
    /// use rust_supervisor::role::result::job::JobResult;
    /// use rust_supervisor::role::templates::job::JobTemplate;
    /// use rust_supervisor::role::traits::job::JobRole;
    ///
    /// struct ExampleJob;
    ///
    /// impl JobRole for ExampleJob {
    ///     async fn run(&mut self, ctx: &JobContext) -> JobResult<()> {
    ///         let _ = ctx;
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let _job = JobTemplate::new(ExampleJob).into_inner();
    /// ```
    pub fn into_inner(self) -> T {
        self.inner
    }

    /// Converts the template into a job role adapter.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns a [`JobRoleAdapter`] that can be registered as a task factory.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::role::adapter::job::JobRoleAdapter;
    /// use rust_supervisor::role::context::job::JobContext;
    /// use rust_supervisor::role::result::job::JobResult;
    /// use rust_supervisor::role::templates::job::JobTemplate;
    /// use rust_supervisor::role::traits::job::JobRole;
    ///
    /// struct ExampleJob;
    ///
    /// impl JobRole for ExampleJob {
    ///     async fn run(&mut self, ctx: &JobContext) -> JobResult<()> {
    ///         let _ = ctx;
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let _adapter: JobRoleAdapter<ExampleJob> = JobTemplate::new(ExampleJob).adapter();
    /// ```
    pub fn adapter(self) -> JobRoleAdapter<T> {
        JobRoleAdapter::new(self.inner)
    }

    /// Builds a child specification for the wrapped job role.
    ///
    /// # Arguments
    ///
    /// - `id`: Stable child identifier used by the supervisor tree.
    /// - `name`: Human-readable child name stored in the child specification.
    ///
    /// # Returns
    ///
    /// Returns a validated [`ChildSpec`] with job role defaults and an async
    /// worker task kind.
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
    /// use rust_supervisor::role::context::job::JobContext;
    /// use rust_supervisor::role::result::job::JobResult;
    /// use rust_supervisor::role::templates::job::JobTemplate;
    /// use rust_supervisor::role::traits::job::JobRole;
    /// use rust_supervisor::spec::child::TaskKind;
    ///
    /// struct ExampleJob;
    ///
    /// impl JobRole for ExampleJob {
    ///     async fn run(&mut self, ctx: &JobContext) -> JobResult<()> {
    ///         let _ = ctx;
    ///         Ok(())
    ///     }
    /// }
    ///
    /// # fn example() -> Result<(), rust_supervisor::error::types::SupervisorError> {
    /// let spec = JobTemplate::new(ExampleJob)
    ///     .child_spec(ChildId::new("daily-job"), "Daily Job")?;
    /// assert_eq!(spec.kind, TaskKind::AsyncWorker);
    /// assert_eq!(spec.task_role, Some(TaskRole::Job));
    /// # Ok(())
    /// # }
    /// ```
    pub fn child_spec(
        self,
        id: ChildId,
        name: impl Into<String>,
    ) -> Result<ChildSpec, SupervisorError> {
        ChildSpecBuilder::job(id, name, TaskKind::AsyncWorker, Arc::new(self.adapter())).build()
    }
}
