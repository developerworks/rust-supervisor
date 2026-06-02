//! Role adapter implementations.
//!
//! Adapters are the only layer that translates role contracts into task factory
//! futures consumed by the runtime.

pub mod job;
pub mod service;
pub mod sidecar;
pub mod supervisor;
pub mod worker;
