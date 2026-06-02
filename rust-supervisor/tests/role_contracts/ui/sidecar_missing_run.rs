//! Sidecar macro missing-run compile-fail fixture.
//!
//! The fixture intentionally omits the required `run` lifecycle method.

#![allow(dead_code, unused_imports)]

use rust_supervisor::role::context::sidecar::SidecarContext;
use rust_supervisor::role::result::sidecar::SidecarResult;
use rust_supervisor_macros::sidecar;

/// Sidecar fixture that intentionally misses the required run method.
struct MissingRunSidecar;

#[sidecar(
    id = "missing-run-sidecar",
    name = "Missing Run Sidecar",
    primary = "primary-service"
)]
impl MissingRunSidecar {
    /// Defines initialization but not the required sidecar body.
    async fn init(&mut self, ctx: &SidecarContext) -> SidecarResult<()> {
        let _ = ctx;
        Ok(())
    }
}

/// Keeps the fixture as a complete binary crate.
fn main() {}
