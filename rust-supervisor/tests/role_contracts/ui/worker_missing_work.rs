//! Worker macro missing-work compile-fail fixture.
//!
//! The fixture intentionally omits the required `work` lifecycle method.

#![allow(dead_code, unused_imports)]

use rust_supervisor::role::context::worker::WorkerContext;
use rust_supervisor::role::result::worker::WorkerResult;
use rust_supervisor_macros::worker;

/// Worker fixture that intentionally misses the required work method.
struct MissingWorkWorker;

#[worker(id = "missing-work-worker", name = "Missing Work Worker")]
impl MissingWorkWorker {
    /// Defines initialization but not the required worker body.
    async fn init(&mut self, ctx: &WorkerContext) -> WorkerResult<()> {
        let _ = ctx;
        Ok(())
    }
}

/// Keeps the fixture as a complete binary crate.
fn main() {}
