//! four-stage shutdown(四阶段关闭) example(示例).

use rust_supervisor::config::loader::load_config_state;
use rust_supervisor::runtime::supervisor::Supervisor;
use rust_supervisor::shutdown::stage::ShutdownPhase;

#[tokio::main]
async fn main() -> Result<(), rust_supervisor::error::types::SupervisorError> {
    let state = load_config_state("examples/config/supervisor.yaml")?;
    let spec = state.to_supervisor_spec()?;
    let handle = Supervisor::start(spec).await?;
    for phase in [
        ShutdownPhase::RequestStop,
        ShutdownPhase::GracefulDrain,
        ShutdownPhase::AbortStragglers,
        ShutdownPhase::Reconcile,
    ] {
        println!("planned phase={phase:#?}");
    }
    handle
        .shutdown_tree("operator", "shutdown tree example")
        .await?;
    Ok(())
}
