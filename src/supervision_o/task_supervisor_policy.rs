use std::time::Duration;

use crate::supervision_o::enums::TaskClass;

/// 任务重启策略。
#[derive(Debug, Clone, Copy)]
pub struct TaskSupervisorPolicy {
    pub(super) task_class: TaskClass,
    pub(super) max_restarts: u32,
    pub(super) restart_base_delay: Duration,
    pub(super) restart_max_delay: Duration,
}
