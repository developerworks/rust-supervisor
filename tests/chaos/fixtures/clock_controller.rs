//! Clock controller fixture.
//!
//! Provides `FixtureClockController` that simulates clock step-backward
//! scenarios. Since Rust's `Instant` is CLOCK_MONOTONIC and cannot be
//! rolled back via software, this fixture records an offset and test
//! code asserts that sliding window budgets are not distorted by
//! wall-clock changes.
//!
//! Design decision (research.md §2.9): we do NOT inject a time-source
//! trait into production code. Instead, assertions verify that
//! `std::time::Instant`-based components behave correctly under
//! simulated wall-clock step-back.

use std::time::Duration;

/// A fixture that simulates a clock step-backward event.
#[derive(Debug, Clone, Default)]
pub struct FixtureClockController {
    /// Offset recorded when `step_backward` is called.
    pub step_back_offset: Option<Duration>,
}

impl FixtureClockController {
    /// Creates a new clock controller.
    pub fn new() -> Self {
        Self::default()
    }

    /// Simulates stepping the clock backward by the given duration.
    ///
    /// This records the offset; the actual monotonic clock is unaffected.
    /// Test code should use this offset to verify that components using
    /// `std::time::Instant` (e.g., `RestartBudgetTracker`, `FailureWindow`)
    /// are not distorted by the simulated wall-clock change.
    pub fn step_backward(&mut self, duration: Duration) {
        self.step_back_offset = Some(duration);
    }

    /// Returns the current offset, if any.
    pub fn offset(&self) -> Option<Duration> {
        self.step_back_offset
    }

    /// Resets the offset to None.
    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.step_back_offset = None;
    }
}
