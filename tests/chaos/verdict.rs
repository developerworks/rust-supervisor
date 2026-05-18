//! Scenario verdict types and JSON output.
//!
//! Defines `ScenarioVerdict`, `ThresholdResult`, and `VerdictWriter`
//! for producing JSON judgement documents per chaos scenario run.
//! The JSON schema is formally defined in
//! `specs/006-7-chaos-soak-reliability/contracts/chaos-scenario-verdict.md`.

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

/// Result for a single threshold metric.
#[derive(Debug, Clone, Serialize)]
pub struct ThresholdResult {
    /// Actual measured value.
    pub value: f64,
    /// Threshold limit.
    pub limit: f64,
    /// Whether value meets the pass criterion.
    pub passed: bool,
}

/// Verdict for one chaos scenario run.
///
/// Serialized as JSON and written to stdout so CI can parse it with `jq`.
#[derive(Debug, Clone, Serialize)]
pub struct ScenarioVerdict {
    /// Scenario identifier in snake_case, e.g. `child_panic_storm`.
    pub scenario_id: &'static str,
    /// Semantic version from `CARGO_PKG_VERSION`.
    pub semver: &'static str,
    /// Overall pass/fail for this scenario.
    pub passed: bool,
    /// Per-threshold measurement results.
    pub thresholds: BTreeMap<&'static str, ThresholdResult>,
    /// Unix timestamp in nanoseconds when the scenario started.
    pub started_at_unix_nanos: u128,
    /// Duration of the scenario in nanoseconds.
    pub duration_ns: u128,
    /// Error message if the scenario failed unexpectedly; null on success.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ScenarioVerdict {
    /// Creates a new verdict builder with the current timestamp.
    pub fn new(scenario_id: &'static str) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        Self {
            scenario_id,
            semver: env!("CARGO_PKG_VERSION"),
            passed: true,
            thresholds: BTreeMap::new(),
            started_at_unix_nanos: now,
            duration_ns: 0,
            error: None,
        }
    }

    /// Inserts a threshold measurement.
    pub fn with_threshold(mut self, name: &'static str, value: f64, limit: f64) -> Self {
        let passed = value <= limit;
        if !passed {
            self.passed = false;
        }
        self.thresholds.insert(
            name,
            ThresholdResult {
                value,
                limit,
                passed,
            },
        );
        self
    }

    /// Sets the duration after the scenario completes.
    pub fn with_duration(mut self, duration_ns: u128) -> Self {
        self.duration_ns = duration_ns;
        self
    }

    /// Marks the verdict as failed with an error message.
    pub fn with_error(mut self, error: String) -> Self {
        self.passed = false;
        self.error = Some(error);
        self
    }
}

/// Writes a verdict as a JSON line to stdout.
pub fn write_verdict(verdict: &ScenarioVerdict) {
    let json = serde_json::to_string(verdict).unwrap_or_else(|e| {
        format!(
            r#"{{"scenario_id":"{}","semver":"{}","passed":false,"thresholds":{{}},"started_at_unix_nanos":0,"duration_ns":0,"error":"serialization failed: {}"}}"#,
            verdict.scenario_id,
            verdict.semver,
            e
        )
    });
    println!("{json}");
}

