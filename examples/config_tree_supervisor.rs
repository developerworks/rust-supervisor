//! rust-config-tree(集中配置树) configuration(配置) example(示例).

use rust_supervisor::config::loader::load_config_state;

fn main() -> Result<(), rust_supervisor::error::types::SupervisorError> {
    let state = load_config_state("examples/config/supervisor.yaml")?;
    let spec = state.to_supervisor_spec()?;
    println!("{spec:#?}");
    Ok(())
}
