//! Demonstrates step 02: mounting one service under a supervisor runtime.
//! 演示 step 02: 在 supervisor(监督者) runtime(运行时) 下挂载一个 Service(服务).

use rust_supervisor::error::types::SupervisorError;
use rust_supervisor::role::context::service::ServiceContext;
use rust_supervisor::role::result::service::ServiceResult;
use rust_supervisor::runtime::supervisor::Supervisor;
use rust_supervisor::spec::child::ChildSpec;
use rust_supervisor::spec::supervisor_builder::SupervisorSpecBuilder;
use rust_supervisor_macros::service;
use std::time::Duration;
use tokio::sync::mpsc;

// Define the shared example result type.
// 定义本示例共用的结果类型.
type ExampleResult = Result<(), rust_supervisor::error::types::SupervisorError>;

/// Service contract used by the step 02 example.
/// Step 02 uses the macro-generated runtime adapter.
struct Step02Service {
    /// Channel used to publish service lifecycle facts.
    events: mpsc::UnboundedSender<String>,
}

// Attach the service role contract macro.
// 使用 service role(服务角色) contract(契约) macro(宏).
#[service(id = "step-02-service", name = "Step 02 Service")]
impl Step02Service {
    /// Initializes one service attempt.
    ///
    /// # Arguments
    ///
    /// - `ctx`: Service context used to report readiness and heartbeat state.
    ///
    /// # Returns
    ///
    /// Returns success after readiness is visible to the runtime.
    async fn init(&mut self, ctx: &ServiceContext) -> ServiceResult<()> {
        // Mark the service as ready for the supervisor.
        // 向 supervisor(监督者) 标记 Service(服务) 已 ready(就绪).
        ctx.ready();
        // Emit a heartbeat for liveness observation.
        // 发送 heartbeat(心跳) 供 liveness(存活) observation(观察) 使用.
        ctx.heartbeat();
        // Publish the service initialization fact.
        // 发布 Service(服务) 初始化事实.
        let _ignored = self
            .events
            .send(format!("service initialized: child={}", ctx.child_id()));
        Ok(())
    }

    /// Runs one service attempt until shutdown is requested.
    ///
    /// # Arguments
    ///
    /// - `ctx`: Service context used to observe shutdown and emit heartbeat state.
    ///
    /// # Returns
    ///
    /// Returns success after the service observes cooperative shutdown.
    async fn run(&mut self, ctx: &ServiceContext) -> ServiceResult<()> {
        // Build a periodic running loop for the long-lived service.
        // 为长期运行的 Service(服务) 构建周期性 running(运行) loop(循环).
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        // Track running ticks for observable output.
        // 记录 running(运行) tick(节拍), 便于观察输出.
        let mut tick = 0_u64;
        // Keep the service alive until shutdown.
        // 保持 Service(服务) 存活, 直到收到 shutdown(关闭) 信号.
        loop {
            // Wait for either shutdown or the next service tick.
            // 等待 shutdown(关闭) 信号, 或下一次 Service(服务) tick(节拍).
            tokio::select! {
                // Stop cooperatively when the runtime cancels this attempt.
                // 当 runtime(运行时) 取消本次 attempt(尝试) 时, 协作式停止.
                _ = ctx.wait_shutdown() => {
                    // Leave the lifecycle body and let the shutdown hook run.
                    // 退出 lifecycle(生命周期) 主体, 并让 shutdown(关闭) hook(钩子) 运行.
                    return Ok(());
                }
                // Emit one running tick.
                // 发出一次 running(运行) tick(节拍).
                _ = interval.tick() => {
                    // Advance the tick counter.
                    // 递增 tick(节拍) 计数器.
                    tick += 1;
                    // Emit heartbeat for liveness observation.
                    // 发送 heartbeat(心跳) 供 liveness(存活) observation(观察) 使用.
                    ctx.heartbeat();
                    // Publish one running fact for the example observer.
                    // 向示例 observer(观察者) 发布一条 running(运行) 事实.
                    let _ignored = self.events.send(format!(
                        "service running: child={} tick={tick}",
                        ctx.child_id()
                    ));
                }
            }
        }
    }