#[cfg(test)]
/// Validates that a JSON verdict string conforms to the schema defined in
/// `contracts/chaos-scenario-verdict.md`.
///
/// Checks top-level required fields and threshold structure.
fn validate_verdict_json(json_str: &str) -> Result<(), String> {
    let parsed: serde_json::Value =
        serde_json::from_str(json_str).map_err(|e| format!("invalid JSON: {e}"))?;

    let obj = parsed.as_object().ok_or("root must be a JSON object")?;

    // Required fields from the schema.
    for field in &[
        "scenario_id",
        "semver",
        "passed",
        "thresholds",
        "started_at_unix_nanos",
        "duration_ns",
    ] {
        if !obj.contains_key(*field) {
            return Err(format!("missing required field: {field}"));
        }
    }

    // scenario_id must be snake_case.
    let sid = obj["scenario_id"]
        .as_str()
        .ok_or("scenario_id must be a string")?;
    if !sid
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return Err(format!("scenario_id must be snake_case, got: {sid}"));
    }

    // semver must be "X.Y.Z".
    let semver = obj["semver"].as_str().ok_or("semver must be a string")?;
    let parts: Vec<&str> = semver.split('.').collect();
    if parts.len() != 3 {
        return Err(format!("semver must be X.Y.Z, got: {semver}"));
    }

    // passed must be boolean.
    obj["passed"].as_bool().ok_or("passed must be a boolean")?;

    // thresholds must be an object with ThresholdResult values.
    let thresholds = obj["thresholds"]
        .as_object()
        .ok_or("thresholds must be an object")?;
    for (name, tr) in thresholds {
        let tr_obj = tr
            .as_object()
            .ok_or_else(|| format!("threshold {name} must be an object"))?;
        if !tr_obj.contains_key("value")
            || !tr_obj.contains_key("limit")
            || !tr_obj.contains_key("passed")
        {
            return Err(format!("threshold {name} missing value/limit/passed"));
        }
    }

    // error must be string or null if present.
    if let Some(err) = obj.get("error") {
        if !err.is_string() && !err.is_null() {
            return Err("error must be a string or null".into());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verdict_serialization() {
        let v = ScenarioVerdict::new("test_scenario")
            .with_threshold("cpu_p99", 42.0, 100.0)
            .with_threshold("mem_mb", 128.0, 256.0)
            .with_duration(1_000_000_000);
        let json = serde_json::to_string(&v).unwrap();
        assert!(json.contains(r#""scenario_id":"test_scenario""#));
        assert!(json.contains(r#""passed":true"#));
        assert!(json.contains(r#""cpu_p99""#));
    }

    #[test]
    fn test_verdict_failure() {
        let v = ScenarioVerdict::new("fail_scenario").with_threshold("latency", 200.0, 100.0);
        assert!(!v.passed);
    }

    #[test]
    fn test_verdict_error() {
        let v = ScenarioVerdict::new("err_scenario").with_error("something went wrong".into());
        assert!(!v.passed);
        assert_eq!(v.error.unwrap(), "something went wrong");
    }

    #[test]
    fn test_semver_from_cargo() {
        let v = ScenarioVerdict::new("ver_test");
        // The semver must be a valid semver string like "0.1.2".
        let parts: Vec<&str> = v.semver.split('.').collect();
        assert_eq!(parts.len(), 3, "semver must have 3 dot-separated parts");
    }

    #[test]
    fn test_verdict_schema_validation_static() {
        // Statically construct a representative verdict for each scenario
        // and verify it passes schema validation. This avoids running the
        // actual scenarios (which require Tokio runtime and may take 60s+).
        let test_cases = [
            ("child_panic_storm", "self_panic_count", 0.0, 0.0),
            ("child_block_forever", "shutdown_duration_ms", 800.0, 1000.0),
            (
                "child_ignore_cancel",
                "slot_deactivated_ms",
                100.0,
                10_000.0,
            ),
            ("rapid_failure_10k", "restart_recovery_rate", 1.0, 0.0),
            ("slow_event_subscriber", "event_gap_total", 0.0, 0.0),
            ("command_channel_full", "send_closed", 1.0, 1.0),
            (
                "ipc_connection_storm",
                "legitimate_handshake_ok",
                100.0,
                100.0,
            ),
            ("socket_path_contention", "structured_error", 1.0, 1.0),
            ("relay_crash_loop", "restarts_completed", 5.0, 5.0),
            ("clock_step_backward", "monotonic_clock_ok", 1.0, 1.0),
            (
                "runtime_starvation_probe",
                "control_loop_iter_per_sec",
                1.0,
                0.0,
            ),
        ];
        for (scenario_id, threshold_name, value, limit) in &test_cases {
            let verdict = ScenarioVerdict::new(scenario_id)
                .with_threshold(threshold_name, *value, *limit)
                .with_duration(1_000_000_000);
            let json = serde_json::to_string(&verdict).unwrap();
            if let Err(e) = validate_verdict_json(&json) {
                panic!("scenario {scenario_id} schema validation failed: {e}");
            }
        }
    }

    #[test]
    fn test_verdict_schema_passed_scenario() {
        let v = ScenarioVerdict::new("schema_test_pass")
            .with_threshold("cpu", 42.0, 100.0)
            .with_duration(500_000_000);
        let json = serde_json::to_string(&v).unwrap();
        assert!(validate_verdict_json(&json).is_ok());
    }

    #[test]
    fn test_verdict_schema_failed_scenario() {
        let v = ScenarioVerdict::new("schema_test_fail")
            .with_threshold("cpu", 200.0, 100.0)
            .with_duration(500_000_000);
        let json = serde_json::to_string(&v).unwrap();
        // Should still pass schema validation even though the test failed.
        assert!(validate_verdict_json(&json).is_ok());
    }

    #[test]
    fn test_validate_verdict_rejects_bad_semver() {
        let json = r#"{"scenario_id":"test","semver":"bad","passed":true,"thresholds":{},"started_at_unix_nanos":0,"duration_ns":0}"#;
        assert!(validate_verdict_json(json).is_err());
    }

    #[test]
    fn test_validate_verdict_rejects_missing_field() {
        let json = r#"{"scenario_id":"test"}"#;
        assert!(validate_verdict_json(json).is_err());
    }
}
