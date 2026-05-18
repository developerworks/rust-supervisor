//! Shared test fixtures for chaos scenarios.
//!
//! These fixtures inject controlled faults into the supervisor runtime
//! without modifying `src/` production code.

pub mod child_spawner;
pub mod clock_controller;
pub mod event_throttle;
pub mod ipc_stress;
pub mod runtime_probe;
