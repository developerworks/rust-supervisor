//! observability(可观测性) probe(探针) example(示例).

use rust_supervisor::config::loader::load_config_state;
use rust_supervisor::runtime::supervisor::Supervisor;

#[tokio::main]
async fn main() -> Result<(), rust_supervisor::error::types::SupervisorError> {
    let state = load_config_state("examples/config/supervisor.yaml")?;
    let spec = state.to_supervisor_spec()?;
    let handle = Supervisor::start(spec).await?;
    let mut events = handle.subscribe_events();
    let current = handle.current_state().await?;
    println!("current={current:#?}");
    if let Ok(event) = events.recv().await {
        println!("event={event:#?}");
    }
    handle
        .shutdown_tree("operator", "observability probe complete")
        .await?;
    Ok(())
}
