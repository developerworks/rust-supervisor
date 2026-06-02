//! Source layout integration tests.
//!
//! These tests enforce the top-level directory module layout used by the crate.

use std::path::Path;

/// Returns the project root path supplied by Cargo.
fn project_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// Returns the top-level source modules owned by the crate.
fn source_modules() -> Vec<&'static str> {
    vec![
        "child_runner",
        "config",
        "control",
        "error",
        "event",
        "health",
        "id",
        "journal",
        "observe",
        "policy",
        "readiness",
        "registry",
        "runtime",
        "shutdown",
        "spec",
        "state",
        "summary",
        "task",
        "test_support",
        "tree",
    ]
}

/// Verifies that every core module uses a top-level directory module.
#[test]
fn source_layout_uses_top_level_directory_modules() {
    let root = project_root();

    assert!(root.join("src/lib.rs").is_file());
    assert!(!root.join("src/supervision").exists());
    assert!(!root.join("src/supervision_o").exists());

    for module in source_modules() {
        assert!(
            root.join("src").join(module).is_dir(),
            "missing module {module}"
        );
        assert!(
            root.join("src").join(module).join("mod.rs").is_file(),
            "missing mod.rs for {module}"
        );
        assert!(
            root.join("src").join(module).join("tests").is_dir(),
            "missing tests directory for {module}"
        );
        assert!(
            !root.join("src").join(format!("{module}.rs")).exists(),
            "flat module file is forbidden for {module}"
        );
    }
}
