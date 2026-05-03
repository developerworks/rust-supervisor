use super::supervised_task_handle::SupervisedTaskHandle;

impl SupervisedTaskHandle {
    /// 请求关闭任务。
    ///
    /// 该方法只发送关闭信号，具体的 child task 中断由监督任务统一执行。
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
    }

    /// 关闭任务并等待监督任务退出。
    ///
    /// 录制流程在发布产物前需要确认所有后台写入者已经停稳，
    /// 否则 staging 目录裁剪可能与最后一批落盘竞争。
    pub async fn shutdown_and_wait(&mut self) {
        let _ = self.shutdown_tx.send(true);
        let Some(handle) = self.handle.take() else {
            return;
        };

        let _ = handle.await;
    }
}
