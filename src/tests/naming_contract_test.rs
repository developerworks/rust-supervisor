//! Naming contract integration tests.
//!
//! These tests keep source code on the agreed state and configuration names.

use std::fs;
use std::path::{Path, PathBuf};

/// Verifies that code does not introduce forbidden state suffixes.
#[test]
fn source_code_avoids_forbidden_snapshot_and_view_names() {
    let snapshot_suffix = ["Snap", "shot"].concat();
    let view_suffix = ["Vi", "ew"].concat();
    let snapshot_query = ["snap", "shot", "("].concat();
    let forbidden_state_module = ["state", "_", "view"].concat();

    for path in rust_files(Path::new(env!("CARGO_MANIFEST_DIR")).join("src")) {
        let text = fs::read_to_string(&path).expect("read rust file");
        assert!(
            !text.contains(&snapshot_suffix),
            "forbidden state suffix found in {:?}",
            path
        );
        assert!(
            !text.contains(&view_suffix),
            "forbidden view suffix found in {:?}",
            path
        );
        assert!(
            !text.contains(&snapshot_query),
            "forbidden state query found in {:?}",
            path
        );
        assert!(
            !text.contains(&forbidden_state_module),
            "forbidden state module name found in {:?}",
            path
        );
    }
}

/// Verifies that the approved state names exist in source code.
#[test]
fn source_code_uses_approved_state_names() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let combined = rust_files(root)
        .into_iter()
        .map(|path| fs::read_to_string(path).expect("read rust file"))
        .collect::<Vec<_>>()
        .join("\n");

    assert!(combined.contains("ConfigState"));
    assert!(combined.contains("SupervisorState"));
    assert!(combined.contains("ChildState"));
    assert!(combined.contains("current_state"));
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
