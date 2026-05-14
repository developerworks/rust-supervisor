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
    "manual/zh/language.md",
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
    "manual/en/language.md",
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

/// Verifies that example module comments remain English-only source docs.
#[test]
fn example_module_docs_use_english_only() {
    for path in rust_files(Path::new(env!("CARGO_MANIFEST_DIR")).join("examples")) {
        let lines = read_lines(&path);
        assert!(
            lines
                .first()
                .is_some_and(|line| line.trim_start().starts_with("//!")),
            "missing example module documentation in {:?}",
            path
        );
        for (index, line) in lines.iter().enumerate() {
            if line.trim_start().starts_with("//") {
                assert!(
                    !contains_han(line),
                    "non-English source comment in {:?}:{}",
                    path,
                    index.saturating_add(1)
                );
            }
        }
    }
}

/// Verifies that Rust comments and rustdoc do not contain Chinese text.
#[test]
fn rust_source_comments_use_english() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for path in rust_files(root.join("src"))
        .into_iter()
        .chain(rust_files(root.join("examples")))
    {
        let text = fs::read_to_string(&path).expect("read rust file");
        for (index, line) in text.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") && contains_han(trimmed) {
                panic!("Rust comment must use English in {:?}:{}", path, index + 1);
            }
        }
    }
}

/// Verifies that every Rust function has a documentation comment.
#[test]
fn rust_functions_have_documentation() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for path in rust_files(root.join("src"))
        .into_iter()
        .chain(rust_files(root.join("examples")))
    {
        let text = fs::read_to_string(&path).expect("read rust file");
        let lines = text.lines().collect::<Vec<_>>();
        for (index, line) in lines.iter().enumerate() {
            if is_function_line(line) && !has_previous_doc(&lines, index) {
                panic!("missing function documentation in {:?}:{}", path, index + 1);
            }
        }
    }
}

/// Verifies that every named struct field has a documentation comment.
#[test]
fn rust_struct_fields_have_documentation() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for path in rust_files(root.join("src"))
        .into_iter()
        .chain(rust_files(root.join("examples")))
    {
        let text = fs::read_to_string(&path).expect("read rust file");
        let lines = text.lines().collect::<Vec<_>>();
        check_struct_field_docs(&path, &lines);
    }
}

/// Verifies that every example code line has a comment immediately above it.
#[test]
fn example_code_lines_have_leading_comments() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for path in rust_files(root.join("examples")) {
        let text = fs::read_to_string(&path).expect("read example file");
        let lines = text.lines().collect::<Vec<_>>();
        for (index, line) in lines.iter().enumerate() {
            if is_code_line(line) && !has_previous_comment(&lines, index) {
                panic!(
                    "example code needs leading comment in {:?}:{}",
                    path,
                    index + 1
                );
            }
        }
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

/// Reads a text file into owned lines.
fn read_lines(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .expect("read rust file")
        .lines()
        .map(str::to_owned)
        .collect()
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

/// Returns whether text contains a Han character.
fn contains_han(text: &str) -> bool {
    text.chars()
        .any(|character| ('\u{4e00}'..='\u{9fff}').contains(&character))
}

/// Returns whether a line starts a Rust function item.
fn is_function_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    [
        "fn ",
        "async fn ",
        "pub fn ",
        "pub async fn ",
        "pub(crate) fn ",
        "pub(crate) async fn ",
    ]
    .iter()
    .any(|prefix| trimmed.starts_with(prefix))
}

/// Returns whether an item has rustdoc before its attributes.
fn has_previous_doc(lines: &[&str], index: usize) -> bool {
    let mut cursor = index;
    while cursor > 0 {
        cursor -= 1;
        let trimmed = lines[cursor].trim_start();
        if trimmed.is_empty() || trimmed.starts_with("#[") {
            continue;
        }
        return trimmed.starts_with("///") || trimmed.starts_with("//!");
    }
    false
}

/// Checks field documentation inside named struct bodies.
fn check_struct_field_docs(path: &Path, lines: &[&str]) {
    let mut in_struct = false;
    let mut depth = 0_i32;
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if !in_struct && starts_struct_body(trimmed) {
            in_struct = true;
            depth = brace_delta(trimmed);
            continue;
        }
        if !in_struct {
            continue;
        }
        if depth == 1 && is_struct_field_line(trimmed) && !has_previous_doc(lines, index) {
            panic!(
                "missing struct field documentation in {:?}:{}",
                path,
                index + 1
            );
        }
        depth += brace_delta(trimmed);
        if depth <= 0 {
            in_struct = false;
        }
    }
}

/// Returns whether a line starts a named struct body.
fn starts_struct_body(trimmed: &str) -> bool {
    (trimmed.starts_with("struct ") || trimmed.starts_with("pub struct ")) && trimmed.contains('{')
}

/// Returns whether a line looks like a named struct field.
fn is_struct_field_line(trimmed: &str) -> bool {
    let Some((name, _rest)) = trimmed.split_once(':') else {
        return false;
    };
    let name = name.trim_start_matches("pub ").trim();
    !name.is_empty()
        && name
            .chars()
            .all(|character| character == '_' || character.is_ascii_alphanumeric())
}

/// Returns the net brace count for a line.
fn brace_delta(line: &str) -> i32 {
    line.chars().filter(|character| *character == '{').count() as i32
        - line.chars().filter(|character| *character == '}').count() as i32
}

/// Returns whether a line is example code rather than comment or whitespace.
fn is_code_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    !trimmed.is_empty() && !trimmed.starts_with("//")
}

/// Returns whether the previous non-empty example line is a comment.
fn has_previous_comment(lines: &[&str], index: usize) -> bool {
    let mut cursor = index;
    while cursor > 0 {
        cursor -= 1;
        let trimmed = lines[cursor].trim_start();
        if trimmed.is_empty() {
            continue;
        }
        return trimmed.starts_with("//");
    }
    false
}
