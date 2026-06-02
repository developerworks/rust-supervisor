# 拆分配置与透明数组 Section

语言: [English](../en/split-config.html)

## 概述

`SupervisorConfig`(监督器配置) 的 `groups`(分组) 和 `children`(子任务) 使用 **透明数组 Section**(transparent array section). 它们在 YAML(数据序列化格式) 里表现为数组, 在 Rust 里通过 `items` 字段存储, 加载与模板生成由 `rust-config-tree` 0.3.0 处理.

一句话: 单文件里写 `children: [...]`, split 文件里只写 `[...]`, 不需要 `items:` 包裹.

## 适用字段

| 字段       | Rust 类型               | split 文件      | section schema(配置段结构定义) 顶层类型 |
| ---------- | ----------------------- | --------------- | --------------------------------------- |
| `groups`   | `GroupsConfigSection`   | `groups.yaml`   | `array`                                 |
| `children` | `ChildrenConfigSection` | `children.yaml` | `array`                                 |

访问子项:

```rust
config.children.len();
config.children.as_slice();
config.groups.as_slice();
```

## 单文件写法

根配置里直接写数组:

```yaml
groups:
  - name: core
children:
  - name: api
    kind: async_worker
```

## Split 写法

根配置引用 split 文件:

```yaml
include:
  - groups.yaml
  - children.yaml
supervisor:
  strategy: OneForAll
policy:
  child_restart_limit: 10
  # ... 其余 policy / shutdown / observability 字段
```

`groups.yaml` 与 `children.yaml` **只写数组体**, 不要写 section 根键或 `items:`:

```yaml
# groups.yaml
- name: core
  budget:
    window_secs: 60
    max_burst: 10
    recovery_rate_per_sec: 0.5
```

```yaml
# children.yaml
- name: worker
  kind: async_worker
  criticality: optional
  restart_policy: permanent
```

仓库参考:

- 生成模板目录: `config/supervisor_config/`
- 示例配置: `examples/config/split/`
- 可运行示例: `cargo run --example split_config_supervisor`

## 支持的 YAML 形状

加载器兼容三种写法:

| 形状            | 示例                                 |
| --------------- | ------------------------------------ |
| 透明数组        | `children: [{ name: api }]`          |
| body-only split | `children.yaml` 内只有 `- name: api` |
| 旧式 items 包裹 | `children:\n  items: [...]`          |

不要使用 flow 风格 `[{ name: worker }]`. 模板生成会输出 block YAML(块状 YAML).

## CLI(命令行)

使用二进制 `rust-tokio-supervisor` 或 `cargo run`. 子命令在顶层, 没有 `config` 前缀. `--config` 是 `run` 与 `validate-config` 子命令的参数, 不是全局选项.

```bash
# 校验并打印摘要(默认 run 子命令)
cargo run -- run --config examples/config/supervisor.yaml

# 校验完整配置树(含 include 合并与运行时校验)
cargo run -- validate-config --config examples/config/split/supervisor.yaml

# 生成 split 模板(含 groups.yaml / children.yaml)
# 模板来源默认为 examples/config/supervisor.yaml
cargo run -- generate-template \
  --output config/supervisor_config/supervisor_config.example.yaml

# 生成 schema(结构定义)
cargo run -- generate-schema \
  --output config/supervisor_config/supervisor.schema.json
```

生成后 **不需要** 再手动去掉 `children:` 或把 flow 数组改成 block 形式, 库内已完成 normalize(规范化).

## 代码加载

```rust
use rust_config_tree::config::load_config;
use rust_supervisor::config::{
    configurable::SupervisorConfig,
    loader::load_config_from_yaml_file,
};

// 直接得到 SupervisorConfig
let config = load_config::<SupervisorConfig>("supervisor.yaml")?;

// 或经过校验得到 ConfigState(配置状态)
let state = load_config_from_yaml_file("supervisor.yaml")?;
```

## 运行时 default(默认值) 与模板样例

| 场景                        | `children` 结果                                                     |
| --------------------------- | ------------------------------------------------------------------- |
| 配置文件完全省略 `children` | 运行时 `[]`, 不会注入模板样例 `worker`                              |
| 执行 `generate-template`    | `children.yaml` 可含 `worker` 样例(来自 `#[config(default = ...)]`) |
| split 文件 body-only        | 正常加载数组内容                                                    |

`groups` 模板 default 为 `[]`.

## IDE(集成开发环境) 补全

split 文件绑定 section schema:

```yaml
# yaml-language-server: $schema=./children.schema.json

- name: worker
  kind: async_worker
```

`children.schema.json` 顶层是 `array`, 不是 `{ items: [...] }` 对象.

## 在新项目中复用

若在自己的 crate(包) 里新增透明数组 Section:

1. 为每个透明数组 Section 声明独立 Struct(结构体), 字段为 `items: Vec<T>`, 并实现透明数组的 `Serialize`, `Deserialize`, `JsonSchema` 与访问方法(可参考 `ChildrenConfigSection`, `GroupsConfigSection`).
2. 在根配置字段上加 `#[schemars(extend("x-tree-split" = true, "x-tree-transparent-array" = true))]`.
3. 实现 `ConfigSchema::include_paths`.
4. 用 `load_config`, `write_config_templates`, `write_config_schemas` 加载与生成.

详见 [rust-config-tree 文档](https://github.com/developerworks/rust-config-tree) 中 `x-tree-transparent-array` 说明.
