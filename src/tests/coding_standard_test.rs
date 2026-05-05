//! Coding standard integration tests.
//!
//! These tests enforce source documentation and test naming rules.

use std::fs;
use std::path::{Path, PathBuf};

/// Verifies that Rust source files start with module documentation.
#[test]
fn rust_source_files_have_module_documentation() {
    for path in rust_files(Path::new(env!("CARGO_MANIFEST_DIR")).join("src")) {
        if path.file_name().is_some_and(|name| name == "mod.rs") {
            continue;
        }
        let text = fs::read_to_string(&path).expect("read rust file");
        assert!(
            text.trim_start().starts_with("//!"),
            "missing module documentation in {:?}",
            path
        );
    }
}

/// Verifies that every test file uses the required suffix.
#[test]
fn rust_test_files_use_test_suffix() {
    for path in rust_files(Path::new(env!("CARGO_MANIFEST_DIR")).join("src")) {
        if path
            .components()
            .any(|component| component.as_os_str() == "tests")
        {
            let name = path.file_name().expect("file name").to_string_lossy();
            assert!(name.ends_with("_test.rs"), "invalid test file name {name}");
        }
    }
}

/// Collects Rust files under a directory.
fn rust_files(root: PathBuf) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_rust_files(&root, &mut files);
    files
}

/// Recursively appends Rust files to the output list.
fn collect_rust_files(path: &Path, files: &mut Vec<PathBuf>) {
    if path.is_file() {
        if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path.to_path_buf());
        }
        return;
    }

    for entry in fs::read_dir(path).expect("read directory") {
        collect_rust_files(&entry.expect("read entry").path(), files);
    }
}
