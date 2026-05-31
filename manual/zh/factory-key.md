# factory_key 配置说明

语言: [English](../en/factory-key.html)

## 1. 结论

`factory_key` 是 YAML(配置文件格式) 里的任务工厂 key(键). 它的值是配置文件和 Rust(系统编程语言) 代码约定的名字, 例如 `api_server`. 它把配置文件中的 worker(工作任务) 声明连接到 Rust(系统编程语言) 代码里注册的 `TaskFactory`(任务工厂).

配置文件只保存声明, 不保存可执行 closure(闭包). 真正的任务启动逻辑必须由 Rust(系统编程语言) 代码提供.

## 2. 解决的问题

Supervisor(监督器) 任务树可以通过配置文件声明子任务. 但是 `async_worker`(异步工作任务) 和 `blocking_worker`(阻塞工作任务) 在真正启动时需要可执行的 `TaskFactory`(任务工厂). `TaskFactory`(任务工厂) 本质上包含 Rust(系统编程语言) 代码和 closure(闭包), 不能安全地直接写入 YAML(配置文件格式).

`factory_key` 解决这个边界问题. 配置文件写一个约定好的 key(键), Rust(系统编程语言) 代码用同一个 key(键) 注册任务工厂. 启动前, 系统把二者绑定起来.

## 3. 配置写法

`children.yaml` 可以这样声明 worker(工作任务):

```yaml
- name: api
  kind: async_worker
  factory_key: api_server

- name: exporter
  kind: blocking_worker
  factory_key: report_exporter
```

这里的 `api_server` 和 `report_exporter` 不是函数名. 它们是配置层任务工厂 key(键). Rust(系统编程语言) 代码必须注册同名 `TaskFactory`(任务工厂).

## 4. Rust 代码侧注册流程

Rust(系统编程语言) 代码通过 `TaskFactoryRegistry`(任务工厂注册表) 建立 key(键) 到 `TaskFactory`(任务工厂) 的映射.

```rust
use rust_supervisor::spec::child::TaskKind;
use rust_supervisor::task::factory::{TaskResult, service_fn};
use rust_supervisor::task::factory_registry::{
    TaskFactoryDescriptor, TaskFactoryRegistry,
};
use std::sync::Arc;

let mut registry = TaskFactoryRegistry::new();

registry.register(TaskFactoryDescriptor::new(
    "api_server",
    "API Server",
    "Runs the API service.",
    [TaskKind::AsyncWorker],
    Arc::new(service_fn(|_ctx| async { TaskResult::Succeeded })),
))?;

registry.register(TaskFactoryDescriptor::new(
    "report_exporter",
    "Report Exporter",
    "Runs blocking export work.",
    [TaskKind::BlockingWorker],
    Arc::new(service_fn(|_ctx| async { TaskResult::Succeeded })),
))?;
```

`TaskFactoryDescriptor`(任务工厂描述符) 同时保存 3 类信息:

- `key`: 配置文件使用的任务工厂 key(键).
- `title` 和 `description`: Schema(结构定义) 自动补全时展示给编辑器的说明.
- `allowed_kinds`: 允许使用这个工厂的任务类型, 例如 `TaskKind::AsyncWorker` 或 `TaskKind::BlockingWorker`.

## 5. 启动时绑定流程

配置加载后, `factory_key` 仍然只是字符串. 启动前需要把字符串解析为真正的 `TaskFactory`(任务工厂).

当前绑定路径是:

1. `ConfigState`(配置状态) 读取 YAML(配置文件格式) 子任务声明.
2. `to_supervisor_spec_with_factories` 使用 `TaskFactoryRegistry`(任务工厂注册表) 绑定 worker(工作任务).
3. `bind_task_factories` 检查每个 worker(工作任务) 的 `factory_key`.
4. 注册表找到对应 `TaskFactory`(任务工厂) 后, 把它写入 `ChildSpec`(子任务规格).
5. `Supervisor`(监督器) 启动时只看到已经绑定好的可执行任务工厂.

绑定规则是:

- worker(工作任务) 必须声明 `factory_key`.
- Supervisor(监督器) 子节点不能声明 `factory_key`.
- 未注册的 `factory_key` 会导致配置错误.
- 工厂不支持当前 `TaskKind`(任务类型) 时会导致配置错误.

## 6. 自动补全生成流程

自动补全依赖 JSON Schema(JSON 结构定义). 当前实现不是重写 rust-config-tree(配置树库) 的 Schema(结构定义) 生成逻辑, 而是在基础 Schema(结构定义) 生成之后做后处理.

流程是:

1. `generate-template` 或 `generate-schema` 调用 rust-config-tree(配置树库) 生成基础 Schema(结构定义).
2. `supervisor_schema_targets_with_factory_registry` 取得 root schema(根结构定义) 和 split-section schema(拆分配置段结构定义).
3. 每个 Schema(结构定义) 先解析成 `serde_json::Value`(JSON 值).
4. `inject_factory_key_completions_if_present` 查找 `factory_key` 字段.
5. 系统把 `TaskFactoryRegistry`(任务工厂注册表) 中的 key(键) 写入 `oneOf`(枚举补全候选).
6. Schema(结构定义) 被重新序列化并写入目标文件.

生成后, `children.schema.json` 的 `factory_key` 字段会包含类似内容:

```json
{
  "factory_key": {
    "description": "TaskFactory registry key used to bind worker children before startup.",
    "oneOf": [
      {
        "const": "api_server",
        "description": "Runs the API service.",
        "title": "API Server"
      },
      {
        "const": "report_exporter",
        "description": "Runs blocking export work.",
        "title": "Report Exporter"
      }
    ],
    "type": [
      "string",
      "null"
    ]
  }
}
```

编辑器读取 `children.yaml` 顶部的 yaml-language-server(YAML 语言服务器) Schema(结构定义) 指令后, 可以对 `factory_key` 给出候选值.

## 7. 命令行为

执行模板生成:

```bash
target/debug/rust-tokio-supervisor generate-template
```

这个命令会生成配置模板, 同时生成带补全信息的 Schema(结构定义).

执行 Schema(结构定义) 生成:

```bash
target/debug/rust-tokio-supervisor generate-schema
```

这个命令只生成 Schema(结构定义), 也会包含 `factory_key` 的 `oneOf`(枚举补全候选).

## 8. 当前边界

- `factory_key` 只是配置层声明, 不是可执行代码.
- 自动补全候选来自当前命令使用的 `TaskFactoryRegistry`(任务工厂注册表).
- 如果 Rust(系统编程语言) 代码没有注册某个 key(键), 配置文件写这个 key(键) 也无法启动.
- Schema(结构定义) 自动补全只能帮助编辑器提示合法候选, 不能替代启动前绑定校验.
- 运行时添加子任务也会经过同一类绑定校验, 避免配置绕过注册表.
