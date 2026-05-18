//! Naming contract integration tests.
//!
//! These tests keep source code on the agreed state and configuration names.

use std::fs;
use std::path::{Path, PathBuf};

/// Verifies that checked artifacts do not introduce forbidden state terms.
#[test]
fn checked_artifacts_avoid_forbidden_state_terms() {
    let state_copy_suffix = ["Snap", "shot"].concat();
    let visual_suffix = ["Vi", "ew"].concat();
    let state_copy_query = ["snap", "shot", "("].concat();
    let state_copy_literal = ["snap", "shot"].concat();
    let state_copy_generation = ["snap", "shot", "_generation"].concat();
    let forbidden_state_module = ["state", "_", "view"].concat();
    let repository_root = Path::new(env!("CARGO_MANIFEST_DIR"));

    for path in checked_files(repository_root) {
        let text = fs::read_to_string(&path)
            .expect("read rust file")
            .replace("scrollIntoView", "scroll_into_view_dom_api")
            .replace("fitTopologyView", "fit_topology_dom_api")
            .replace("scheduleFitTopologyView", "schedule_fit_topology_dom_api")
            .replace(
                "clearScheduledFitTopologyView",
                "clear_scheduled_fit_topology_dom_api",
            )
            .replace("fitView", "fit_canvas_dom_api")
            .replace("View diagnostics", "Open diagnostics");
        assert_forbidden_absent(&path, &text, &state_copy_suffix, "state suffix");
        assert_forbidden_absent(&path, &text, &visual_suffix, "visual suffix");
        assert_forbidden_absent(&path, &text, &state_copy_query, "state query");
        assert_forbidden_absent(&path, &text, &forbidden_state_module, "state module");
        assert_forbidden_absent(&path, &text, &state_copy_generation, "state generation");
        assert_forbidden_absent(&path, &text, &state_copy_literal, "state wire literal");
    }
}

/// Verifies that the approved state names exist in source code.
#[test]
fn source_code_uses_approved_state_names() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let combined = collect_files(root)
        .into_iter()
        .map(|path| fs::read_to_string(path).expect("read rust file"))
        .collect::<Vec<_>>()
        .join("\n");

    let approved_names = [
        "ConfigState",
        "SupervisorState",
        "ChildAttemptStatus",
        "ChildControlOperation",
        "ChildStopState",
        "ChildControlFailurePhase",
        "ChildControlFailure",
        "RestartLimitState",
        "ChildLivenessState",
        "ChildRuntimeRecord",
        "ChildControlResult",
        "GenerationFencePhase",
        "GenerationFenceDecision",
        "GenerationFenceOutcome",
        "GenerationFenceState",
        "PendingRestart",
        "StaleAttemptReport",
        "StaleReportHandling",
        "ChildRestartFenceEntered",
        "ChildRestartFenceAbortRequested",
        "ChildRestartFenceReleased",
        "ChildRestartConflict",
        "ChildAttemptStaleReport",
        "ChildRestartFencePendingDrained",
        "ReadinessState",
        "current_state",
    ];

    for approved_name in approved_names {
        assert!(
            combined.contains(approved_name),
            "approved state name `{approved_name}` was not found in source code"
        );
    }

    let old_child_state_result = ["CommandResult::", "Child", "State"].concat();
    assert!(
        !combined.contains(&old_child_state_result),
        "old child state command result variant must not be used"
    );
}

/// Collects files that are part of the cross-repository naming contract.
fn checked_files(repository_root: &Path) -> Vec<PathBuf> {
    let documents = [
        repository_root.join("src"),
        repository_root.join("tests"),
        repository_root.join("examples"),
        repository_root.join("manual"),
        repository_root.join("specs/003-supervisor-dashboard"),
    ];
    let sibling_root = repository_root
        .parent()
        .expect("repository should have a parent directory");
    let relay_root = sibling_root.join("rust-supervisor-relay");
    let ui_root = sibling_root.join("rust-supervisor-ui");
    let sibling_documents = [
        relay_root.join("src"),
        relay_root.join("tests"),
        relay_root.join("manual"),
        relay_root.join("README.md"),
        ui_root.join("src"),
        ui_root.join("tests"),
        ui_root.join("README.md"),
    ];
    documents
        .into_iter()
        .chain(sibling_documents)
        .filter(|path| path.exists())
        .flat_map(collect_files)
        .filter(|path| !is_generated_or_dependency_path(path))
        .collect()
}

/// Collects all readable source and documentation files under a path.
fn collect_files(root: PathBuf) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_text_files(&root, &mut files);
    files
}

/// Recursively appends source and documentation files to the output list.
fn collect_text_files(path: &Path, files: &mut Vec<PathBuf>) {
    if path.is_file() {
        if is_checked_file(path) {
            files.push(path.to_path_buf());
        }
        return;
    }

    for entry in fs::read_dir(path).expect("read directory") {
        let entry_path = entry.expect("read entry").path();
        if is_generated_or_dependency_path(&entry_path) {
            continue;
        }
        collect_text_files(&entry_path, files);
    }
}

/// Returns whether a file extension is part of the naming proof.
fn is_checked_file(path: &Path) -> bool {
    path.extension().is_some_and(|extension| {
        matches!(
            extension.to_string_lossy().as_ref(),
            "rs" | "ts" | "vue" | "md" | "json" | "yaml" | "yml"
        )
    })
}

/// Returns whether a path belongs to generated output or third-party dependencies.
fn is_generated_or_dependency_path(path: &Path) -> bool {
    if path
        .to_string_lossy()
        .contains("rust-supervisor-ui/src/components/ui")
    {
        return true;
    }
    path.components().any(|component| {
        let name = component.as_os_str().to_string_lossy();
        matches!(
            name.as_ref(),
            "target"
                | "node_modules"
                | "dist"
                | "build"
                | "coverage"
                | ".specify"
                | "Cargo.lock"
                | "package-lock.json"
                | "playwright-report"
                | "test-results"
        )
    })
}

/// Verifies that a forbidden term is absent from one checked file.
fn assert_forbidden_absent(path: &Path, text: &str, forbidden: &str, label: &str) {
    assert!(
        !text.contains(forbidden),
        "forbidden {label} `{forbidden}` found in {path:?}"
    );
}
