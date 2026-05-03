use crate::supervision_o::task_supervision::generate_runtime_id;

use super::runtime_trace_context::RuntimeTraceContext;

impl RuntimeTraceContext {
    /// 创建一个本地唯一的运行追踪上下文。
    ///
    /// 生成规则为 `前缀-毫秒时间戳-单进程自增序号`，
    /// 目标是在不依赖外部服务的前提下，提供可读且足够唯一的日志关联键。
    pub fn new() -> Self {
        Self {
            startup_id: generate_runtime_id("run"),
            supervisor_id: generate_runtime_id("session"),
        }
    }
}

impl Default for RuntimeTraceContext {
    fn default() -> Self {
        Self::new()
    }
}
