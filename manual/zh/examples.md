# 示例程序

## 快速开始

```bash
cargo run --example supervisor_quickstart
```

`supervisor_quickstart` 读取 `examples/config/supervisor.yaml`, 派生 `SupervisorSpec`(监督器规格), 启动 supervisor(监督器), 查询 current state(当前状态), 然后关闭整棵树.

## 配置树

```bash
cargo run --example config_tree_supervisor
```

`config_tree_supervisor` 展示 rust-config-tree(集中配置树) v0.1.9 的 YAML(数据序列化格式)配置加载路径, 并打印派生后的 `SupervisorSpec`(监督器规格).

## 重启策略实验

```bash
cargo run --example restart_policy_lab
```

`restart_policy_lab` 展示 `TaskFailure`(任务失败), `TaskFailureKind`(任务失败类别), `RestartPolicy`(重启策略), canonical `spec::supervisor::SupervisionStrategy`(规范归属的监督策略) 和 `RestartDecision`(重启决策) 的基本形状.

## 关闭树

```bash
cargo run --example shutdown_tree
```

`shutdown_tree` 展示 request stop(请求停止), graceful drain(优雅排空), abort stragglers(强制终止拖尾任务)和 reconcile(状态对账)四个阶段, 然后执行 `shutdown_tree`.

## 可观测性探针

```bash
cargo run --example observability_probe
```

`observability_probe` 订阅事件流, 查询当前状态, 打印一个事件, 然后执行关闭. 它用于检查 observability(可观测性)接入路径.

## 监督树故事

```bash
cargo run --example supervisor_tree_story
```

`supervisor_tree_story` 声明 market feed(行情输入), risk engine(风控引擎) 和 audit sink(审计输出) 三个 child(子任务), 展示 dependencies(依赖), tags(标签), criticality(关键程度), explicit readiness(显式就绪), startup order(启动顺序), shutdown order(关闭顺序) 和 `RestForOne`(从失败处开始) restart scope(重启范围).

## 运行时控制故事

```bash
cargo run --example runtime_control_story
```

`runtime_control_story` 启动真实 supervisor(监督器), 执行 `add_child`, `pause_child`, `resume_child`, `quarantine_child`, `current_state`, `subscribe_events` 和 `shutdown_tree`. 它覆盖 operator control(操作员控制) 和 audit event(审计事件) 的组合场景.

## 策略失败矩阵

```bash
cargo run --example policy_failure_matrix
```

`policy_failure_matrix` 对 `Permanent`(永久), `Transient`(瞬时) 和 `Temporary`(临时) restart policy(重启策略) 分别输入 success(成功), external dependency(外部依赖), fatal bug(致命缺陷) 和 panic(恐慌) 退出结果, 同时展示 deterministic jitter(确定性抖动) 和 meltdown tracker(熔断跟踪器).

## 诊断回放

```bash
cargo run --example diagnostic_replay
```

`diagnostic_replay` 构造 deterministic event(确定性事件), 写入 event journal(事件日志缓冲区), 回放 failure(失败), backoff(退避) 和 restart(重启) 事实, 然后生成 metrics(指标) 样本和 `RunSummary`(运行摘要). 它用于排查生产事件后的 replay(回放) 和 report(报告) 路径.
