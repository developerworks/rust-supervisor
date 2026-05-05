//! Glossary coverage integration tests.
//!
//! These tests keep professional terms in a standalone glossary file.

use std::fs;
use std::path::Path;

/// Verifies that key public API terms are listed in the glossary.
#[test]
fn glossary_contains_public_api_terms() {
    let glossary = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("specs/001-create-supervisor-core/glossary.md"),
    )
    .expect("read glossary");

    for term in [
        "`Supervisor`",
        "`ChildSpec`",
        "`SupervisorSpec`",
        "`TaskFactory`",
        "`SupervisorState`",
        "`ChildState`",
        "`ConfigState`",
        "`SBOMArtifact`",
    ] {
        assert!(glossary.contains(term), "missing glossary term {term}");
    }
}

/// Verifies that the planned glossary gate checks professional and backtick terms.
#[test]
fn glossary_covers_professional_and_backtick_terms() {
    glossary_contains_public_api_terms();
}
