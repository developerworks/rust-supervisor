//! Release readiness integration tests.
//!
//! These tests verify crates.io-oriented package metadata and files.

use std::fs;
use std::path::Path;

/// Verifies that required release files and metadata are present.
#[test]
fn release_metadata_and_files_are_present() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cargo = fs::read_to_string(root.join("Cargo.toml")).expect("read Cargo.toml");

    for key in [
        "description = ",
        "license = ",
        "repository = ",
        "documentation = ",
        "readme = ",
        "keywords = ",
        "categories = ",
    ] {
        assert!(cargo.contains(key), "missing Cargo metadata {key}");
    }

    for file in ["README.md", "README.zh.md", "LICENSE", "CHANGELOG.md"] {
        assert!(root.join(file).is_file(), "missing release file {file}");
    }
}
