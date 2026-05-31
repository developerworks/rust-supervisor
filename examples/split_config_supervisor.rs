//! Loads a split supervisor configuration with body-only `groups.yaml` and
//! `children.yaml` transparent array sections.

use rust_supervisor::config::loader::load_config_from_yaml_file;

// Define the example result type used by the split configuration demo.
type ExampleResult = Result<(), rust_supervisor::error::types::SupervisorError>;

/// Loads the split example config and prints derived child and group counts.
fn main() -> ExampleResult {
    // Load the split configuration through the public YAML loader.
    let state = load_config_from_yaml_file("examples/config/split/supervisor.yaml")?;

    // Print the selected root supervision strategy.
    println!("strategy: {:?}", state.supervisor.strategy);
    // Print the number of configured groups.
    println!("groups: {}", state.groups.len());
    // Print the number of configured children.
    println!("children: {}", state.children.len());

    // Print every configured group name.
    for group in &state.groups {
        // Print the current group name.
        println!("  group: {}", group.name);
    }

    // Print every configured child name.
    for child in &state.children {
        // Print the current child name.
        println!("  child: {}", child.name);
    }

    // Return success after the split configuration summary is printed.
    Ok(())
}
