//! Coding standard integration tests.
//!
//! These tests enforce source documentation and test naming rules.

use std::fs;
use std::path::{Path, PathBuf};

/// Documentation files that must not use Chinese punctuation.
const DOCUMENTATION_FILES: &[&str] = &[
    "README.md",
    "README.zh.md",
    "manual/zh/index.md",
    "manual/zh/getting-started.md",
    "manual/zh/configuration.md",
    "manual/zh/supervisor-tree.md",
    "manual/zh/task-model.md",
    "manual/zh/policies.md",
    "manual/zh/runtime-control.md",
    "manual/zh/shutdown.md",
    "manual/zh/observability.md",
    "manual/zh/examples.md",
    "manual/zh/quality-gates.md",
    "manual/en/index.md",
    "manual/en/getting-started.md",
    "manual/en/configuration.md",
    "manual/en/supervisor-tree.md",
    "manual/en/task-model.md",
    "manual/en/policies.md",
    "manual/en/runtime-control.md",
    "manual/en/shutdown.md",
    "manual/en/observability.md",
    "manual/en/examples.md",
    "manual/en/quality-gates.md",
];

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

/// Verifies that public documentation avoids Chinese punctuation.
#[test]
fn coding_standard_is_enforced() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));

    for relative in DOCUMENTATION_FILES {
        let text = fs::read_to_string(root.join(relative)).expect("read documentation file");
        assert!(
            !contains_chinese_punctuation(&text),
            "Chinese punctuation is not allowed in {relative}"
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

/// Returns whether text contains disallowed Chinese punctuation.
fn contains_chinese_punctuation(text: &str) -> bool {
    text.chars()
        .any(|character| "，。；：！？、（）【】《》“”‘’".contains(character))
}
