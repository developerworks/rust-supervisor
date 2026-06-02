//! Code generation modules for role contract macros.
//!
//! Expanders turn parsed role metadata and lifecycle impl blocks into Rust code
//! that targets runtime contracts in the main crate.

pub(crate) mod job;
pub(crate) mod service;
pub(crate) mod sidecar;
pub(crate) mod supervisor_role;
pub(crate) mod worker;
