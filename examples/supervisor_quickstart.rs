//! Demonstrates the minimal supervisor quickstart flow.

// Import the YAML configuration loader.
use rust_supervisor::config::loader::load_config_from_yaml_file;
// Import the supervisor runtime entry point.
use rust_supervisor::runtime::supervisor::Supervisor;

// Define the shared example result type.
type ExampleResult = Result<(), rust_supervisor::error::types::SupervisorError>;

// Use the Tokio runtime for the asynchronous example.
#[tokio::main]
// Return typed supervisor errors from the example.
async fn main() -> ExampleResult {
    // Load centralized YAML configuration.
    // 加载中央 YAML 配置
    let state = load_config_from_yaml_file("examples/config/supervisor.yaml")?;

    // Derive the supervisor specification from configuration.
    // 从配置派生 Supervisor 规范
    let spec = state.to_supervisor_spec()?;

    // Start the supervisor runtime from the specification.
    // 从规范启动 Supervisor 运行时
    let supervisor_handle = Supervisor::start(spec).await?;

    // Query the current runtime state.
    // 查询当前运行时状态
    let current = supervisor_handle.current_state().await?;

    // Print the current state for the learner.
    // 打印当前状态供学习者参考
    println!("{current:#?}");

    // Use the runtime handle for the shutdown request.
    // 使用运行时句柄进行关闭请求
    supervisor_handle
        // Request tree shutdown with audit metadata.
        // 请求树关闭并提供审计元数据
        .shutdown_tree("operator", "quickstart complete")
        // Wait for the shutdown command result.
        // 等待关闭命令结果
        .await?;

    // Query the state after shutdown.
    // 关闭后查询状态
    let current = supervisor_handle.current_state().await?;
    println!("{current:#?}");

    Ok(())
}
