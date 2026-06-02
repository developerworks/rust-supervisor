//! Soak test suite entry point.
//!
//! Run with: `cargo test --test soak_suite -- --ignored`
//!
//! Executes a 24-hour soak test (default) that generates a SoakReport
//! Markdown file. Duration can be shortened via `SOAK_DURATION_MINUTES`
//! environment variable for development verification.

mod soak;

use std::time::Duration;

use soak::SoakRuntime;

/// 24-hour soak test.
///
/// Override duration with `SOAK_DURATION_MINUTES` env var for testing.
#[test]
#[ignore]
fn soak_24h() {
    let duration_minutes: u64 = std::env::var("SOAK_DURATION_MINUTES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(24 * 60);

    let duration = Duration::from_secs(duration_minutes * 60);
    let mut runtime = SoakRuntime::new(duration);
    let report = runtime.run_blocking();

    // Verify shutdown success ratio meets threshold.
    let passed = report.shutdown_success_ratio >= 0.99;
    assert!(
        passed,
        "shutdown_success_ratio {} < 0.99",
        report.shutdown_success_ratio
    );

    println!("{}", report.to_markdown());
}
