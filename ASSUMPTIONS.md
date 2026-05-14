# ASSUMPTIONS(假设记录)

## 执行默认值

- 许可证使用 MIT(麻省理工许可证).
- crate(包) 版本使用 `0.1.0`.
- 主配置格式使用 YAML(数据序列化格式), 示例路径是 `examples/config/supervisor.yaml`.
- rust-config-tree(集中配置树) 版本固定为 v0.1.9.
- 当用户没有提供 GitHub(代码托管平台) 仓库状态时, crates.io(软件包发布平台) 验证使用 `cargo publish --dry-run --allow-dirty`.

## API(接口) 默认值

- `ConfigState`(配置状态) 是集中配置加载后的唯一派生入口.
- `ConfigState::to_supervisor_spec` 派生 `SupervisorSpec`(监督器规格).
- `Supervisor::start` 只接收 `SupervisorSpec`(监督器规格), 不提供 compatibility method(兼容方法).
- `SupervisorHandle`(监督器句柄) 提供 `add_child`, `remove_child`, `restart_child`, `pause_child`, `resume_child`, `quarantine_child`, `shutdown_tree`, `current_state` 和 `subscribe_events`.
- 测试文件统一放在模块自己的 `tests/*_test.rs` 或 `src/tests/*_test.rs`.

## Dashboard(看板) 默认值

- supervisor target(监督器目标进程), relay(中继) 和 UI(用户界面) 使用三个独立目录实现.
- 当前仓库 `~/rust-supervisor` 只拥有 target process IPC(目标进程进程间通信) 和 shared contract(共享契约).
- relay(中继) 实现在 `~/rust-supervisor-relay`, 并通过 dynamic registration(动态注册) 发现目标进程, 不使用静态目标清单.
- UI(用户界面) 实现在 `~/rust-supervisor-ui`, 技术栈固定为 Vue(网页界面框架), shadcn-vue(组件库) 和 Tailwind(样式框架).
- target IPC(目标进程进程间通信) 使用 Unix domain socket(Unix 域套接字) 和 newline-delimited JSON(按行分隔的 JSON 数据).
- `ipc.enabled=false` 或缺少 `ipc` section(配置节) 时, target process(目标进程) 不打开 IPC(进程间通信).
- `ipc.enabled=true` 时, `ipc.path` 和 `registration.relay_registration_path` 必须是 absolute path(绝对路径).
- dynamic registration(动态注册) 只上报 `target_id`, `display_name`, `ipc_path`, `lease_seconds` 和 `supported_commands`.
- event(事件) 和 log(日志) subscription(订阅) 必须由 established dashboard session(已建立看板会话) 触发, registration(注册) 本身不触发主动推送.
- 本次 UI(用户界面) browser test(浏览器测试) 使用 `wss://` relay(中继) URL(统一资源定位符) 和本地 TLS(传输层安全协议) WebSocket(网络套接字协议) 协议测试服务验证交互. 浏览器测试不证明真实 mTLS(双向传输层安全协议认证) 证书链有效.
- 当前仓库的 target-side IPC server(目标侧进程间通信服务端) 提供可测试 dispatcher(分发器), Unix listener(Unix 监听器) bind(绑定) 和命令映射边界. relay(中继) 测试使用真实 Unix domain socket IPC(Unix 域套接字进程间通信) 测试目标覆盖会话门控和转发顺序.

## 发布默认值

- SBOM(软件物料清单) 生成 CycloneDX JSON(CycloneDX JSON 格式) 和 SPDX JSON(SPDX JSON 格式) 两个文件.
- package include(打包包含清单) 使用仓库根锚定路径, 避免把 `.agents` 等开发材料打入 crate(包).
- Cargo(构建工具) 自动生成的 `.cargo_vcs_info.json` 和 `Cargo.toml.orig` 属于正常打包校验输出.
