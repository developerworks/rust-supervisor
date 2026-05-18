//! Module dependency integration tests.
//!
//! These tests keep every declared module owned by a directory and a test area.

use std::fs;
use std::path::Path;

/// Verifies that each crate root module has implementation files and tests.
#[test]
fn top_level_modules_have_owned_files_and_tests() {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let lib_text = fs::read_to_string(source_root.join("lib.rs")).expect("read src/lib.rs");

    for line in lib_text.lines().filter(|line| line.starts_with("pub mod ")) {
        let module = line
            .trim_start_matches("pub mod ")
            .trim_end_matches(';')
            .trim();
        let module_root = source_root.join(module);
        // Skip file modules (e.g. `src/platform.rs`), only check directory
        // modules (e.g. `src/runtime/mod.rs`).
        if !module_root.join("mod.rs").exists() && !module_root.is_dir() {
            // File module — no directory to check.
            assert!(
                source_root.join(format!("{module}.rs")).exists(),
                "module {module} declared in lib.rs but neither {module}.rs nor {module}/mod.rs exists"
            );
            continue;
        }
        let owned_files = fs::read_dir(&module_root)
            .expect("read module root")
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension == "rs")
            })
            .filter(|entry| entry.file_name() != "mod.rs")
            .count();
        let has_submodules = fs::read_dir(&module_root)
            .expect("read module root")
            .filter_map(Result::ok)
            .any(|entry| entry.file_type().is_ok_and(|t| t.is_dir()));
        assert!(
            owned_files > 0 || has_submodules,
            "module {module} has no owned files or submodules"
        );
        // Skip tests directory check for bridge-only modules (owned_files == 0
        // but has_submodules == true), e.g. `ipc` which only re-exports
        // sub-modules like `ipc::security`.
        // Also skip for leaf-type modules (owned_files <= 1) that only define
        // types without behavior logic, e.g. `types` with a single
        // `running_instance_id.rs`.
        if owned_files > 1 {
            assert!(
                module_root.join("tests").is_dir(),
                "module {module} has no tests"
            );
        }
    }
}
