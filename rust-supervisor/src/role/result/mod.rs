//! Role result and error types.
//!
//! Each role owns its public result alias while shared lifecycle diagnostics stay
//! aligned through a common phase label.

pub mod job;
pub mod service;
pub mod sidecar;
pub mod supervisor;
pub mod worker;
