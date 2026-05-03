use crate::supervision_o::{
    constants::{
        TASK_CRITICAL_MAX_RESTARTS, TASK_DEGRADED_MAX_RESTARTS, TASK_RESTART_BASE_DELAY,
        TASK_RESTART_EXPONENT_CAP,
    },
    enums::TaskClass,
};

use super::task_supervisor_policy::TaskSupervisorPolicy;

impl TaskSupervisorPolicy {
    /// 关键任务策略。
    pub const fn for_critical() -> Self {
        Self {
            task_class: TaskClass::Critical,
            max_restarts: TASK_CRITICAL_MAX_RESTARTS,
            restart_base_delay: TASK_RESTART_BASE_DELAY,
            restart_max_delay: TASK_RESTART_BASE_DELAY
                .saturating_mul(1_u32 << TASK_RESTART_EXPONENT_CAP),
        }
    }

    /// 降级任务策略。
    pub const fn for_degraded() -> Self {
        Self {
            task_class: TaskClass::Degraded,
            max_restarts: TASK_DEGRADED_MAX_RESTARTS,
            restart_base_delay: TASK_RESTART_BASE_DELAY,
            restart_max_delay: TASK_RESTART_BASE_DELAY
                .saturating_mul(1_u32 << TASK_RESTART_EXPONENT_CAP),
        }
    }

    /// 策略绑定的任务分类。
    pub const fn task_class(self) -> TaskClass {
        self.task_class
    }

    /// 最大重启次数（不含最终退出本次）。
    pub const fn max_restarts(self) -> u32 {
        self.max_restarts
    }
}
