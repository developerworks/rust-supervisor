# Quickstart(快速开始): 005-2 Work Role Defaults(工作角色默认值)

本文提供 **005-2** 功能的代码阅读顺序、配置示例与验收步骤。贡献者应按本节指引理解实现细节并验证功能正确性。

## 1. 源码阅读顺序

按以下顺序阅读 `src/` 中的模块, 从数据结构到运行时集成逐步深入:

### 1.1 核心数据结构 (Phase 1 设计产物)

1. **`src/policy/role_defaults.rs`** - 工作角色枚举与默认策略包定义
   - **`WorkRole`** 枚举: 五类角色声明
   - **`SidecarConfig`** 结构: 边车主服务绑定配置
   - **`OnSuccessAction`**, **`OnFailureAction`**, **`OnManualStopAction`**, **`OnTimeoutAction`**, **`OnBudgetExhaustedAction`** 枚举: 各场景动作定义
   - **`RoleDefaultPolicyPack`** 结构: 角色默认策略包, 包含五个角色的常量定义 (**`SERVICE_DEFAULT`**, **`WORKER_DEFAULT`**, **`JOB_DEFAULT`**, **`SIDECAR_DEFAULT`**, **`SUPERVISOR_DEFAULT`**)
   - **`PolicySource`** 枚举: 策略来源标识
   - **`EffectivePolicy`** 结构: 合并后的生效策略
   - **`RoleDefaultPolicyPack::for_role()`** 函数: 角色到默认策略包的查找逻辑

2. **`src/spec/child.rs`** - 子任务规格扩展
   - 查看 **`ChildSpec`** 结构新增的 **`work_role: Option<WorkRole>`** 字段
   - 查看 **`sidecar_config: Option<SidecarConfig>`** 字段
   - 理解配置反序列化时的 **`#[serde(default)]`** 标注

3. **`src/event/payload.rs`** - 事件载荷扩展
   - 查看 **`TypedSupervisionEvent`** 结构新增的 **`work_role`**, **`used_fallback_default`**, **`effective_policy_source`** 字段
   - 理解事件序列化与可观察性管道集成

### 1.2 配置加载与验证

4. **`src/config/configurable.rs`** - 配置加载集成
   - 查看角色默认策略如何与用户配置合并
   - 理解三层优先级模型 (用户覆写 > 角色默认 > 全局兜底)
   - 查看冲突检测与警告逻辑

5. **`src/config/yaml.rs`** - YAML 配置解析
   - 查看 **`work_role`** 字段的 YAML 反序列化
   - 查看 **`sidecar_config`** 字段的 YAML 反序列化

### 1.3 运行时集成

6. **`src/runtime/control_loop.rs`** - 控制循环编排
   - 查看 **`prepare_effective_policy()`** 函数: 在 **`evaluate budget`** 之前计算生效策略
   - 查看 **`decide action`** 阶段如何使用 **`EffectivePolicy`**
   - 查看 **`execute action`** 阶段如何写入带角色信息的事件载荷

7. **`src/policy/decision.rs`** - 策略决策引擎
   - 查看 **`PolicyEngine`** 如何读取合并后的生效策略
   - 理解角色默认与用户覆写的合并逻辑

### 1.4 测试夹具

8. **`tests/work_role_defaults_integration.rs`** - 端到端集成测试
   - 查看五类角色的标准验收样例
   - 查看冲突检测与警告日志验证
   - 查看诊断事件字段验证

## 2. 配置示例

### 2.1 基础示例: 五类角色声明

```yaml
supervisor:
  id: root
  children:
    # Service: 常驻服务, 成功退出后自动重启以保持在线
    - id: web-server
      work_role: service
      command: ["nginx", "-g", "daemon off;"]

    # Worker: 工作任务, 成功退出后停止, 失败后有限重试
    - id: background-worker
      work_role: worker
      command: ["python", "worker.py"]

    # Job: 一次性作业, 成功退出后不得自动再起
    - id: migration-job
      work_role: job
      command: ["rails", "db:migrate"]

    # Sidecar: 辅助任务, 需声明主服务绑定
    - id: primary-app
      work_role: service
      command: ["my-app"]

    - id: log-collector
      work_role: sidecar
      command: ["fluentd"]
      sidecar_config:
        primary_child_id: primary-app
        linked_lifecycle: false  # 允许单独重启边车

    # Supervisor: 嵌套监督器, 外层核算预算
    - id: nested-supervisor
      work_role: supervisor
      supervisor_spec:
        # ... nested tree definition ...
```

