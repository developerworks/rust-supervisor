//! Example suite integration tests.
//!
//! These tests keep the learning examples present and wired to YAML config.

use std::fs;
use std::path::Path;

/// Verifies that the expected examples exist and reference the public API.
#[test]
fn example_suite_contains_learning_programs() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for example in [
        "supervisor_quickstart.rs",
        "config_tree_supervisor.rs",
        "restart_policy_lab.rs",
        "shutdown_tree.rs",
        "observability_probe.rs",
        "supervisor_tree_story.rs",
        "runtime_control_story.rs",
        "policy_failure_matrix.rs",
        "diagnostic_replay.rs",
        "work_role_demo.rs",
        "group_isolation_demo.rs",
        "generation_fencing_demo.rs",
        "backpressure_demo.rs",
        "health_readiness_demo.rs",
        "shutdown_pipeline_demo.rs",
    ] {
        let text = fs::read_to_string(root.join("examples").join(example)).expect("read example");
        assert!(text.contains("rust_supervisor::"));
    }

    assert!(root.join("examples/config/supervisor.yaml").is_file());
}

/// Verifies that the demo keeps example runtime ownership outside core.
#[test]
fn demo_example_owns_dashboard_runtime_outside_core() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let demo = root.join("examples/demo/main.rs");
    let text = fs::read_to_string(&demo).expect("read demo example");
    let runner = fs::read_to_string(root.join("examples/demo/runner.rs")).expect("read runner");
    let bootstrap =
        fs::read_to_string(root.join("examples/demo/bootstrap.rs")).expect("read bootstrap");

    assert!(runner.contains("load_config_from_yaml_file"));
    assert!(runner.contains("Supervisor::start_from_config_state"));
    assert!(runner.contains("state.ipc = None"));
    assert!(bootstrap.contains("start_demo_dashboard_runtime"));
    assert!(!text.contains("Supervisor::start_from_config_file"));
    assert!(!runner.contains("Supervisor::start_from_config_file"));
    assert!(!runner.contains("to_supervisor_spec"));
    assert!(!root.join("src/bin").exists());

    let readme = fs::read_to_string(root.join("README.md")).expect("read README");
    assert!(
        readme.contains("cargo run --example demo -- --config examples/config/supervisor.yaml")
    );
}

/// Verifies that the dashboard demo uses modules instead of one large entry file.
#[test]
fn demo_example_uses_modular_runtime_files() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let demo_root = root.join("examples/demo");
    let main = fs::read_to_string(demo_root.join("main.rs")).expect("read demo main");

    for module in [
        "args.rs",
        "bootstrap.rs",
        "output.rs",
        "runner.rs",
        "scenario.rs",
        "shutdown.rs",
    ] {
        assert!(demo_root.join(module).is_file(), "missing {module}");
    }

    assert!(main.contains("mod args;"));
    assert!(main.contains("mod bootstrap;"));
    assert!(main.contains("mod output;"));
    assert!(main.contains("mod runner;"));
    assert!(main.contains("mod scenario;"));
    assert!(main.contains("mod shutdown;"));
    assert!(main.contains("runner::run_demo"));
    assert!(!main.contains("fn parse_config_path"));
    assert!(!main.contains("current_state().await"));
    assert!(!main.contains("shutdown_tree("));
}

/// Verifies that the demo scenario covers UI-visible child states and commands.
#[test]
fn demo_example_scenario_covers_ui_surface() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let scenario =
        fs::read_to_string(root.join("examples/demo/scenario.rs")).expect("read scenario");

    for child in [
        "duplicate_guard",
        "retry_scheduler",
        "invoice_writer",
        "index_stream",
        "healthy_worker",
    ] {
        assert!(scenario.contains(child), "missing child {child}");
    }

    for state in ["failed", "restarting", "paused", "quarantined", "running"] {
        assert!(scenario.contains(state), "missing state {state}");
    }

    for command in [
        "RestartChild",
        "PauseChild",
        "ResumeChild",
        "QuarantineChild",
        "RemoveChild",
        "AddChild",
        "ShutdownTree",
    ] {
        assert!(scenario.contains(command), "missing command {command}");
    }
}
