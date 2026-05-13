use rust_supervisor::dashboard::state::{
    DashboardStateInput, build_dashboard_state, declared_state_from_spec,
};
use rust_supervisor::id::types::ChildId;
use rust_supervisor::journal::ring::EventJournal;
use rust_supervisor::spec::child::{ChildSpec, TaskKind};
use rust_supervisor::spec::supervisor::SupervisorSpec;
use rust_supervisor::task::factory::{TaskResult, service_fn};
use std::sync::Arc;

fn sample_spec() -> SupervisorSpec {
    let factory = service_fn(|_ctx| async { TaskResult::Succeeded });
    let child = ChildSpec::worker(
        ChildId::new("payment_loop"),
        "payment loop",
        TaskKind::AsyncWorker,
        Arc::new(factory),
    );
    SupervisorSpec::root(vec![child])
}

#[test]
fn dashboard_state_contains_topology_and_runtime_state() {
    let spec = sample_spec();
    let state = declared_state_from_spec(&spec);
    let journal = EventJournal::new(16);

    let state = build_dashboard_state(
        DashboardStateInput {
            target_id: "payments-worker-a".to_owned(),
            display_name: "payments worker a".to_owned(),
            authorization_scope: "payments:operate".to_owned(),
            state_generation: 1,
            recent_limit: 16,
        },
        &spec,
        &state,
        &journal,
    );

    assert_eq!(state.target.target_id, "payments-worker-a");
    assert_eq!(state.topology.nodes.len(), 2);
    assert_eq!(state.runtime_state.len(), 1);
    assert_eq!(state.state_generation, 1);
}
