//! Exit handler abstraction for testable process termination.
//!
//! The default implementation calls `std::process::exit(1)` as a last resort
//! when orphaned tasks exceed the configured threshold. Tests can swap in a
//! stub that records the exit request instead of terminating the process.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Abstraction for process exit, allowing tests to observe exit requests
/// without actually terminating the process.
pub trait ExitHandler: Send + Sync + std::fmt::Debug {
    /// Terminates the process (or records the request in test mode).
    fn exit(&self, code: i32);
}

/// Default exit handler that calls `std::process::exit(code)`.
#[derive(Debug, Clone, Copy)]
pub struct DefaultExitHandler;

impl ExitHandler for DefaultExitHandler {
    /// Terminates the current process with the provided code.
    fn exit(&self, code: i32) {
        std::process::exit(code);
    }
}

/// Test-friendly exit handler that records a flag instead of terminating.
#[derive(Debug, Clone)]
pub struct TestExitHandler {
    /// Set to `true` when `exit()` was called.
    pub called: Arc<AtomicBool>,
    /// Captured exit code.
    pub exit_code: Arc<std::sync::Mutex<Option<i32>>>,
}

impl TestExitHandler {
    /// Creates a new test exit handler with `called = false`.
    pub fn new() -> Self {
        Self {
            called: Arc::new(AtomicBool::new(false)),
            exit_code: Arc::new(std::sync::Mutex::new(None)),
        }
    }

    /// Returns `true` when `exit()` was called since the last reset.
    pub fn was_called(&self) -> bool {
        self.called.load(Ordering::SeqCst)
    }

    /// Returns the exit code if `exit()` was called.
    pub fn last_exit_code(&self) -> Option<i32> {
        *self.exit_code.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Resets the recorded state.
    pub fn reset(&self) {
        self.called.store(false, Ordering::SeqCst);
        *self.exit_code.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }
}

impl ExitHandler for TestExitHandler {
    /// Records the requested exit code without terminating the process.
    fn exit(&self, code: i32) {
        self.called.store(true, Ordering::SeqCst);
        *self.exit_code.lock().unwrap_or_else(|e| e.into_inner()) = Some(code);
        // Do NOT call std::process::exit — let the test continue.
    }
}

impl Default for TestExitHandler {
    /// Creates a default test exit handler.
    fn default() -> Self {
        Self::new()
    }
}
