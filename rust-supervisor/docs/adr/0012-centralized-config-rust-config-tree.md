# ADR-012: 配置集中化 (rust-config-tree 作为唯一入口)

- **日期**: 2026-05-05
- **状态**: Accepted

## 背景

运行时行为 (阈值、窗口、超时、退避、抖动、容量、开关、预算) 不应散落在模块内部. 需要集中配置入口.

## 可选方案

- 方案 A: 每个模块自己定义默认值, 运行时通过环境变量覆盖.
- 方案 B: 使用 rust-config-tree v0.1.9 作为唯一集中配置入口, YAML 格式.
- 方案 C: 使用 TOML 或 JSON 作为主配置格式.

## 决策

选择方案 B.

## 理由

- rust-config-tree v0.1.9 提供集中加载、校验和包含树支持.
- `SupervisorConfig` 同时支持 `confique::Config`, `schemars::JsonSchema`, `serde::Serialize/Deserialize`.
- 模块内部不得保存可调配置默认值, 消除配置碎片.
- `ConfigState` 加载后不可变, 派生 `SupervisorSpec` 和默认策略.
- 主配置使用 YAML, 示例路径 `examples/config/supervisor.yaml`.

## 后果

- 正面: 配置集中化, 消除硬编码常量.
- 正面: 同一模型用于 YAML 加载、模板生成和 JSON Schema 生成.
- 正面: 缺失配置时失败, 不会使用隐式默认值.
- 负面: 增加 `rust-config-tree` 依赖.
- 负面: 配置变更需重启 supervisor 实例.

## 关联

- 关联 Spec: `specs/002-config-schema-support/`, `specs/001-create-supervisor-core/`
