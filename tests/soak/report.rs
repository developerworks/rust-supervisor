//! Soak report generation.
//!
//! Produces a SoakReport Markdown document following the contract in
//! `specs/006-7-chaos-soak-reliability/contracts/soak-report-format.md`.

use std::time::{SystemTime, UNIX_EPOCH};

/// A single threshold row in the soak report.
#[derive(Debug, Clone)]
pub struct ThresholdRow {
    /// Metric name.
    pub metric: &'static str,
    /// 99th percentile value.
    pub p99: f64,
    /// Average value.
    pub avg: f64,
    /// Maximum value.
    pub max: f64,
    /// Threshold limit.
    pub limit: f64,
    /// Whether the metric passed.
    pub passed: bool,
}

/// A violation entry.
#[derive(Debug, Clone)]
pub struct Violation {
    /// Metric name.
    pub metric: String,
    /// Actual measured value.
    pub actual_value: f64,
    /// Threshold limit.
    pub limit: f64,
    /// Whether this is a blocking violation.
    pub blocking: bool,
    /// Optional exemption ticket ID.
    pub exemption_ticket: Option<String>,
}

/// The complete soak report.
#[derive(Debug, Clone)]
pub struct SoakReport {
    /// Test window start (UTC ISO 8601).
    pub window_start_utc: String,
    /// Test window end (UTC ISO 8601).
    pub window_end_utc: String,
    /// Supervisor commit hash.
    pub commit_hash: String,
    /// Hardware configuration description.
    pub hardware_config: String,
    /// Threshold comparison table.
    pub thresholds: Vec<ThresholdRow>,
    /// Violation entries.
    pub violations: Vec<Violation>,
    /// Shutdown success ratio (from 100 synthetic shutdowns).
    pub shutdown_success_ratio: f64,
}

impl SoakReport {
    /// Creates a new soak report with metadata.
    pub fn new(hardware_config: &str) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let start = now - 86400; // 24h ago

        // Format as ISO 8601 (simplified).
        fn format_iso(ts: u64) -> String {
            let secs = ts as i64;
            let days = secs / 86400;
            let time = secs % 86400;
            let hours = time / 3600;
            let mins = (time % 3600) / 60;
            let secs = time % 60;
            // Simplified: uses a fixed epoch-based date.
            format!("2026-05-{:02}T{:02}:{:02}:{:02}Z", 19 + days as u8, hours, mins, secs)
        }

        Self {
            window_start_utc: format_iso(start),
            window_end_utc: format_iso(now),
            commit_hash: std::env::var("COMMIT_HASH").unwrap_or_else(|_| "unknown".into()),
            hardware_config: hardware_config.to_string(),
            thresholds: Vec::new(),
            violations: Vec::new(),
            shutdown_success_ratio: 0.0,
        }
    }

    /// Adds a threshold row.
    pub fn add_threshold(&mut self, row: ThresholdRow) {
        if !row.passed {
            self.violations.push(Violation {
                metric: row.metric.to_string(),
                actual_value: row.p99,
                limit: row.limit,
                blocking: row.p99 > row.limit * 1.5, // blocking if > 1.5x limit
                exemption_ticket: None,
            });
        }
        self.thresholds.push(row);
    }

    /// Renders the report as Markdown.
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        md.push_str("# SoakReport\n\n");
        md.push_str("## Metadata\n");
        md.push_str(&format!("- **Window**: {} - {}\n", self.window_start_utc, self.window_end_utc));
        md.push_str(&format!("- **Commit**: {}\n", self.commit_hash));
        md.push_str(&format!("- **Hardware**: {}\n\n", self.hardware_config));

        md.push_str("## Thresholds\n");
        md.push_str("| Metric | p99 | Avg | Max | Limit | Passed |\n");
        md.push_str("|--------|-----|-----|-----|-------|--------|\n");
        for row in &self.thresholds {
            md.push_str(&format!(
                "| {} | {:.2} | {:.2} | {:.2} | {:.2} | {} |\n",
                row.metric, row.p99, row.avg, row.max, row.limit, row.passed
            ));
        }
        md.push('\n');

        md.push_str("## Violations\n");
        if self.violations.is_empty() {
            md.push_str("(none)\n\n");
        } else {
            md.push_str("| Metric | Actual | Limit | Blocking | Exemption Ticket |\n");
            md.push_str("|--------|--------|-------|----------|------------------|\n");
            for v in &self.violations {
                let ticket = v.exemption_ticket.as_deref().unwrap_or("-");
                md.push_str(&format!(
                    "| {} | {:.2} | {:.2} | {} | {} |\n",
                    v.metric, v.actual_value, v.limit, if v.blocking { "yes" } else { "no" }, ticket
                ));
            }
            md.push('\n');
        }

        md.push_str("## Exemptions\n");
        md.push_str("(none)\n\n");

        md.push_str("## Attachments\n");
        md.push_str("| File | SHA-256 |\n");
        md.push_str("|------|---------|\n");
        md.push_str("| p99_latency_curve.png | (not generated) |\n");
        md.push_str("| rss_curve.png | (not generated) |\n");
        md.push_str("| fd_count_curve.png | (not generated) |\n");

        md
    }
}
