//! Test support utilities for integration tests.
//!
//! This module contains factory, time, and child spawning helpers used by
//! integration tests. Downstream crate users should treat this surface as
//! an internal testing API.

pub mod assertions;
pub mod child_spawn;
pub mod factory;
pub mod test_time;
