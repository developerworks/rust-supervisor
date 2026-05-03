use tokio::sync::watch;
use tokio::task::JoinHandle;

/// 子任务运行句柄与关闭信号。
#[derive(Debug)]
pub struct SupervisedTaskHandle {
    pub(super) handle: Option<JoinHandle<()>>,
    pub(super) shutdown_tx: watch::Sender<bool>,
}
