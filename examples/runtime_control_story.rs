//! Demonstrates an operator control flow against a running supervisor.

use rust_supervisor::config::loader::load_config_state;
use rust_supervisor::control::command::CommandResult;
use rust_supervisor::id::types::{ChildId, SupervisorPath};
use rust_supervisor::runtime::supervisor::Supervisor;

#[tokio::main]
async fn main() -> Result<(), rust_supervisor::error::types::SupervisorError> {
    let state = load_config_state("examples/config/supervisor.yaml")?;
    let spec = state.to_supervisor_spec()?;
    let handle = Supervisor::start(spec).await?;
    let mut events = handle.subscribe_events();
    let child_id = ChildId::new("market_feed");

    let add = handle
        .add_child(
            SupervisorPath::root(),
            "id=market_feed kind=AsyncWorker readiness=Explicit",
            "operator",
            "attach market feed during incident rehearsal",
        )
        .await?;
    print_result("add_child", add);

    print_result(
        "pause_child",
        handle
            .pause_child(child_id.clone(), "operator", "stop automatic restart")
            .await?,
    );
    print_result(
        "resume_child",
        handle
            .resume_child(child_id.clone(), "operator", "resume lifecycle governance")
            .await?,
    );
    print_result(
        "quarantine_child",
        handle
            .quarantine_child(child_id, "operator", "manual investigation")
            .await?,
    );
    print_result("current_state", handle.current_state().await?);

    while let Ok(event) = events.try_recv() {
        println!("event={event}");
    }

    print_result(
        "shutdown_tree",
        handle
            .shutdown_tree("operator", "runtime control story complete")
            .await?,
    );

    Ok(())
}

fn print_result(label: &str, result: CommandResult) {
    println!("{label}={result:#?}");
}
