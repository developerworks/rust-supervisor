//! 任务监督模块内部使用的枚举定义。
//!
//! 这里集中放置监督层自己的事件语义枚举，
//! 避免和 [`structs`](super::structs) 里的运行时句柄、上下文结构混放。

/// 运行时任务分类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskClass {
    /// 关键任务：达到重试上限后应触发上层失败处理。
    Critical,
    /// 降级任务：达到重试上限后允许上层继续运行。
    Degraded,
}

impl TaskClass {
    /// 返回日志标签：`critical` / `degraded`。
    pub const fn label(self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::Degraded => "degraded",
        }
    }
}

/// 任务生命周期事件。
#[derive(Debug)]
pub enum TaskLifecycleEvent {
    /// 任务退出后按策略重建。
    Restarted {
        task_name: &'static str,
        task_class: TaskClass,
        restart_count: u32,
        reason: String,
    },
    /// 达到重试上限并退出。
    Exited {
        task_name: &'static str,
        task_class: TaskClass,
        restart_count: u32,
        reason: String,
    },
}
