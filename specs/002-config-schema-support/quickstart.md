# Quickstart(快速开始): 配置结构体模式支持

## Goal(目标)

本 quickstart(快速开始) 说明 crate user(crate 使用者) 如何复用 `SupervisorConfig`(监督器配置) 生成 schema(结构模式),生成官方 YAML(数据序列化格式) template(模板),加载配置,并在配置错误时得到 startup rejection(启动拒绝)。

## 1. Add dependencies(添加依赖)

使用者项目需要依赖本 crate(包)。如果使用者要在自己的项目中直接生成 schema(结构模式),需要在自己的项目中使用 `schemars`(结构模式库)。如果使用者要在自己的项目中生成 template(模板),需要使用 `rust-config-tree`(配置树库) 和 `confique`(配置库) 对应能力。

```toml
[dependencies]
rust-tokio-supervisor = "0.1.1"
schemars = { version = "1", features = ["derive"] }
rust-config-tree = "0.1.9"
```

## 2. Load YAML config(加载 YAML 配置)

```rust
use rust_supervisor::config::loader::load_config_from_yaml_file;

fn main() -> Result<(), rust_supervisor::error::types::SupervisorError> {
    let state = load_config_from_yaml_file("examples/config/supervisor.yaml")?;
    let spec = state.to_supervisor_spec()?;
    spec.validate()?;
    Ok(())
}
```

## 3. Start runtime from config(从配置启动运行时)

```rust
use rust_supervisor::runtime::supervisor::Supervisor;

#[tokio::main]
async fn main() -> Result<(), rust_supervisor::error::types::SupervisorError> {
    let handle = Supervisor::start_from_config_file("examples/config/supervisor.yaml").await?;
    let _state = handle.current_state().await?;
    Ok(())
}
```

`Supervisor::start_from_config_file`(从配置文件启动) 必须先完成 YAML(数据序列化格式) 解析,semantic validation(语义校验) 和 `SupervisorSpec::validate()`(监督器规格校验),然后才能创建 runtime channel(运行时通道) 和 control loop(控制循环)。

## 4. Generate schema(生成结构模式)

```rust
use rust_supervisor::config::configurable::SupervisorConfig;

fn main() {
    let schema = schemars::schema_for!(SupervisorConfig);
    let schema_json = serde_json::to_string_pretty(&schema).unwrap();
    assert!(schema_json.contains("supervisor"));
    assert!(!schema_json.contains("x-tree-split"));
}
```

## 5. Keep official template single-file(保持官方模板为单文件)

官方 template(模板) 默认只生成一个 root YAML template target(根 YAML 模板目标)。本 crate(包) 不添加 `x-tree-split`(树形拆分扩展)。使用者如果需要拆分配置,应在自己的项目中包装 `SupervisorConfig`(监督器配置),并自行声明 schema extension(结构模式扩展)。

## 6. Verify startup rejection(验证启动拒绝)

以下配置必须在 runtime startup(运行时启动) 前失败:

- 缺失必填项。
- 非法 enum value(枚举值)。
- 零值 capacity(容量)。
- 零值 timeout(超时)。
- 越界 `jitter_ratio`(抖动比例)。
- 反向 backoff(退避)。

验证命令:

```bash
cargo fmt --all --check
cargo test configurable_schema_test
cargo test configurable_template_test
cargo test no_baked_in_tree_split_test
cargo test invalid_config_rejected_test
cargo test supervisor_config_test
cargo clippy --all-targets --all-features -- -D warnings
```

## 7. Documentation sync(文档同步)

任何配置字段变化都必须同步 README(说明文档),manual(手册),quickstart(快速开始),examples(示例程序) 和 contracts(契约)。documentation sync check(文档同步检查) 必须验证这些文档和 `SupervisorConfig`(监督器配置) 保持一致。
