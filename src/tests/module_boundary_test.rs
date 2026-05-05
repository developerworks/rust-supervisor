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
        let allowed = trimmed.is_empty() || trimmed.starts_with("//!") || is_pub_mod(trimmed);
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
            assert!(
                trimmed.is_empty() || is_pub_mod(trimmed),
                "unexpected module entry line in {:?}: {trimmed}",
                module_file
            );
        }
        assert!(!text.contains("pub use"));
    }
}

/// Reports whether a line is a simple public module declaration.
fn is_pub_mod(line: &str) -> bool {
    line.starts_with("pub mod ") && line.ends_with(';') && !line.contains('{')
}
