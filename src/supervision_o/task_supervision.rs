//! 任务监督核心。
//!
//! 为后台任务提供统一的重启策略、生命周期事件与关闭句柄，
//! 供录制运行时和主运行时共用，避免重复实现导致语义漂移。

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;
use tracing::{error, warn};

use crate::supervision_o::backoff::ReconnectBackoff;
use crate::supervision_o::enums::{TaskClass, TaskLifecycleEvent};
use crate::supervision_o::supervised_task_handle::SupervisedTaskHandle;
use crate::supervision_o::task_supervisor_policy::TaskSupervisorPolicy;

/// 生成运行时 ID。
///
/// 该函数只在 `supervision` 模块内部使用，保证 `startup_id` / `supervisor_id`
/// 采用一致的编码格式，便于日志检索与测试断言。
pub(super) fn generate_runtime_id(prefix: &str) -> String {
    static NEXT_ID: AtomicU64 = AtomicU64::new(1);
    let ts_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let seq = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}-{ts_ms}-{seq}")
}

/// 启动一个受监督子任务。
pub fn spawn_supervised_task<F>(
    task_name: &'static str,
    policy: TaskSupervisorPolicy,
    mut spawn_task: F,
    lifecycle_events: mpsc::UnboundedSender<TaskLifecycleEvent>,
) -> SupervisedTaskHandle
where
    F: FnMut() -> JoinHandle<()> + Send + 'static,
{
    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let merged_task_name = task_name;
    let merged_task_class = policy.task_class;
    let mut restart_backoff = ReconnectBackoff::new_without_immediate_retry(
        policy.restart_base_delay,
        policy.restart_max_delay,
    );

    let handle = tokio::spawn(async move {
        let mut restart_count = 0u32;
        let mut child_handle = spawn_task();

        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        child_handle.abort();
                        let _ = child_handle.await;
                        return;
                    }
                }
                join_result = &mut child_handle => {
                    if *shutdown_rx.borrow() {
                        return;
                    }

                    let reason = match join_result {
                        Ok(()) => "task completed".to_string(),
                        Err(error) => join_failure_reason(&error, merged_task_name),
                    };

                    if restart_count < policy.max_restarts {
                        restart_count = restart_count.saturating_add(1);
                        let _ = lifecycle_events.send(TaskLifecycleEvent::Restarted {
                            task_name: merged_task_name,
                            task_class: merged_task_class,
                            restart_count,
                            reason: reason.clone(),
                        });
                        warn!(
                            component = merged_task_name,
                            task_class = merged_task_class.label(),
                            attempt = restart_count,
                            reason = %reason,
                            "任务退出，按重试策略重建"
                        );

                        let delay = restart_backoff.next_delay();
                        tokio::select! {
                            _ = shutdown_rx.changed() => {
                                if *shutdown_rx.borrow() {
                                    return;
                                }
                            }
                            _ = tokio::time::sleep(delay) => {}
                        }
                        child_handle = spawn_task();
                        continue;
                    }

                    let _ = lifecycle_events.send(TaskLifecycleEvent::Exited {
                        task_name: merged_task_name,
                        task_class: merged_task_class,
                        restart_count,
                        reason: reason.clone(),
                    });

                    if merged_task_class == TaskClass::Critical {
                        error!(
                            component = merged_task_name,
                            task_class = merged_task_class.label(),
                            restart_count,
                            reason = %reason,
                            "关键任务已到达重启上限，终止"
                        );
                    } else {
                        warn!(
                            component = merged_task_name,
                            task_class = merged_task_class.label(),
                            restart_count,
                            reason = %reason,
                            "降级任务已到达重试上限，终止"
                        );
                    }
                    return;
                }
            }
        }
    });

    SupervisedTaskHandle {
        handle: Some(handle),
        shutdown_tx,
    }
}

fn join_failure_reason(error: &tokio::task::JoinError, task_name: &'static str) -> String {
    if error.is_cancelled() {
        format!("{task_name} task aborted")
    } else if error.is_panic() {
        "task panic".to_string()
    } else {
        error.to_string()
    }
}

#[cfg(test)]
#[path = "tests/task_supervision_tests.rs"]
mod tests;
