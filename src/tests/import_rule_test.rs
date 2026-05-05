//! Import rule integration tests.
//!
//! These tests reject relative project imports in source files.

use std::fs;
use std::path::{Path, PathBuf};

/// Verifies that project source code does not use relative parent imports.
#[test]
fn source_imports_use_absolute_crate_paths() {
    let forbidden = format!("{}{}", "super", "::");
    for path in rust_files(Path::new(env!("CARGO_MANIFEST_DIR")).join("src")) {
        let text = fs::read_to_string(&path).expect("read rust file");
        assert!(
            !text.contains(&forbidden),
            "relative super import is forbidden in {:?}",
            path
        );
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
