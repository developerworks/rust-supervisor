//! Bilingual documentation integration tests.
//!
//! These tests verify that Chinese and English documentation sets stay paired.

use std::path::Path;

/// Verifies that bilingual document pairs exist.
#[test]
fn bilingual_document_pairs_exist() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for relative in ["index.md", "quality-gates.md", "parallel-governance.md"] {
        assert!(root.join("docs/zh").join(relative).is_file());
        assert!(root.join("docs/en").join(relative).is_file());
    }

    assert!(root.join("manual/zh/index.md").is_file());
    assert!(root.join("manual/en/index.md").is_file());
}
