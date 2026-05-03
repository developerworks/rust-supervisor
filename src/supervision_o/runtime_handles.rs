use tokio::sync::mpsc;

use crate::supervision_o::{
    enums::TaskLifecycleEvent, supervised_task_handle::SupervisedTaskHandle,
};

use super::runtime_trace_context::RuntimeTraceContext;
/// 主运行时持有的全部受监督任务句柄。
///
/// 该结构体聚合了主机器人进程里的后台任务句柄，
/// 让启动层只需要持有一个对象，就能完成：
/// - 关键任务退出监听；
/// - 全量后台任务关停；
/// - 统一关联 `run_id` / `session_id` 日志。
#[derive(Debug)]
pub struct RuntimeHandles {
    pub trace_context: RuntimeTraceContext,
    pub market_data_handle: SupervisedTaskHandle,
    pub lifecycle_events: mpsc::UnboundedReceiver<TaskLifecycleEvent>,
}
