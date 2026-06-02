//! Maintainability integration tests.
//!
//! These tests ensure code, examples, and docs keep separate ownership areas.

use std::fs;
use std::path::Path;

/// Verifies that maintainability docs and scripts exist.
#[test]
fn maintainability_materials_exist() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));

    for file in [
        "scripts/check-maintainability.sh",
        "docs/zh/quality-gates.md",
        "docs/en/quality-gates.md",
        "docs/zh/parallel-governance.md",
        "docs/en/parallel-governance.md",
    ] {
        assert!(root.join(file).is_file(), "missing {file}");
    }
}

/// Verifies that the maintainability script documents the shutdown terminology.
#[test]
fn maintainability_script_checks_shutdown_terminology() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let script =
        fs::read_to_string(root.join("scripts/check-maintainability.sh")).expect("read script");

    assert!(script.contains("Shutdown Without Orphaned Tasks"));
}