### 2.2 高级示例: 用户显式覆写

```yaml
supervisor:
  id: root
  children:
    # Job with custom restart limit (覆盖默认重启次数)
    - id: critical-job
      work_role: job
      command: ["critical-task"]
      restart_policy:
        max_restarts: 3  # 用户覆写: 最多重试 3 次
        window_secs: 300

    # Service with custom backoff (覆盖默认退避策略)
    - id: resilient-service
      work_role: service
      command: ["my-server"]
      backoff_policy:
        strategy: decorrelated_jitter  # 使用去相关抖动
        base_delay_ms: 100
        max_delay_ms: 10000

    # WARNING: Semantic conflict example (语义冲突示例)
    - id: risky-job
      work_role: job
      command: ["risky-task"]
      restart_policy: permanent  # ⚠️ 与 Job 角色语义矛盾, 将输出警告日志
```

### 2.3 错误示例: 缺失必需配置

```yaml
# ❌ 错误: Sidecar 未声明 sidecar_config
- id: orphan-sidecar
  work_role: sidecar
  command: ["fluentd"]
  # 配置加载阶段将拒绝并报错: "Sidecar role requires sidecar_config"

# ❌ 错误: Sidecar 引用不存在的 primary_child_id
- id: broken-sidecar
  work_role: sidecar
  command: ["fluentd"]
  sidecar_config:
    primary_child_id: non-existent-service  # 不存在
  # 配置加载阶段将拒绝并报错: "primary_child_id 'non-existent-service' not found"

# ❌ 错误: 链式边车
- id: sidecar-1
  work_role: sidecar
  command: ["logger-1"]
  sidecar_config:
    primary_child_id: main-service

- id: sidecar-2
  work_role: sidecar
  command: ["logger-2"]
  sidecar_config:
    primary_child_id: sidecar-1  # primary 本身是 Sidecar, 禁止
  # 配置加载阶段将拒绝并报错: "Chain sidecar not allowed"
```

## 3. 验收步骤

### 3.1 编译与格式化

```bash
# 格式化全部源码
cargo fmt

# 检查代码风格
cargo clippy --all-targets --all-features -- -D warnings

# 编译项目
cargo build
```

### 3.2 运行单元测试

```bash
# 运行全部测试
cargo test

# 仅运行角色默认相关测试
cargo test role_defaults

# 仅运行配置加载测试
cargo test config
```

### 3.3 运行集成测试

```bash
# 运行端到端集成测试
cargo test --test work_role_defaults_integration

# 详细输出
cargo test --test work_role_defaults_integration -- --nocapture
```

### 3.4 行为对照表验收

为每个角色准备最小示例拓扑, 验证默认行为与契约一致:

#### Test 1: Job 成功退出后不得自动再起

```bash
# 运行 job-success-stop 测试用例
cargo test job_success_exit_no_restart --test work_role_defaults_integration

# 预期结果:
# - Job 进程以退出码 0 退出
# - 监督器不再次启动该进程
# - 事件载荷中 work_role = Job, on_success_exit = Stop
```

#### Test 2: Service 成功退出后允许自动重启

```bash
# 运行 service-success-restart 测试用例
cargo test service_success_exit_restart --test work_role_defaults_integration

# 预期结果:
# - Service 进程以退出码 0 退出
# - 监督器自动再次启动该进程
# - 事件载荷中 work_role = Service, on_success_exit = Restart
```

#### Test 3: Worker 失败后限次数重试

```bash
# 运行 worker-failure-limited-retry 测试用例
cargo test worker_failure_limited_retry --test work_role_defaults_integration

# 预期结果:
# - Worker 进程失败退出
# - 监督器带退避重启, 达到默认重启次数限制后停止
# - 事件载荷中 work_role = Worker, on_failure_exit = RestartWithBackoff
```

#### Test 4: Sidecar 失败时不连带主服务

```bash
# 运行 sidecar-failure-isolated 测试用例
cargo test sidecar_failure_isolated --test work_role_defaults_integration

# 预期结果:
# - Sidecar 进程失败退出
# - 监督器单独重启 Sidecar, 主服务继续运行
# - 事件载荷中 work_role = Sidecar, linked_lifecycle = false
```

#### Test 5: Supervisor 外层核算预算

