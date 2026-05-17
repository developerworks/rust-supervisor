//! IPC (Inter-Process Communication) security modules.
//!
//! This module tree implements the nine IPC security control points
//! (C1-C9) for the dashboard Unix domain socket IPC channel.
//! The entire subtree is gated behind `#[cfg(unix)]` because it depends
//! on Unix Domain Sockets and peer credential syscalls.

pub mod security;