    /// Shuts down one service attempt.
    ///
    /// # Arguments
    ///
    /// - `ctx`: Service context used to report final lifecycle facts.
    ///
    /// # Returns
    ///
    /// Returns success after the stopping fact has been published.
    async fn shutdown(&mut self, ctx: &ServiceContext) -> ServiceResult<()> {
        // Publish the service stopping fact.
        // 发布 Service(服务) 正在 stopping(停止) 的事实.
        let _ignored = self
            .events
            .send(format!("service stopping: child={}", ctx.child_id()));
        Ok(())
    }
}

// Use the Tokio runtime for the asynchronous example.
// 使用 Tokio runtime(运行时) 运行这个 async(异步) 示例.
#[tokio::main]
/// Runs the step 02 supervisor with service example.
/// 运行 step 02 的 supervisor(监督者) 与 Service(服务) 示例.
async fn main() -> ExampleResult {
    // Build a channel that receives service lifecycle facts.
    // 创建用于接收 Service(服务) 生命周期事实的 channel(通道).
    let (service_event_sender, mut service_events) = mpsc::unbounded_channel();

    // Build one service child before creating the supervisor tree.
    // 在创建 supervisor(监督者) 树之前, 先构建一个 Service(服务) child(子任务).
    let service_child = service_child(service_event_sender)?;

    // Mount the service child under the root supervisor specification.
    // 把 Service(服务) child(子任务) 挂到 root(根) supervisor(监督者) 规格下.
    let spec = SupervisorSpecBuilder::root(Vec::new())
        // Append the service child under the supervisor.
        // 把 Service(服务) child(子任务) 追加到 supervisor(监督者) 下.
        .child(service_child)
        // Validate and finish the supervisor specification.
        // 校验并完成 supervisor(监督者) 规格构建.
        .build()?;

    // Start the supervisor runtime with the mounted service.
    // 启动已挂载 Service(服务) 的 supervisor(监督者) runtime(运行时).
    let handle = Supervisor::start(spec).await?;

    // Wait until the service reports readiness.
    // 等待 Service(服务) 上报 readiness(就绪) 状态.
    let initialized = service_events.recv().await.ok_or_else(|| {
        // Convert a closed channel into the example error type.
        // 把已关闭的 channel(通道) 转成示例错误类型.
        SupervisorError::fatal_config("service initialization channel closed")
    })?;

    // Print the service initialization fact.
    // 打印 Service(服务) 初始化事实.
    println!("step=02 {initialized}");

    // Query the current runtime state after service startup.
    // 在 Service(服务) 启动后查询当前 runtime(运行时) 状态.
    let current_state = handle.current_state().await?;

    // Print the supervisor state that contains the service child.
    // 打印包含 Service(服务) child(子任务) 的 supervisor(监督者) 状态.
    println!("step=02 supervisor_state={current_state:#?}");

    // Print the long-running service hint for the operator.
    // 向操作者提示 Service(服务) 会长期运行, 直到按下 Ctrl+C.
    println!("step=02 service running until Ctrl+C");

    // Build periodic observation ticks for the long-running service.
    // 为长期运行的 Service(服务) 创建周期性 observation(观察) tick(节拍).
    let mut observation_interval = tokio::time::interval(Duration::from_secs(1));
    // Keep the service running until the operator requests shutdown.
    // 保持 Service(服务) 运行, 直到操作者请求 shutdown(关闭).
    loop {
        // Wait for either an operator signal or the next observation tick.
        // 等待操作者信号, 或下一次 observation(观察) tick(节拍).
        tokio::select! {
            // Stop the example only when the operator sends Ctrl+C.
            // 仅在操作者发送 Ctrl+C 时停止示例.
            signal = tokio::signal::ctrl_c() => {
                // Convert signal errors into the example error type.
                // 把 signal(信号) 错误转成示例错误类型.
                signal.map_err(|error| {
                    SupervisorError::fatal_config(format!(
                        "failed to receive Ctrl+C signal: {error}"
                    ))
                })?;
                // Print the operator stop signal.
                // 打印操作者发出的停止 signal(信号).
                println!("step=02 operator signal=ctrl_c");
                // Leave the persistent running loop.
                // 退出持续运行的 loop(循环).
                break;
            }
            // Print periodic service observation.
            // 周期性打印 Service(服务) observation(观察) 输出.
            _ = observation_interval.tick() => {
                // Print service facts emitted while the service is running.
                // 打印 Service(服务) 运行期间发出的事实.
                drain_service_events(&mut service_events);
            }
        }
    }
    // Request shutdown for the supervisor tree and mounted service.
    // 请求关闭 supervisor(监督者) 树以及已挂载的 Service(服务).
    handle
        // Attach operator metadata to the shutdown command.
        // 为 shutdown(关闭) 命令附加操作者 metadata(元数据).
        .shutdown_tree("operator", "step 02 supervisor with service complete")
        // Wait until the shutdown command finishes.
        // 等待 shutdown(关闭) 命令执行完成.
        .await?;
    // Drain service facts emitted during cooperative shutdown.
    // 清空 cooperative shutdown(协作式关闭) 期间发出的 Service(服务) 事实.
    drain_service_events(&mut service_events);
    // Query the current runtime state after shutdown.
    // 在 shutdown(关闭) 后查询当前 runtime(运行时) 状态.
    let current_state = handle.current_state().await?;
    // Print the stopped supervisor runtime state.
    // 打印已停止的 supervisor(监督者) runtime(运行时) 状态.
    println!("step=02 after_shutdown={current_state:#?}");
    // Finish the example successfully.
    // 示例成功结束.
    Ok(())
}

