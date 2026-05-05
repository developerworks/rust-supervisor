//! Demonstrates policy decisions across typed task exits and fuse tracking.

use rust_supervisor::policy::backoff::BackoffPolicy;
use rust_supervisor::policy::decision::{PolicyEngine, PolicyFailureKind, RestartPolicy, TaskExit};
use rust_supervisor::policy::meltdown::{MeltdownPolicy, MeltdownTracker};
use std::time::{Duration, Instant};

fn main() {
    let backoff = BackoffPolicy::new(
        Duration::from_millis(100),
        Duration::from_secs(5),
        10,
        Duration::from_secs(60),
    )
    .with_deterministic_jitter(42);
    let engine = PolicyEngine::new();

    for (policy, exit) in [
        (RestartPolicy::Permanent, TaskExit::Succeeded),
        (
            RestartPolicy::Transient,
            TaskExit::Failed {
                kind: PolicyFailureKind::ExternalDependency,
            },
        ),
        (
            RestartPolicy::Transient,
            TaskExit::Failed {
                kind: PolicyFailureKind::FatalBug,
            },
        ),
        (
            RestartPolicy::Temporary,
            TaskExit::Failed {
                kind: PolicyFailureKind::Panic,
            },
        ),
    ] {
        let decision = engine.decide(policy, exit, 3, &backoff);
        println!("policy={policy:?} exit={exit:?} decision={decision:?}");
    }

    let policy = MeltdownPolicy::new(
        2,
        Duration::from_secs(60),
        5,
        Duration::from_secs(60),
        Duration::from_secs(300),
    );
    let mut tracker = MeltdownTracker::new(policy);
    let now = Instant::now();

    for offset_ms in [0, 10, 20] {
        let outcome = tracker.record_child_restart(now + Duration::from_millis(offset_ms));
        println!(
            "restart_at_ms={offset_ms} child_failures={} outcome={outcome:?}",
            tracker.child_failure_count()
        );
    }
}