```bash
# 运行 supervisor-outer-budget-accounting 测试用例
cargo test supervisor_outer_budget_accounting --test work_role_defaults_integration

# 预期结果:
# - 内层监督树失败
# - 外层监督器将整个内层树作为单一单元核算重启次数
# - 事件载荷中 work_role = Supervisor, scopes_triggered 包含外层边界
```

### 3.5 冲突检测验收

#### Test 6: Job + Permanent 重启策略警告

```bash
# 运行 semantic-conflict-warning 测试用例
cargo test semantic_conflict_warning --test work_role_defaults_integration

# 预期结果:
# - 配置加载成功 (警告而非拒绝)
# - WARN 级别日志输出冲突详情
# - 日志包含 child_id, work_role, conflicting_field, user_value, expected_semantic
```

#### Test 7: Sidecar 缺失 sidecar_config 拒绝

```bash
# 运行 sidecar-missing-config-rejected 测试用例
cargo test sidecar_missing_config_rejected --test work_role_defaults_integration

# 预期结果:
# - 配置加载阶段拒绝并报错
# - 错误消息明确指出 "Sidecar role requires sidecar_config"
```

### 3.6 诊断可观察性验收

#### Test 8: 事件载荷字段验证

```bash
# 运行 event-payload-fields 测试用例
cargo test event_payload_fields --test work_role_defaults_integration

# 预期结果:
# - 所有 TypedSupervisionEvent 包含 work_role 字段
# - 所有 TypedSupervisionEvent 包含 used_fallback_default 字段
# - 所有 TypedSupervisionEvent 包含 effective_policy_source 字段
```

#### Test 9: 兜底默认日志验证

```bash
# 运行 fallback-default-logging 测试用例
cargo test fallback_default_logging --test work_role_defaults_integration

# 预期结果:
# - 角色缺失的子任务触发 WARN 级别日志
# - 日志标注 "falling back to Worker default"
# - used_fallback_default = true
```

## 4. 常见问题

### Q1: 为什么角色缺失时回落到 Worker 而不是其他角色?

**A**: Worker 是最保守的默认, 其行为为"成功退出后停止, 失败后有限重试", 既不会像 Service 那样无限重启, 也不会像 Job 那样成功后立即停止。这符合安全回退原则。

### Q2: 能否为同一子任务混合使用两个角色的默认策略?

**A**: 不能。**`RoleDefaultPolicyPack`** 是原子绑定到单一角色的, 用户只能选择覆盖整个包或部分字段, 但不能从不同角色拼凑默认值。这保证了角色语义的一致性。

### Q3: 如何让 Job 在成功后也重启 (例如周期性批处理)?

**A**: 有两种方式:
1. 显式覆写: 在 **`ChildSpec`** 中设置 **`restart_policy`** 为 **`Restart`** (会触发语义冲突警告)
2. 外部调度: 不在监督器内声明为常驻任务, 而是通过 cron 或外部调度器定期启动新的 Job 实例

推荐方式 2, 因为更符合 Job 的角色语义。

### Q4: Sidecar 的 linked_lifecycle 设为 true 后, 主服务停止时边车会怎样?

**A**: 主服务收到人工停止请求时, 监督器会连带发送停止请求给边车。但边车仍可独立接收人工停止请求而不影响主服务。

### Q5: 如何在生产环境关闭冲突警告?

**A**: 当前版本不支持关闭警告, 因为冲突可能指示配置错误。未来版本可通过 **`strict_role_semantics: false`** 配置开关降级为 DEBUG 级别日志。

## 5. 下一步

完成 **005-2** 验收后, 贡献者可参考以下方向继续:

- **配置热更新**: 支持运行中动态修改子任务角色声明
- **健康检查集成**: 将健康检查结果纳入成功退出语义判定
- **指标导出**: 为每个角色导出独立的监控指标 (重启次数、预算使用率等)
- **Dashboard 可视化**: 在仪表板中展示子任务角色与生效策略来源

## 6. 参考资料

- **功能规格**: `specs/005-2-work-role-defaults/spec.md`
- **实现计划**: `specs/005-2-work-role-defaults/plan.md`
- **研究结论**: `specs/005-2-work-role-defaults/research.md`
- **数据模型**: `specs/005-2-work-role-defaults/data-model.md`
- **接口契约**: `specs/005-2-work-role-defaults/contracts/role-defaults.md`
- **宪章**: `.specify/memory/constitution.md`
- **依赖功能**: `specs/005-1-failure-policy-reliability/spec.md`
