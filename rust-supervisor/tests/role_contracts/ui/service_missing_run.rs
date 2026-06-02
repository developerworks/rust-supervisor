//! Service macro missing-run compile-fail fixture.
//!
//! The fixture intentionally omits the required `run` lifecycle method.

#![allow(dead_code, unused_imports)]

use rust_supervisor::role::context::service::ServiceContext;
use rust_supervisor::role::result::service::ServiceResult;
use rust_supervisor_macros::service;

/// Service fixture that intentionally misses the required run method.
struct MissingRunService;

#[service(id = "missing-run-service", name = "Missing Run Service")]
impl MissingRunService {
    /// Defines initialization but not the required service body.
    async fn init(&mut self, ctx: &ServiceContext) -> ServiceResult<()> {
        let _ = ctx;
        Ok(())
    }
}

/// Keeps the fixture as a complete binary crate.
fn main() {}
