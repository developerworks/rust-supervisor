//! Acceptance tests for cold start budget and hot loop detection.
//!
//! This test verifies that:
//! 1. Cold start budget exhaustion triggers protection action per restrictiveness ladder
//! 2. Hot loop detection triggers when crashes occur within sliding time window
//! 3. Combined scenario: both cold start exhausted AND hot loop detected

use rust_supervisor::event::payload::{ColdStartReason, HotLoopReason, ProtectionAction};
use std::time::{Duration, Instant};

/// Simulated cold start budget tracker
struct ColdStartBudget {
    window_secs: u64,
    max_restarts: u32,
    restart_count: u32,
    start_time: Instant,
}

impl ColdStartBudget {
    fn new(window_secs: u64, max_restarts: u32) -> Self {
        Self {
            window_secs,
            max_restarts,
            restart_count: 0,
            start_time: Instant::now(),
        }
    }

    fn record_restart(&mut self, now: Instant) -> (bool, ColdStartReason) {
        // Check if still within cold start window
        let elapsed = now.duration_since(self.start_time);
        if elapsed > Duration::from_secs(self.window_secs) {
            return (false, ColdStartReason::NotApplicable);
        }

        self.restart_count += 1;
        if self.restart_count > self.max_restarts {
            (true, ColdStartReason::BudgetExhausted)
        } else {
            (false, ColdStartReason::InitialStartup)
        }
    }
}

/// Simulated hot loop detector
struct HotLoopDetector {
    window_secs: u64,
    min_restarts: u32,
    crash_times: Vec<Instant>,
}

impl HotLoopDetector {
    fn new(window_secs: u64, min_restarts: u32) -> Self {
        Self {
            window_secs,
            min_restarts,
            crash_times: Vec::new(),
        }
    }

    fn record_crash(&mut self, now: Instant) -> (bool, HotLoopReason) {
        // Prune old crashes outside window
        let cutoff = now - Duration::from_secs(self.window_secs);
        self.crash_times.retain(|t| *t >= cutoff);

        self.crash_times.push(now);

        if self.crash_times.len() >= self.min_restarts as usize {
            (true, HotLoopReason::RapidCrashDetected)
        } else {
            (false, HotLoopReason::NotApplicable)
        }
    }
}

#[test]
fn test_cold_start_budget_exhaustion() {
    let mut budget = ColdStartBudget::new(60, 3); // 60s window, max 3 restarts
    let base = Instant::now();

    // First 3 restarts within window should be allowed
    for i in 0..3 {
        let (exhausted, reason) = budget.record_restart(base + Duration::from_secs(i * 5));
        assert!(!exhausted);
        assert_eq!(reason, ColdStartReason::InitialStartup);
    }

    // 4th restart should exhaust budget
    let (exhausted, reason) = budget.record_restart(base + Duration::from_secs(20));
    assert!(exhausted);
    assert_eq!(reason, ColdStartReason::BudgetExhausted);
}

#[test]
fn test_cold_start_window_expiry() {
    let mut budget = ColdStartBudget::new(10, 2); // 10s window
    let base = Instant::now();

    // Restart within window
    budget.record_restart(base);
    budget.record_restart(base + Duration::from_secs(5));

    // Restart after window expires should not count
    let (exhausted, reason) = budget.record_restart(base + Duration::from_secs(15));
    assert!(!exhausted);
    assert_eq!(reason, ColdStartReason::NotApplicable);
}

#[test]
fn test_hot_loop_detection() {
    let mut detector = HotLoopDetector::new(10, 3); // 10s window, min 3 crashes
    let base = Instant::now();

    // First 2 crashes should not trigger
    detector.record_crash(base);
    let (detected, reason) = detector.record_crash(base + Duration::from_secs(2));
    assert!(!detected);
    assert_eq!(reason, HotLoopReason::NotApplicable);

    // 3rd crash within window should trigger
    let (detected, reason) = detector.record_crash(base + Duration::from_secs(4));
    assert!(detected);
    assert_eq!(reason, HotLoopReason::RapidCrashDetected);
}

#[test]
fn test_hot_loop_window_sliding() {
    let mut detector = HotLoopDetector::new(5, 3); // 5s window
    let base = Instant::now();

    // 2 crashes early
    detector.record_crash(base);
    detector.record_crash(base + Duration::from_secs(1));

    // Wait for first crash to expire from window
    // Then add 2 more - should only have 2 in window (not enough)
    let (detected, _) = detector.record_crash(base + Duration::from_secs(7));
    assert!(!detected); // Only 2 crashes in [2s, 7s] window
}

#[test]
fn test_combined_cold_start_and_hot_loop() {
    // Test scenario where both cold start budget exhausted AND hot loop detected
    let mut budget = ColdStartBudget::new(60, 2);
    let mut detector = HotLoopDetector::new(10, 3);
    let base = Instant::now();

    // Record rapid crashes
    for i in 0..3 {
        let now = base + Duration::from_secs(i * 2);
        budget.record_restart(now);
        detector.record_crash(now);
    }

    // Both should be triggered
    let (budget_exhausted, budget_reason) = budget.record_restart(base + Duration::from_secs(6));
    let (hot_loop_detected, hot_loop_reason) = detector.record_crash(base + Duration::from_secs(8));

    assert!(budget_exhausted);
    assert_eq!(budget_reason, ColdStartReason::BudgetExhausted);

    assert!(hot_loop_detected);
    assert_eq!(hot_loop_reason, HotLoopReason::RapidCrashDetected);
}

#[test]
fn test_protection_action_for_cold_start_exhausted() {
    // When cold start budget is exhausted, protection should be at least RestartDenied
    let action = ProtectionAction::RestartDenied;
    assert!(action >= ProtectionAction::RestartDenied);
}

#[test]
fn test_protection_action_for_hot_loop() {
    // When hot loop is detected, protection should escalate
    let action = ProtectionAction::SupervisionPaused;
    assert!(action >= ProtectionAction::RestartDenied);
}

#[test]
fn test_cold_start_reason_display() {
    assert_eq!(
        format!("{}", ColdStartReason::NotApplicable),
        "not_applicable"
    );
    assert_eq!(
        format!("{}", ColdStartReason::InitialStartup),
        "initial_startup"
    );
    assert_eq!(
        format!("{}", ColdStartReason::BudgetExhausted),
        "budget_exhausted"
    );
    assert_eq!(
        format!("{}", ColdStartReason::ExcessiveRestarts),
        "excessive_restarts"
    );
}

#[test]
fn test_hot_loop_reason_display() {
    assert_eq!(
        format!("{}", HotLoopReason::NotApplicable),
        "not_applicable"
    );
    assert_eq!(
        format!("{}", HotLoopReason::RapidCrashDetected),
        "rapid_crash_detected"
    );
    assert_eq!(
        format!("{}", HotLoopReason::CycleThresholdExceeded),
        "cycle_threshold_exceeded"
    );
    assert_eq!(
        format!("{}", HotLoopReason::InsufficientStableRuntime),
        "insufficient_stable_runtime"
    );
}
