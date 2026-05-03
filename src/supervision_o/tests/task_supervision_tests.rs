//! `supervision` 模块单元测试。
//!
//! # 测试范围
//! - 覆盖运行追踪上下文 ID 生成规则；
//! - 覆盖监督策略的默认重启上限与任务分类；
//! - 覆盖可等待关停语义，防止录制裁剪与后台写入竞争。

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use tokio::sync::{Notify, mpsc};

use crate::supervision_o::{
    enums::TaskClass, runtime_handles::RuntimeHandles, runtime_trace_context::RuntimeTraceContext,
    task_supervision::spawn_supervised_task, task_supervisor_policy::TaskSupervisorPolicy,
};

use super::generate_runtime_id;

struct DropFlag(Arc<AtomicBool>);

impl Drop for DropFlag {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

#[test]
fn runtime_trace_context_uses_expected_prefixes() {
    let ctx = RuntimeTraceContext::new();
    assert!(ctx.startup_id.starts_with("run-"));
    assert!(ctx.supervisor_id.starts_with("session-"));
}

#[test]
fn runtime_trace_context_generates_distinct_ids() {
    let first = RuntimeTraceContext::new();
    let second = RuntimeTraceContext::new();

    assert_ne!(first.startup_id, second.startup_id);
    assert_ne!(first.supervisor_id, second.supervisor_id);
}

#[test]
fn generate_runtime_id_uses_expected_shape() {
    let first = generate_runtime_id("run");
    let second = generate_runtime_id("run");

    let first_parts = first.split('-').collect::<Vec<_>>();
    let second_parts = second.split('-').collect::<Vec<_>>();

    assert_eq!(first_parts.len(), 3);
    assert_eq!(second_parts.len(), 3);
    assert_eq!(first_parts[0], "run");
    assert_eq!(second_parts[0], "run");

    let first_ts = first_parts[1]
        .parse::<u128>()
        .expect("timestamp should be numeric");
    let second_ts = second_parts[1]
        .parse::<u128>()
        .expect("timestamp should be numeric");
    let first_seq = first_parts[2]
        .parse::<u64>()
        .expect("sequence should be numeric");
    let second_seq = second_parts[2]
        .parse::<u64>()
        .expect("sequence should be numeric");

    assert!(first_ts > 0);
    assert!(second_ts > 0);
    assert!(second_seq > first_seq);
}

#[test]
fn task_policies_have_expected_restart_limits() {
    let critical = RuntimeHandles::critical_policy();
    let degraded = RuntimeHandles::degraded_policy();

    assert_eq!(critical.task_class(), TaskClass::Critical);
    assert_eq!(critical.max_restarts(), 5);
    assert_eq!(degraded.task_class(), TaskClass::Degraded);
    assert_eq!(degraded.max_restarts(), 2);
}

#[tokio::test]
async fn supervised_task_shutdown_and_wait_waits_for_task_exit() {
    let (events_tx, _events_rx) = mpsc::unbounded_channel();
    let started = Arc::new(Notify::new());
    let dropped = Arc::new(AtomicBool::new(false));
    let task_started = started.clone();
    let task_dropped = dropped.clone();

    let mut handle = spawn_supervised_task(
        "shutdown_wait_test",
        TaskSupervisorPolicy::for_critical(),
        move || {
            let started = task_started.clone();
            let dropped = task_dropped.clone();
            tokio::spawn(async move {
                let _drop_flag = DropFlag(dropped);
                started.notify_one();
                std::future::pending::<()>().await;
            })
        },
        events_tx,
    );

    started.notified().await;
    handle.shutdown_and_wait().await;
    assert!(dropped.load(Ordering::SeqCst));

    handle.shutdown_and_wait().await;
}
