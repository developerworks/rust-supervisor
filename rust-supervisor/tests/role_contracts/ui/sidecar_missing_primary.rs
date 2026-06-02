//! Sidecar macro missing-primary compile-fail fixture.
//!
//! The fixture intentionally omits the required `primary` attribute argument.

#![allow(dead_code, unused_imports)]

use rust_supervisor::role::context::sidecar::SidecarContext;
use rust_supervisor::role::result::sidecar::SidecarResult;
use rust_supervisor_macros::sidecar;

/// Sidecar fixture that intentionally misses the required primary argument.
struct MissingPrimarySidecar;

#[sidecar(id = "missing-primary-sidecar", name = "Missing Primary Sidecar")]
impl MissingPrimarySidecar {
    /// Defines the required sidecar body.
    async fn run(&mut self, ctx: &SidecarContext) -> SidecarResult<()> {
        let _ = ctx;
        Ok(())
    }
}

/// Keeps the fixture as a complete binary crate.
fn main() {}
