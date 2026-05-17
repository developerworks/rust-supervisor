//! Target-side dashboard service modules.
//!
//! The dashboard IPC subsystem is only available on Unix platforms. The
//! entire module tree is gated behind `#[cfg(unix)]` so that the core
//! supervision library compiles on all Rust targets.

#[cfg(unix)]
pub mod config;
#[cfg(unix)]
pub mod diagnostics;
#[cfg(unix)]
pub mod error;
#[cfg(unix)]
pub mod events;
#[cfg(unix)]
pub mod ipc_server;
#[cfg(unix)]
pub mod model;
#[cfg(unix)]
pub mod protocol;
#[cfg(unix)]
pub mod registration;
#[cfg(unix)]
pub mod runtime;
#[cfg(unix)]
pub mod state;
