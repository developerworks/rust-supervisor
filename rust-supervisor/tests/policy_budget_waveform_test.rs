//! Restart budget waveform tests.
//!
//! Validates budget limiting under fast failure scenarios.

use rust_supervisor::policy::budget::{BudgetVerdict, RestartBudgetConfig, RestartBudgetTracker};
use std::time::Duration;

/// Simulates 10_000 fast failures across 300s and asserts the tracker
/// exhausts tokens and stays bounded by the effective restart rate.
///
/// Budget curve: window=60s, max_burst=10, recovery=0.5/s
/// max_effective_rpm = (10/60 + 0.5) * 60 = 40 RPM
#[test]
fn test_budget_limits_effective_restart_rate() {
    let config = RestartBudgetConfig::new(Duration::from_secs(60), 10, 0.5);
    let now = 1_000_000_000_000u128;
    let mut tracker = RestartBudgetTracker::new(config, now);

    let mut granted = 0u64;
    let mut exhausted = 0u64;

    // Inject 10_000 failures over 300s at 30ms intervals
    for i in 0..10_000u128 {
        let t = now + (i * 30_000_000);
        match tracker.try_consume(t) {
            BudgetVerdict::Granted => granted += 1,
            BudgetVerdict::Exhausted { .. } => exhausted += 1,
        }
    }

    // Over 300s, max grants <= 40 RPM * 5 min = 200 * 1.05 ~= 210
    let max_expected = 210u64;
    assert!(
        granted <= max_expected,
        "granted {granted} exceeds 105% upper bound {max_expected}"
    );
    assert!(
        exhausted > 100,
        "expected significant budget exhaustion, got {exhausted}"
    );
}

/// Tokens recover over time after exhaustion.
/// Boundary: when 1 token is recovered, next try_consume() passes immediately.
#[test]
fn test_budget_recovers_tokens_over_time() {
    let config = RestartBudgetConfig::new(Duration::from_secs(10), 2, 1.0);
    let start = 1_000_000_000_000u128;
    let mut tracker = RestartBudgetTracker::new(config, start);

    // Exhaust both tokens
    assert!(matches!(tracker.try_consume(start), BudgetVerdict::Granted));
    assert!(matches!(
        tracker.try_consume(start + 1),
        BudgetVerdict::Granted
    ));
    assert!(matches!(
        tracker.try_consume(start + 2),
        BudgetVerdict::Exhausted { .. }
    ));

    // Wait 1.5s — should recover >1 token
    let later = start + 1_500_000_000;
    assert!(
        matches!(tracker.try_consume(later), BudgetVerdict::Granted),
        "token should be granted immediately after recovery"
    );
    assert!(matches!(
        tracker.try_consume(later + 1),
        BudgetVerdict::Exhausted { .. }
    ));

    // After 10s, tokens should recover close to max_burst=2
    // (consuming one leaves ~1 token remaining)
    let much_later = start + 10_000_000_000;
    let _ = tracker.try_consume(much_later); // trigger refill, consume 1
    let tokens = tracker.current_tokens(much_later + 1);
    assert!(
        tokens >= 0.9,
        "tokens should be >= 1 after recovery and one consume, got {tokens}"
    );
}
