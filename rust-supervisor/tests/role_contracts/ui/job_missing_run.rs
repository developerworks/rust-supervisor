//! Job macro missing-run compile-fail fixture.
//!
//! The fixture intentionally omits the required `run` lifecycle method.

#![allow(dead_code, unused_imports)]

use rust_supervisor::role::context::job::JobContext;
use rust_supervisor::role::result::job::JobResult;
use rust_supervisor_macros::job;

/// Job fixture that intentionally misses the required run method.
struct MissingRunJob;

#[job(id = "missing-run-job", name = "Missing Run Job")]
impl MissingRunJob {
    /// Defines initialization but not the required job body.
    async fn init(&mut self, ctx: &JobContext) -> JobResult<()> {
        let _ = ctx;
        Ok(())
    }
}

/// Keeps the fixture as a complete binary crate.
fn main() {}
