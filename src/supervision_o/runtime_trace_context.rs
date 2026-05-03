/// 运行时追踪上下文。
///
/// 该对象为单次进程运行生成两类稳定标识：
/// - `startup_id`：本次启动实例标识，用于串联启动到退出的整条链路；
/// - `supervisor_id`：当前监督器实例标识，用于区分不同监督上下文。
///
/// 日志字段仍然统一输出为 `run_id` / `session_id`
/// 以保持现有可观测性字段口径不变。
#[derive(Debug, Clone)]
pub struct RuntimeTraceContext {
    /// 当前进程启动实例 ID。
    pub startup_id: String,
    /// 当前任务监督器实例 ID。
    pub supervisor_id: String,
}
