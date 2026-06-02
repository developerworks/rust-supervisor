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
            "missing module documentation in {path:?}",
        );
    }
}

/// Verifies that example crate docs start with English module documentation.
#[test]
fn example_module_docs_start_with_english() {
    for path in rust_files(Path::new(env!("CARGO_MANIFEST_DIR")).join("examples")) {
        let lines = read_lines(&path);
        let first = lines.first().expect("example file must not be empty");
        assert!(
            first.trim_start().starts_with("//!"),
            "missing example module documentation in {path:?}",
        );
        assert!(
            has_ascii_alpha(first),
            "example module doc must include English in {:?}:1",
            path
        );
    }
}

/// Verifies that each Rust comment run includes English; Chinese is optional.
#[test]
fn rust_source_comments_require_english() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for path in rust_files(root.join("src"))
        .into_iter()
        .chain(rust_files(root.join("examples")))
    {
        let text = fs::read_to_string(&path).expect("read rust file");
        let lines: Vec<&str> = text.lines().collect();
        assert_comment_runs_have_english(&path, &lines);
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

/// Returns whether a line is a Rust comment (`//`, `///`, or `//!`).
fn is_rust_comment_line(line: &str) -> bool {
    line.trim_start().starts_with("//")
}

/// Returns whether a line contains at least one ASCII letter (English text).
fn has_ascii_alpha(line: &str) -> bool {
    line.chars().any(|character| character.is_ascii_alphabetic())
}

/// Requires every contiguous comment run to include English; Chinese lines are optional.
fn assert_comment_runs_have_english(path: &Path, lines: &[&str]) {
    let mut run_start: Option<usize> = None;
    for (index, line) in lines.iter().enumerate() {
        if is_rust_comment_line(line) {
            if run_start.is_none() {
                run_start = Some(index);
            }
        } else if let Some(start) = run_start {
            assert_comment_run_has_english(path, lines, start, index);
            run_start = None;
        }
    }
    if let Some(start) = run_start {
        assert_comment_run_has_english(path, lines, start, lines.len());
    }
}

/// Panics when a comment run has no line with ASCII letters.
fn assert_comment_run_has_english(path: &Path, lines: &[&str], start: usize, end: usize) {
    let has_english = lines[start..end].iter().any(|line| has_ascii_alpha(line));
    assert!(
        has_english,
        "comment run requires English (Chinese optional) in {:?}:{}",
        path,
        start.saturating_add(1)
    );
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
        // Skip blank lines, attribute starts (#[...]), and attribute
        // continuations (]), (])), etc. so that a doc comment placed
        // before a multi-line #[serde(...)] attribute is still found.
        if trimmed.is_empty()
            || trimmed.starts_with("#[")
            || trimmed.starts_with(']')
            || trimmed == ")"
            || trimmed == "),"
        {
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

/// Returns whether the current example code group has a leading comment.
fn has_previous_comment(lines: &[&str], index: usize) -> bool {
    let mut cursor = index;
    let mut blank_before_first_code = false;
    while cursor > 0 {
        cursor -= 1;
        let trimmed = lines[cursor].trim_start();
        if trimmed.is_empty() {
            blank_before_first_code = true;
            continue;
        }
        if trimmed.starts_with("//") {
            return true;
        }
        if is_code_line(trimmed) {
            if blank_before_first_code {
                return false;
            }
            while cursor > 0 {
                cursor -= 1;
                let previous = lines[cursor].trim_start();
                if previous.is_empty() {
                    while cursor > 0 {
                        cursor -= 1;
                        let leading = lines[cursor].trim_start();
                        if leading.is_empty() {
                            continue;
                        }
                        return leading.starts_with("//");
                    }
                    return false;
                }
                if previous.starts_with("//") {
                    return true;
                }
                if !is_code_line(previous) {
                    return false;
                }
            }
            return false;
        }
        return false;
    }
    false
}
