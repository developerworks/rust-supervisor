//! Rust Supervisor provides a typed task supervision core.
//!
//! The crate keeps every public type in its owning top-level module. Users import
//! concrete items through absolute module paths such as
//! `rust_supervisor::runtime::supervisor::Supervisor`.

pub mod child_runner;
pub mod config;
pub mod control;
#[cfg(unix)]
pub mod dashboard;
pub mod error;
pub mod event;
pub mod health;
pub mod id;
#[cfg(unix)]
pub mod ipc;
pub mod journal;
pub mod observe;
pub mod platform;
pub mod policy;
pub mod readiness;
pub mod registry;
pub mod runtime;
pub mod shutdown;
pub mod spec;
pub mod state;
pub mod summary;
pub mod task;
pub mod test_support;
pub mod tree;
