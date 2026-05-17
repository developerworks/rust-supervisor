# Quickstart(快速开始): 005-2 Work Role Defaults(工作角色默认值)

本文提供 005-2 功能的代码阅读顺序, 配置示例与验收步骤. 默认命令都在仓库根目录执行.

## 1. 源码阅读顺序

按以下顺序阅读当前实现:

1. `src/policy/role_defaults.rs`
   - `WorkRole(工作任务角色)` 定义 5 类角色: `service(常驻服务)`, `worker(工作任务)`, `job(一次性作业)`, `sidecar(辅助任务)`, `supervisor(嵌套监督器)`.
   - `RoleDefaultPolicy(角色默认策略包)` 保存成功退出, 失败退出, 人工停止, 超时和预算耗尽时的默认动作.
   - 当前默认包通过 `RoleDefaultPolicy::for_role()` 取得, 不提供按角色命名的公开常量.
   - `EffectivePolicy(生效策略)` 通过 `EffectivePolicy::for_child()` 或 `EffectivePolicy::merge()` 生成, 并记录 `PolicySource(策略来源)`.

2. `src/spec/child.rs`
   - `ChildSpec(子任务规格)` 包含 `work_role` 和 `sidecar_config`.
   - 这两个字段都有 `serde(default)` 行为, 旧配置缺省时不会因为字段缺失而反序列化失败.
   - 单个 `ChildSpec(子任务规格)` 只检查本地 sidecar(辅助任务)字段是否自洽.

3. `src/spec/supervisor.rs`
   - `SupervisorSpec.validate()` 负责需要兄弟节点上下文的验证.
   - `sidecar_config.primary_child_id(主任务标识)` 指向不存在子任务时拒绝加载.
   - 链式 `sidecar(辅助任务)` 会拒绝加载.
   - `Job + Permanent(一次性作业加永久重启)` 属于语义冲突, 当前版本输出 `WARN(日志警告)` 并允许加载.

4. `src/runtime/pipeline.rs`
   - `evaluate budget(评估预算)` 阶段读取 `restart_execution_plan(重启执行计划)`.
   - 当计划没有显式 `restart_limit(重启次数限制)` 或 `escalation_policy(升级策略)` 时, 管线使用 `EffectivePolicy(生效策略)` 中的角色默认值.
   - `decide action(决定动作)` 阶段按照角色默认动作和预算结论选择 `ProtectionAction(保护动作)`.

5. `src/runtime/control_loop.rs`
   - 控制循环在进程退出后生成 `EffectivePolicy(生效策略)`, 然后调用 `SupervisionPipeline(监督管线)`.
   - 真实运行时会把 6 阶段诊断写入共享 `ObservabilityPipeline(可观察性管道)`.

6. `src/event/payload.rs`
   - `SupervisorEvent(监督事件)` 包含 `work_role`, `used_fallback_default`, `effective_policy_source`.

7. `tests/work_role_defaults_integration.rs`
   - 该文件包含 5 类角色的行为样例, sidecar(辅助任务)校验样例和事件字段样例.

## 2. 配置示例

### 2.1 五类角色声明

```yaml
supervisor:
  id: root
  children:
    - id: web-server
      work_role: service
      command: ["nginx", "-g", "daemon off;"]

    - id: background-worker
      work_role: worker
      command: ["python", "worker.py"]

    - id: migration-job
      work_role: job
      command: ["rails", "db:migrate"]

    - id: primary-app
      work_role: service
      command: ["my-app"]

    - id: log-collector
      work_role: sidecar
      command: ["fluentd"]
      sidecar_config:
        primary_child_id: primary-app
        linked_lifecycle: false

    - id: nested-supervisor
      work_role: supervisor
      supervisor_spec:
        # nested tree definition
```

### 2.2 显式覆写示例

```yaml
supervisor:
  id: root
  children:
    - id: critical-job
      work_role: job
      command: ["critical-task"]
      restart_policy:
        max_restarts: 3
        window_secs: 300

    - id: resilient-service
      work_role: service
      command: ["my-server"]
      backoff_policy:
        strategy: decorrelated_jitter
        base_delay_ms: 100
        max_delay_ms: 10000

    - id: risky-job
      work_role: job
      command: ["risky-task"]
      restart_policy: permanent
      # 当前版本输出 WARN(日志警告), 因为 Job(一次性作业)与 Permanent(永久重启)语义冲突.
```

### 2.3 错误示例

