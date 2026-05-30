//! Loads a split supervisor configuration with body-only `groups.yaml` and
//! `children.yaml` transparent array sections.

use rust_supervisor::config::loader::load_config_from_yaml_file;

type ExampleResult = Result<(), rust_supervisor::error::types::SupervisorError>;

/// Loads the split example config and prints derived child and group counts.
fn main() -> ExampleResult {
    let state = load_config_from_yaml_file("examples/config/split/supervisor.yaml")?;

    println!("strategy: {:?}", state.supervisor.strategy);
    println!("groups: {}", state.groups.len());
    println!("children: {}", state.children.len());

    for group in &state.groups {
        println!("  group: {}", group.name);
    }

    for child in &state.children {
        println!("  child: {}", child.name);
    }

    Ok(())
}
