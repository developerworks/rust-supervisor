//! Platform-conditional compilation declarations.
//!
//! This module documents the Unix-only strategy adopted by the project. The
//! core supervision library compiles on all Rust targets, but the dashboard
//! IPC subsystem requires Unix Domain Sockets and is gated behind
//! `#[cfg(unix)]` at the crate root (`src/lib.rs`) and at the dashboard
//! module level (`src/dashboard/mod.rs`).
//!
//! No Cargo feature gate is used — Rust's built-in `#[cfg(unix)]` provides
//! compiler-enforced safety that prevents dashboard IPC code from appearing
//! in non-Unix builds.

/// Confirms the crate compiles in a Unix environment.
///
/// This constant exists only as a compile-time assertion. It is `true` on
/// all Unix targets and absent on non-Unix targets.
#[cfg(unix)]
pub const UNIX_PLATFORM: bool = true;
