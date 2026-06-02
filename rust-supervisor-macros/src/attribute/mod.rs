//! Attribute macro dispatch modules.
//!
//! Each file owns one public procedural macro entry shape and delegates parsing
//! plus expansion to smaller modules.

pub(crate) mod job;
pub(crate) mod service;
pub(crate) mod sidecar;
pub(crate) mod supervisor_role;
pub(crate) mod worker;
