//! Chaos suite test entry point.
//!
//! Run with: `cargo test --test chaos_suite -- --include-ignored`
//!
//! Executes all 11 chaos scenarios and outputs JSON verdicts to stdout.
//! If any scenario verdict has `passed: false`, the test fails.

mod chaos;

use chaos::scenarios::ScenarioRouter;
use chaos::verdict::{ScenarioVerdict, write_verdict};

/// Runs all chaos scenarios sequentially.
///
/// Each scenario is defined in `tests/chaos/scenarios/` with a
/// corresponding `run()` function that returns a `ScenarioVerdict`.
#[test]
#[ignore]
fn chaos_suite() {
    let router = ScenarioRouter::new();
    let verdicts = router.run_all();

    let all_passed = verdicts.iter().all(|v| v.passed);
    for v in &verdicts {
        write_verdict(v);
    }

    let summary = ScenarioVerdict::new("__suite_summary__")
        .with_threshold("scenarios_total", verdicts.len() as f64, verdicts.len() as f64)
        .with_threshold("scenarios_passed", verdicts.iter().filter(|v| v.passed).count() as f64, verdicts.len() as f64);
    write_verdict(&summary);

    assert!(all_passed, "One or more chaos scenarios failed");
}