```yaml
- id: orphan-sidecar
  work_role: sidecar
  command: ["fluentd"]
  # 错误: sidecar work_role requires sidecar_config

- id: broken-sidecar
  work_role: sidecar
  command: ["fluentd"]
  sidecar_config:
    primary_child_id: non-existent-service
  # 错误: references unknown primary_child_id

- id: sidecar-1
  work_role: sidecar
  command: ["logger-1"]
  sidecar_config:
    primary_child_id: main-service

- id: sidecar-2
  work_role: sidecar
  command: ["logger-2"]
  sidecar_config:
    primary_child_id: sidecar-1
  # 错误: must not use another sidecar as primary_child_id
```

## 3. 验收步骤

### 3.1 基础检查

```bash
cargo fmt
cargo test --test work_role_defaults_integration
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

### 3.2 行为对照表验收

```bash
cargo test --test work_role_defaults_integration job_success_exit_does_not_request_restart
cargo test --test work_role_defaults_integration service_success_exit_allows_restart
cargo test --test work_role_defaults_integration worker_failure_default_uses_bounded_retry
cargo test --test work_role_defaults_integration worker_default_restart_limit_feeds_budget_evaluation
cargo test --test work_role_defaults_integration sidecar_failure_default_restarts_only_sidecar_scope
cargo test --test work_role_defaults_integration supervisor_role_default_uses_outer_unit_restart_budget
```

预期结果:

- `job(一次性作业)` 成功退出后选择 `SupervisedStop(监督停止)`.
- `service(常驻服务)` 成功退出后选择 `RestartAllowed(允许重启)`.
- `worker(工作任务)` 默认预算进入 `evaluate budget(评估预算)`.
- `sidecar(辅助任务)` 默认只重启自身范围.
- `supervisor(嵌套监督器)` 角色默认包包含外层预算.

### 3.3 冲突检测验收

```bash
cargo test --test work_role_defaults_integration sidecar_missing_config_is_rejected
cargo test --test work_role_defaults_integration sidecar_unknown_primary_is_rejected
cargo test --test work_role_defaults_integration sidecar_chain_is_rejected
cargo test --test work_role_defaults_integration job_permanent_restart_conflict_is_reported
```

预期结果:

- 缺少 `sidecar_config(辅助任务配置)` 的 `sidecar(辅助任务)` 会拒绝加载.
- 指向不存在 `primary_child_id(主任务标识)` 的 `sidecar(辅助任务)` 会拒绝加载.
- 链式 `sidecar(辅助任务)` 会拒绝加载.
- `Job + Permanent(一次性作业加永久重启)` 会产生可读语义冲突.

### 3.4 事件字段验收

```bash
cargo test --test work_role_defaults_integration emitted_pipeline_event_carries_policy_attribution
cargo test --test work_role_defaults_integration supervisor_event_fields_exist_for_policy_source
```

预期结果:

- `SupervisorEvent(监督事件)` 包含 `work_role`.
- `SupervisorEvent(监督事件)` 包含 `used_fallback_default`.
- `SupervisorEvent(监督事件)` 包含 `effective_policy_source`.

## 4. 常见问题

### Q1: 角色缺失时为什么回落到 worker(工作任务)?

`worker(工作任务)` 是保守默认. 它成功退出后停止, 失败后有限重试. 这个默认不会像 `service(常驻服务)` 那样长期重启, 也不会忽略失败重试需求.

### Q2: 能不能混合多个角色的默认策略?

不能. `RoleDefaultPolicy(角色默认策略包)` 原子绑定到单一角色. 用户可以显式覆写字段, 但是不能把多个角色默认值拼成一个隐式策略.

### Q3: 如何让 job(一次性作业)成功后也再次运行?

推荐使用外部调度器启动新的 job(一次性作业)实例. 如果在监督器内把 `job(一次性作业)` 改成永久重启语义, 当前版本会输出 `WARN(日志警告)`.

### Q4: 当前版本是否支持运行中修改 work_role(工作任务角色)?

不支持. `work_role(工作任务角色)` 属于监督单元启动前配置. 运行中需要改变角色时, 调用方必须重建该监督单元, 不能把角色热更新套到已经运行的子任务上.

## 5. 参考资料

- 功能规格: `specs/005-2-work-role-defaults/spec.md`
- 实现计划: `specs/005-2-work-role-defaults/plan.md`
- 研究结论: `specs/005-2-work-role-defaults/research.md`
- 数据模型: `specs/005-2-work-role-defaults/data-model.md`
- 接口契约: `specs/005-2-work-role-defaults/contracts/role-defaults.md`
- 依赖功能: `specs/005-1-failure-policy-reliability/spec.md`
