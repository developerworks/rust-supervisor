//! Demonstrates a multi-child supervisor tree declaration and traversal.

use rust_supervisor::id::types::ChildId;
use rust_supervisor::readiness::signal::ReadinessPolicy;
use rust_supervisor::spec::child::{ChildSpec, Criticality, TaskKind};
use rust_supervisor::spec::supervisor::{SupervisionStrategy, SupervisorSpec};
use rust_supervisor::task::factory::{TaskResult, service_fn};
use rust_supervisor::tree::builder::SupervisorTree;
use rust_supervisor::tree::order::{restart_scope, shutdown_order, startup_order};
use std::sync::Arc;

fn main() -> Result<(), rust_supervisor::error::types::SupervisorError> {
    let mut market_feed = worker("market_feed", "Market Feed");
    market_feed.tags = vec!["market".to_owned(), "network".to_owned()];
    market_feed.readiness_policy = ReadinessPolicy::Explicit;

    let mut risk_engine = worker("risk_engine", "Risk Engine");
    risk_engine.dependencies = vec![market_feed.id.clone()];
    risk_engine.tags = vec!["risk".to_owned()];

    let mut audit_sink = worker("audit_sink", "Audit Sink");
    audit_sink.criticality = Criticality::Optional;
    audit_sink.tags = vec!["audit".to_owned()];

    let mut spec = SupervisorSpec::root(vec![market_feed.clone(), risk_engine, audit_sink]);
    spec.strategy = SupervisionStrategy::RestForOne;
    spec.config_version = "examples-supervisor-tree-story".to_owned();

    let tree = SupervisorTree::build(&spec)?;
    println!("root_path={}", tree.root_path);
    println!("startup_order={:?}", child_names(startup_order(&tree)));
    println!("shutdown_order={:?}", child_names(shutdown_order(&tree)));
    println!(
        "restart_scope_after_market_feed={:?}",
        restart_scope(&tree, spec.strategy, &market_feed.id)
    );

    Ok(())
}

fn worker(id: &str, name: &str) -> ChildSpec {
    let task_name = name.to_owned();
    let factory = service_fn(move |ctx| {
        let task_name = task_name.clone();
        async move {
            ctx.heartbeat();
            ctx.mark_ready();
            println!("worker={task_name} path={}", ctx.path);
            TaskResult::Succeeded
        }
    });

    ChildSpec::worker(
        ChildId::new(id),
        name,
        TaskKind::AsyncWorker,
        Arc::new(factory),
    )
}

fn child_names(nodes: Vec<&rust_supervisor::tree::builder::SupervisorTreeNode>) -> Vec<String> {
    nodes
        .into_iter()
        .map(|node| node.child.name.clone())
        .collect()
}
