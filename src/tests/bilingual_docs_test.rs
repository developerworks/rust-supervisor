//! Bilingual documentation integration tests.
//!
//! These tests verify that Chinese and English documentation sets stay paired.

use std::path::Path;

/// Manual pages that must exist in both language directories.
const MANUAL_PAGES: &[&str] = &[
    "index.md",
    "language.md",
    "getting-started.md",
    "configuration.md",
    "supervisor-tree.md",
    "task-model.md",
    "policies.md",
    "runtime-control.md",
    "shutdown.md",
    "observability.md",
    "examples.md",
    "quality-gates.md",
];

/// Verifies that bilingual document pairs exist.
#[test]
fn bilingual_document_pairs_exist() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for relative in ["index.md", "quality-gates.md", "parallel-governance.md"] {
        assert!(root.join("docs/zh").join(relative).is_file());
        assert!(root.join("docs/en").join(relative).is_file());
    }

    for relative in MANUAL_PAGES {
        assert!(root.join("manual/zh").join(relative).is_file());
        assert!(root.join("manual/en").join(relative).is_file());
    }
}

/// Verifies that manual language directories stay structurally isomorphic.
#[test]
fn bilingual_documentation_is_isomorphic() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));

    for relative in MANUAL_PAGES {
        assert!(
            root.join("manual/zh").join(relative).is_file(),
            "missing Chinese manual page {relative}"
        );
        assert!(
            root.join("manual/en").join(relative).is_file(),
            "missing English manual page {relative}"
        );
    }
}
