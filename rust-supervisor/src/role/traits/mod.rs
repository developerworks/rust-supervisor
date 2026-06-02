//! Role trait contracts.
//!
//! Traits are the explicit contract layer that macros target and users can
//! implement directly when they do not want macro-generated code.

pub mod job;
pub mod service;
pub mod sidecar;
pub mod supervisor;
pub mod worker;