/// Builds the service child mounted under the supervisor.
/// 构建挂载在 supervisor(监督者) 下的 Service(服务) child(子任务).
///
/// # Arguments
///
/// # 参数
///
/// - `events`: Channel used to publish service lifecycle facts.
/// - `events`: 用于发布 Service(服务) 生命周期事实的 channel(通道).
///
/// # Returns
///
/// # 返回值
///
/// Returns a validated service [`ChildSpec`].
/// 返回已校验的 Service(服务) [`ChildSpec`](子任务规格).
fn service_child(events: mpsc::UnboundedSender<String>) -> Result<ChildSpec, SupervisorError> {
    // Build the service role from the macro-generated child specification.
    // 通过 macro(宏) 生成的 child specification(子任务规格) 构建 Service(服务) role(角色).
    let mut child = Step02Service { events }.child_spec()?;
    // Add a diagnostic tag.
    // 添加 diagnostic(诊断) tag(标签).
    child.tags.push("step-02".to_owned());
    // Return the final child specification.
    // 返回最终的 child(子任务) specification(规格).
    Ok(child)
}

/// Drains service lifecycle facts that are already available.
/// 清空当前已到达的 Service(服务) 生命周期事实.
///
/// # Arguments
///
/// # 参数
///
/// - `events`: Receiver for service lifecycle facts.
/// - `events`: 接收 Service(服务) 生命周期事实的 receiver(接收端).
///
/// # Returns
///
/// # 返回值
///
/// This function does not return a value.
/// 本函数没有返回值.
fn drain_service_events(events: &mut mpsc::UnboundedReceiver<String>) {
    // Drain all service facts that arrived before this call.
    // 清空本次调用前已到达的全部 Service(服务) 事实.
    while let Ok(event) = events.try_recv() {
        // Print one service lifecycle fact.
        // 打印一条 Service(服务) 生命周期事实.
        println!("step=02 drain_service_events {event}");
    }
}
