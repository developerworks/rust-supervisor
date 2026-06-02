//! Role-specific context wrappers.
//!
//! Context wrappers expose the narrow capability set each role needs while the
//! runtime keeps owning the complete task context.

pub mod job;
pub mod service;
pub mod sidecar;
pub mod supervisor;
pub mod worker;
