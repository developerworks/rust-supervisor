use rust_supervisor::dashboard::snapshot::{
    DashboardSnapshotInput, build_dashboard_snapshot, declared_state_from_spec,
};
use rust_supervisor::id::types::ChildId;
use rust_supervisor::journal::ring::EventJournal;
use rust_supervisor::spec::child::{ChildSpec, TaskKind};
use rust_supervisor::spec::supervisor::SupervisorSpec;
use rust_supervisor::task::factory::{TaskResult, service_fn};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn large_spec(count: usize) -> SupervisorSpec {
    let mut children = Vec::new();
    for index in 0..count {
        let factory = service_fn(|_ctx| async { TaskResult::Succeeded });
        children.push(ChildSpec::worker(
            ChildId::new(format!("worker_{index}")),
            format!("worker {index}"),
            TaskKind::AsyncWorker,
            Arc::new(factory),
        ));
    }
    SupervisorSpec::root(children)
}

#[test]
fn dashboard_snapshot_builds_two_hundred_children_quickly() {
    let spec = large_spec(200);
    let state = declared_state_from_spec(&spec);
    let journal = EventJournal::new(256);
    let started = Instant::now();

    let snapshot = build_dashboard_snapshot(
        DashboardSnapshotInput {
            target_id: "payments".to_owned(),
            display_name: "payments".to_owned(),
            authorization_scope: "payments:operate".to_owned(),
            snapshot_generation: 1,
            recent_limit: 128,
        },
        &spec,
        &state,
        &journal,
    );

    assert_eq!(snapshot.runtime_state.len(), 200);
    assert!(started.elapsed() < Duration::from_secs(5));
}
