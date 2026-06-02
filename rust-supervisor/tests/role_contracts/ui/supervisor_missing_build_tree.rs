//! Supervisor macro missing-build-tree compile-fail fixture.
//!
//! The fixture intentionally omits the required `build_tree` lifecycle method.

#![allow(dead_code, unused_imports)]

use rust_supervisor::control::handle::SupervisorHandle;
use rust_supervisor::role::context::supervisor::SupervisorContext;
use rust_supervisor::role::result::supervisor::SupervisorResult;
use rust_supervisor_macros::supervisor_role;

/// Supervisor fixture that intentionally misses the required build_tree method.
struct MissingBuildTreeSupervisor;

#[supervisor_role(
    id = "missing-build-tree-supervisor",
    name = "Missing Build Tree Supervisor"
)]
impl MissingBuildTreeSupervisor {
    /// Defines run but not the required nested tree builder.
    async fn run(
        &mut self,
        ctx: &SupervisorContext,
        handle: &SupervisorHandle,
    ) -> SupervisorResult<()> {
        let _ = (ctx, handle);
        Ok(())
    }
}

/// Keeps the fixture as a complete binary crate.
fn main() {}
