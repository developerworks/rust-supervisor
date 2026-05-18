//! Module boundary integration tests.
//!
//! These tests keep crate and module entries limited to module declarations.

use std::fs;
use std::path::{Path, PathBuf};

/// Returns the source root path.
fn source_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Verifies that the crate root contains only crate docs and module entries.
#[test]
fn lib_rs_contains_only_crate_docs_and_public_modules() {
    let text = fs::read_to_string(source_root().join("lib.rs")).expect("read src/lib.rs");
    for line in text.lines() {
        let trimmed = line.trim();
        let allowed = trimmed.is_empty()
            || trimmed.starts_with("//!")
            || is_pub_mod(trimmed)
            || trimmed.starts_with("#[");
        assert!(allowed, "unexpected src/lib.rs line: {trimmed}");
    }
    assert!(!text.contains("pub use"));
}

/// Verifies that every top-level module entry contains only child module entries.
#[test]
fn module_mod_rs_contains_only_public_modules() {
    for entry in fs::read_dir(source_root()).expect("read src") {
        let path = entry.expect("read entry").path();
        if !path.is_dir() {
            continue;
        }
        let module_file = path.join("mod.rs");
        if !module_file.is_file() {
            continue;
        }
        let text = fs::read_to_string(&module_file).expect("read module file");
        for line in text.lines() {
            let trimmed = line.trim();
            let allowed = trimmed.is_empty()
                || is_pub_mod(trimmed)
                || trimmed.starts_with("#[")
                || trimmed.starts_with("//!");
            assert!(
                allowed,
                "unexpected module entry line in {:?}: {trimmed}",
                module_file
            );
        }
        assert!(!text.contains("pub use"));
    }
}

/// Verifies that `SupervisionStrategy` has one source owner.
#[test]
fn supervision_strategy_has_single_source_definition() {
    let mut definitions = Vec::new();
    collect_supervision_strategy_definitions(&source_root(), &mut definitions);

    assert_eq!(
        definitions,
        vec![source_root().join("spec").join("supervisor.rs")]
    );
}

/// Reports whether a line is a simple module declaration (public or private).
fn is_pub_mod(line: &str) -> bool {
    (line.starts_with("pub mod ") || line.starts_with("mod "))
        && line.ends_with(';')
        && !line.contains('{')
}

/// Collects files that define the `SupervisionStrategy` enum.
fn collect_supervision_strategy_definitions(path: &Path, definitions: &mut Vec<PathBuf>) {
    let pattern = ["pub enum", "SupervisionStrategy"].join(" ");
    for entry in fs::read_dir(path).expect("read source directory") {
        let path = entry.expect("read source entry").path();
        if path.is_dir() {
            collect_supervision_strategy_definitions(&path, definitions);
            continue;
        }
        if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
            continue;
        }
        let text = fs::read_to_string(&path).expect("read source file");
        if text.contains(&pattern) {
            definitions.push(path);
        }
    }
    definitions.sort();
}
