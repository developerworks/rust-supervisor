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

/// Verifies that configuration schema support is represented in release files.
#[test]
fn release_files_include_config_schema_support() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cargo = fs::read_to_string(root.join("Cargo.toml")).expect("read Cargo.toml");
    let readme = fs::read_to_string(root.join("README.md")).expect("read README.md");

    assert!(cargo.contains("confique = "));
    assert!(cargo.contains("schemars = "));
    assert!(root.join("examples/config/supervisor.yaml").is_file());
    assert!(
        root.join("examples/config/supervisor.template.yaml")
            .is_file()
    );
    assert!(readme.contains("SupervisorConfig"));
    assert!(readme.contains("x-tree-split"));
}
