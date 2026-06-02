//! Role template entry modules.
//!
//! Templates are thin owned wrappers around explicit role contracts. They
//! create adapters and child specifications without adding lifecycle names.

pub mod job;
pub mod service;
pub mod sidecar;
pub mod supervisor;
pub mod worker;
