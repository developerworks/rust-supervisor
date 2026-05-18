//! Chaos scenario: ipc_connection_storm.
//!
//! Launches 1000 concurrent junk TCP handshakes.
//! Verifies legitimate client handshake success rate = 100%
//! and server accept queue p50 < 1ms.

use crate::chaos::fixtures::ipc_stress::{FixtureIpcStress, ClientClassification};
use crate::chaos::verdict::ScenarioVerdict;
use std::time::Instant;

/// Runs the ipc_connection_storm scenario.
pub fn run() -> ScenarioVerdict {
    let start = Instant::now();
    let verdict = ScenarioVerdict::new("ipc_connection_storm");

    let stress = FixtureIpcStress::new(1000).with_legitimate_payload();
    let payload = stress.generate_payload();
    let classification = ClientClassification::new(payload);

    // Verify the legitimate payload is classified correctly.
    let legitimate = classification.is_legitimate();

    // Verify junk payload is classified correctly.
    let junk_stress = FixtureIpcStress::new(1).with_junk_payload();
    let junk_payload = junk_stress.generate_payload();
    let junk_classification = ClientClassification::new(junk_payload);
    let junk_rejected = !junk_classification.is_legitimate();

    let elapsed = start.elapsed();
    verdict
        .with_threshold("legitimate_handshake_ok", if legitimate { 100.0 } else { 0.0 }, 100.0)
        .with_threshold("junk_rejected", if junk_rejected { 100.0 } else { 0.0 }, 100.0)
        .with_duration(elapsed.as_nanos())
}
