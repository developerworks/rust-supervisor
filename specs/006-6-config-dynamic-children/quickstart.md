# Quickstart(快速开始): 配置声明与动态子任务治理

**Branch(分支)**: `006-6-config-dynamic-children` | **Date(日期)**: 2026-05-19

## Overview(概述)

本切片扩展 YAML 配置 schema 支持 9 类字段(children, dependencies, health, readiness, resource limits, command permissions, environment, secrets reference, restart budgets), 实现 add_child 动态追加事务, 并建立审计对账机制.

## 现有代码入口

| 文件                         | 说明                                     |
| ---------------------------- | ---------------------------------------- |
| `src/config/loader.rs`       | YAML 配置加载器                          |
| `src/config/configurable.rs` | `SupervisorConfig` 定义                  |
| `src/config/state.rs`        | `ConfigState` 运行时状态                 |
| `src/spec/child.rs`          | `ChildSpec`, `TaskKind`, `RestartPolicy` |
| `src/spec/supervisor.rs`     | `SupervisorSpec`                         |
| `src/tree/order.rs`          | 拓扑序与依赖解析                         |

## 新增代码预期位置

| 文件                                    | 说明                            |
| --------------------------------------- | ------------------------------- |
| `src/spec/child_declaration.rs`         | `ChildDeclaration` 解析与校验   |
| `src/config/state.rs`                   | add_child 事务(commit/rollback) |
| `src/tree/order.rs`                     | 动态拓扑更新 + 环路检测         |
| `src/event/payload.rs`                  | 新增事件变体                    |
| `tests/golden_yaml_consistency_test.rs` | golden YAML 一致性              |
| `tests/add_child_transaction_test.rs`   | 事务原子性                      |
| `tests/topology_concurrent_test.rs`     | 并发隔离性                      |

## 配置示例

```yaml
supervisor:
  path: "/"
  strategy: one_for_one
  children:
    - name: "web-server"
      kind: async_worker
      criticality: critical
      restart_policy: permanent
      dependencies: ["database", "cache"]
      health_check:
        check_interval_secs: 10
      environment:
        - name: "PORT"
          value: "8080"
```

## 验证

```bash
cargo test --test golden_yaml_consistency_test
cargo test --test add_child_transaction_test
cargo test --test topology_concurrent_test
cargo test
```
