use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::supervision_o::{
    enums::{TaskClass, TaskLifecycleEvent},
    runtime_handles::RuntimeHandles,
    runtime_trace_context::RuntimeTraceContext,
    supervised_task_handle::SupervisedTaskHandle,
    task_supervisor_policy::TaskSupervisorPolicy,
};

impl RuntimeHandles {
    /// 构造运行时句柄集合。
    ///
    /// 该构造函数供二进制启动层在完成任务装配后调用，
    /// 保证监督句柄矩阵与实际启动的后台任务一一对应。
    #[doc(hidden)]
    pub fn new(
        trace_context: RuntimeTraceContext,
        market_data_handle: SupervisedTaskHandle,
        lifecycle_events: mpsc::UnboundedReceiver<TaskLifecycleEvent>,
    ) -> Self {
        Self {
            trace_context,
            market_data_handle,
            lifecycle_events,
        }
    }

    /// 返回关键任务默认监督策略。
    #[doc(hidden)]
    pub fn critical_policy() -> TaskSupervisorPolicy {
        TaskSupervisorPolicy::for_critical()
    }

    /// 返回降级任务默认监督策略。
    #[doc(hidden)]
    pub fn degraded_policy() -> TaskSupervisorPolicy {
        TaskSupervisorPolicy::for_degraded()
    }

    /// 返回当前运行的启动追踪 ID。
    pub fn startup_id(&self) -> &str {
        &self.trace_context.startup_id
    }

    /// 返回当前监督会话 ID。
    pub fn supervisor_id(&self) -> &str {
        &self.trace_context.supervisor_id
    }

    /// 等待关键任务退出，返回 `{task_name}|{reason}|{restart_count}`。
    ///
    /// 处理规则如下：
    /// - 收到 `Restarted` 事件时，只记录日志并继续等待；
    /// - 收到关键任务 `Exited` 事件时，立即返回给启动层；
    /// - 收到非关键任务 `Exited` 事件时，记录告警但不结束等待；
    /// - 生命周期通道关闭时，返回固定占位字符串。
    pub async fn wait_for_critical_task_exit(&mut self) -> String {
        loop {
            match self.lifecycle_events.recv().await {
                Some(TaskLifecycleEvent::Restarted {
                    task_name,
                    task_class,
                    restart_count,
                    reason,
                }) => {
                    info!(
                        run_id = %self.trace_context.startup_id,
                        session_id = %self.trace_context.supervisor_id,
                        component = task_name,
                        task_class = task_class.label(),
                        restart_count,
                        reason = %reason,
                        "任务已按策略重启"
                    );
                }
                Some(TaskLifecycleEvent::Exited {
                    task_name,
                    task_class: TaskClass::Critical,
                    restart_count,
                    reason,
                }) => {
                    return format!("{task_name}|{reason}|{restart_count}");
                }
                Some(TaskLifecycleEvent::Exited {
                    task_name,
                    task_class,
                    restart_count,
                    reason,
                }) => {
                    warn!(
                        run_id = %self.trace_context.startup_id,
                        session_id = %self.trace_context.supervisor_id,
                        component = task_name,
                        task_class = task_class.label(),
                        restart_count,
                        reason = %reason,
                        "非关键任务退出"
                    );
                }
                None => {
                    return "supervision-channel-closed".to_string();
                }
            }
        }
    }
}
