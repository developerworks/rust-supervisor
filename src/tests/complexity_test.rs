//! Complexity budget integration tests.
//!
//! These tests apply a simple source-level guard for oversized functions.

use std::fs;
use std::path::{Path, PathBuf};

/// Verifies that no function body grows beyond the documented maintainability budget.
#[test]
fn functions_stay_within_line_budget() {
    for path in rust_files(Path::new(env!("CARGO_MANIFEST_DIR")).join("src")) {
        let text = fs::read_to_string(&path).expect("read rust file");
        let mut current_function_lines = 0_usize;
        for line in text.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("fn ") || trimmed.starts_with("pub fn ") {
                current_function_lines = 1;
                continue;
            }
            if current_function_lines > 0 {
                current_function_lines += 1;
                assert!(
                    current_function_lines <= 80,
                    "function line budget exceeded in {:?}",
                    path
                );
                if trimmed == "}" {
                    current_function_lines = 0;
                }
            }
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
