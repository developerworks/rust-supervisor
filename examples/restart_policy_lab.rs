//! restart policy(重启策略) lab(实验) example(示例).

use rust_supervisor::error::types::{TaskFailure, TaskFailureKind};
use rust_supervisor::policy::decision::{RestartDecision, RestartPolicy, SupervisionStrategy};
use std::time::Duration;

fn main() {
    let failure = TaskFailure::new(TaskFailureKind::Panic, "panic", "worker panicked");
    let policy = RestartPolicy::Transient;
    let strategy = SupervisionStrategy::OneForOne;
    let decision = RestartDecision::RestartAfter {
        delay: Duration::from_millis(100),
    };
    println!("failure={failure:#?}");
    println!("policy={policy:#?}");
    println!("strategy={strategy:#?}");
    println!("decision={decision:#?}");
}
