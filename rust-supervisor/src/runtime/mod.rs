pub mod admission;
pub mod child_slot;
pub mod concurrent_gate;
pub mod control_loop;
pub mod lifecycle;
pub mod message;
pub mod pipeline;
pub mod shutdown;
pub mod shutdown_pipeline;
pub mod supervisor;
pub mod watchdog;

#[cfg(test)]
#[path = "tests/pipeline_test.rs"]
mod pipeline_test;
