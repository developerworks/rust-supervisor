//! Supervisor configuration integration tests.
//!
//! These tests verify that validated configuration can drive supervisor startup.

use rust_supervisor::config::loader::load_config_state;
use rust_supervisor::runtime::supervisor::Supervisor;
use std::path::Path;

/// Verifies that the example YAML configuration can produce a running handle.
#[tokio::test]
async fn yaml_config_derives_startable_supervisor_spec() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let state =
        load_config_state(root.join("examples/config/supervisor.yaml")).expect("load YAML config");
    let spec = state.to_supervisor_spec().expect("derive supervisor spec");
    let handle = Supervisor::start(spec).await.expect("start supervisor");

    let current = handle.current_state().await.expect("current state");
    assert!(matches!(
        current,
        rust_supervisor::control::command::CommandResult::CurrentState { .. }
    ));
}
