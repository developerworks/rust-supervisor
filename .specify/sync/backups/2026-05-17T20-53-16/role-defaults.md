# Contract(契约): 角色默认行为映射与配置覆盖优先级

本文定义 **005-2 Work Role Defaults**(工作角色默认值) 的对外稳定契约, 包括角色到默认行为的映射规则、配置覆盖优先级、冲突检测策略以及与 **005-1** 失败流水线的集成点。本契约在 Phase 1(设计阶段) 冻结, 后续实现不得偏离。

## 1. 角色到默认行为的映射表

下表为五类 **`WorkRole`(工作任务角色)** 在五种退出场景下的默认监督动作。此为对外公开的行为对照表, 验收测试必须逐项核对。

| WorkRole(工作任务角色)     | Success Exit(成功退出)         | Failure Exit(失败退出)                          | Manual Stop(人工停止) | Timeout(超时)                  | Budget Exhausted(预算耗尽)             |
| -------------------------- | ------------------------------ | ----------------------------------------------- | --------------------- | ------------------------------ | -------------------------------------- |
| **Service**(常驻服务)      | Restart(重启) - 保持在线       | RestartWithBackoff(带退避重启)                  | StopForever(永久停止) | RestartWithBackoff(带退避重启) | StopAndEscalate(停止并升级)            |
| **Worker**(工作任务)       | Stop(停止) - 任务完成          | RestartWithBackoff(带退避重启) - 限次数         | StopForever(永久停止) | RestartWithBackoff(带退避重启) | StopAndEscalate(停止并升级)            |
| **Job**(一次性作业)        | Stop(停止) - 不得再起          | RestartWithBackoff(带退避重启) - 有限重试       | StopForever(永久停止) | StopAndEscalate(停止并升级)    | StopAndEscalate(停止并升级)            |
| **Sidecar**(辅助任务)      | Restart(重启) - 单独重启辅进程 | RestartWithBackoff(带退避重启) - 不连带主进程   | StopForever(永久停止) | RestartWithBackoff(带退避重启) | StopAndEscalate(停止并升级)            |
| **Supervisor**(嵌套监督器) | Restart(重启) - 外层核算预算   | RestartWithBackoff(带退避重启) - 内层树作为单元 | StopForever(永久停止) | RestartWithBackoff(带退避重启) | StopAndEscalate(停止并升级) - 外层核算 |

**Contract Invariants(契约不变量)**:

- **INV-001**: **`Job`** 角色在成功退出后永远不得自动再起 (除非用户显式覆写为 **`Restart`**)
- **INV-002**: 所有角色在人工停止后必须进入 **`StopForever`** 状态, 角色默认不得覆盖显式停止请求
- **INV-003**: 预算耗尽后所有角色必须进入 **`StopAndEscalate`** 或 **`Quarantine`**, 不得继续无限重启
- **INV-004**: **`Sidecar`** 角色失败时默认不得连带停止主服务, 除非 **`linked_lifecycle`** 显式设为 `true`
- **INV-005**: **`Supervisor`** 角色的重启与预算必须由外层监督器统一核算, 不得出现内外层计数不一致

## 2. 配置覆盖优先级规则

### 2.1 三层优先级模型

```
Priority Level 1 (Highest) - 用户显式覆写
    ↓ (覆盖)
Priority Level 2 (Medium)  - 角色默认策略包
    ↓ (回落到)
Priority Level 3 (Lowest)  - 全局保守兜底默认 (Worker 角色)
```

### 2.2 合并语义

**Rule MERGE-001**: 用户在 **`ChildSpec`** 中显式指定的策略字段 (如 **`restart_policy`**, **`backoff_policy`**, **`shutdown_policy`**) 完全覆盖对应维度的角色默认值。

**Rule MERGE-002**: 用户未指定的字段从 **`RoleDefaultPolicy`** 中填充。

**Rule MERGE-003**: 若角色默认包中某字段也为 **`None`**, 则回落到全局默认 (定义在 **`src/config/configurable.rs`** 或等价位置)。

**Rule MERGE-004**: 合并后的结果写入 **`EffectivePolicy`** 结构, 包含 **`source`** 字段标明来源 (**`RoleDefault`**, **`UserOverride`**, **`FallbackDefault`**)。

### 2.3 示例

```yaml
# 示例 1: 无显式覆写, 完全使用角色默认
children:
  - id: my-job
    work_role: job
    command: ["echo", "hello"]
    # on_success_exit → Stop (来自 JOB_DEFAULT)
    # on_failure_exit → RestartWithBackoff (来自 JOB_DEFAULT)

# 示例 2: 部分覆写
children:
  - id: my-service
    work_role: service
    command: ["my-server"]
    restart_policy:
      max_restarts: 5  # 用户覆写重启次数
      window_secs: 60
    # on_success_exit → Restart (来自 SERVICE_DEFAULT, 未覆写)
    # on_failure_exit → RestartWithBackoff (来自 SERVICE_DEFAULT, 未覆写)
    # restart_limit.max_restarts → 5 (用户覆写)

# 示例 3: 角色缺失, 回落到 Worker 默认
children:
  - id: unknown-role-task
    command: ["my-worker"]
    # work_role 缺失 → 内部使用 Worker + 诊断日志标注
    # on_success_exit → Stop (来自 WORKER_DEFAULT)
    # on_failure_exit → RestartWithBackoff (来自 WORKER_DEFAULT)
```

## 3. 冲突检测与警告策略

### 3.1 冲突定义

**Conflict CONF-001**: 用户为 **`Job`** 角色显式指定 **`restart_policy`** 为 **`Permanent`** (永久重启), 与角色语义矛盾。

**Conflict CONF-002**: 用户为 **`Sidecar`** 角色声明 **`sidecar_config`** 但 **`primary_child_id`** 指向的子任务不存在。

**Conflict CONF-003**: 用户为 **`Sidecar`** 角色声明 **`sidecar_config`** 但 **`primary_child_id`** 指向的子任务本身也是 **`Sidecar`** (链式边车)。

**Conflict CONF-004**: 用户显式指定 **`work_role`** 为 **`Sidecar`** 但未提供 **`sidecar_config`**。

### 3.2 处理策略

**当前版本严格度**: **Warning**(警告) - 输出醒目的警告日志并标注冲突点, 但仍允许加载配置以便渐进迁移。

**演进路径**: 当前版本采用警告模式,后续可通过配置开关 `strict_role_semantics: bool` 升级为拒绝加载。计划在 v0.x 版本中逐步过渡到默认拒绝模式,届时冲突配置将导致加载失败并返回错误。

**警告日志格式**:

```
WARN supervisor::policy::role_defaults: Semantic conflict detected for child '{child_id}'
  - Declared role: {work_role}
  - Conflicting override: {field_name} = {user_value}
  - Role default expects: {expected_semantic}
  - Impact: Behavior may contradict role semantics; consider removing the override or changing the role.
  - used_fallback_default: false
  - effective_policy_source: UserOverride
```

**未来版本演进**: 可通过配置开关 **`strict_role_semantics: bool`** 升级为 **Reject**(拒绝加载), 在后续版本逐步过渡。

### 3.3 诊断字段

所有警告必须包含以下字段以便排查:

- **`child_id`**: 冲突子任务的标识
- **`work_role`**: 声明的角色
- **`conflicting_field`**: 冲突的字段名
- **`user_value`**: 用户指定的值
- **`expected_semantic`**: 角色默认期望的语义
- **`used_fallback_default`**: 是否使用了兜底默认
- **`effective_policy_source`**: 生效策略的来源

## 4. Sidecar 主服务绑定契约

### 4.1 绑定语法

```yaml
children:
  - id: primary-service
    work_role: service
    command: ["my-server"]

  - id: logging-sidecar
    work_role: sidecar
    command: ["fluentd"]
    sidecar_config:
      primary_child_id: primary-service # 必须引用存在的子任务 ID
      linked_lifecycle: false # 默认 false, 允许单独重启
```

### 4.2 验证规则

**Rule SIDE-001**: 若 **`work_role`** 为 **`Sidecar`**, **`sidecar_config`** 必须存在且非空。

**Rule SIDE-002**: **`sidecar_config.primary_child_id`** 必须引用同一监督树内存在的子任务标识。

**Rule SIDE-003**: **`sidecar_config.primary_child_id`** 指向的子任务不能是 **`Sidecar`** 角色 (禁止链式边车)。

**Rule SIDE-004**: 若 **`linked_lifecycle`** 为 `true`, 主服务停止时必须连带停止边车; 若为 `false` (默认), 边车可单独重启。

### 4.3 生命周期联动语义

**When `linked_lifecycle: false` (默认)**:

- 主服务失败 → 边车继续运行
- 边车失败 → 单独重启边车, 不影响主服务
- 主服务人工停止 → 边车继续运行 (除非也收到停止请求)

**When `linked_lifecycle: true`**:

- 主服务失败 → 边车继续运行 (除非配置另有说明)
- 边车失败 → 单独重启边车
- 主服务人工停止 → 连带停止边车
- 边车人工停止 → 不影响主服务

## 5. 与 005-1 失败流水线的集成契约

### 5.1 集成点

**Integration Point INT-001**: **`evaluate budget`(评估预算)** 阶段之前

```rust
// Pseudo-code (伪代码)
fn prepare_effective_policy(child_spec: &ChildSpec) -> EffectivePolicy {
    let role = child_spec.work_role.unwrap_or(WorkRole::Worker); // Fallback to Worker
    let role_defaults = RoleDefaultPolicy::for_role(role);
    let user_overrides = extract_user_overrides(child_spec);
    EffectivePolicy::merge(Some(role), Some(user_overrides))
}
```

**Integration Point INT-002**: **`decide action`(决定动作)** 阶段

```rust
// Pseudo-code (伪代码)
fn decide_action(
    effective_policy: &EffectivePolicy,
    exit_status: ExitStatus,
    meltdown_state: &MeltdownState,
) -> Decision {
    match exit_status {
        ExitStatus::Success(code) if effective_policy.policy_pack.success_exit_codes.contains(&code) => {
            // Use on_success_exit from effective policy
            map_to_decision(effective_policy.policy_pack.on_success_exit)
        }
        ExitStatus::Failure(_) => {
            // Use on_failure_exit from effective policy
            // Check restart limit and backoff from effective policy
            map_to_decision_with_backoff(
                effective_policy.policy_pack.on_failure_exit,
                effective_policy.policy_pack.default_restart_limit,
                effective_policy.policy_pack.default_backoff_policy,
            )
        }
        ExitStatus::ManualStop => {
            // Manual stop always takes precedence
            map_to_decision(effective_policy.policy_pack.on_manual_stop)
        }
        ExitStatus::Timeout => {
            // Use on_timeout from effective policy
            map_to_decision(effective_policy.policy_pack.on_timeout)
        }
    }
}
```

**Integration Point INT-003**: **`execute action`(执行动作)** 阶段

```rust
// Pseudo-code (伪代码)
fn execute_action(
    decision: Decision,
    effective_policy: &EffectivePolicy,
) -> ExecutionResult {
    // Execute the decision
    let result = perform_execution(decision);

    // Write structured event with role attribution
    let event = TypedSupervisionEvent {
        work_role: Some(effective_policy.work_role),
        used_fallback_default: effective_policy.used_fallback,
        effective_policy_source: Some(effective_policy.source),
        // ... other fields ...
    };
    emit_event(event);

    result
}
```

### 5.2 不变量

**Invariant INV-INT-001**: **005-2** 不得分叉第二条失败旁路, 所有角色默认必须通过 **005-1** 定义的统一流水线执行。

**Invariant INV-INT-002**: 角色默认不得覆盖用户显式的 **`manual_stop`(人工停止)** 或 **`external_cancel`(外部取消)** 请求。

**Invariant INV-INT-003**: **`evaluate budget`** 阶段使用的 **`restart limit`** 与 **`escalation policy`** 必须来自合并后的 **`EffectivePolicy`**, 不得绕过角色默认直接使用用户原始配置。

## 6. 成功退出语义契约

### 6.1 退出码判定

**Rule SUCCESS-001**: 默认情况下, 退出码 `0` 视为成功退出。

**Rule SUCCESS-002**: 用户可在 **`ChildSpec`** 中通过 **`success_exit_codes`** 字段覆盖默认列表 (例如 `[0, 1]` 表示退出码 0 和 1 都视为成功)。

**Rule SUCCESS-003**: **`RoleDefaultPolicy`** 中的 **`success_exit_codes`** 字段默认为 `[0]`, 用户覆写优先级更高。

### 6.2 健康检查判定 (可选增强)

若子任务声明了 **`HealthPolicy`(健康策略)** 中的就绪探针, 则成功退出还需满足:

- 进程退出前最后一次健康检查通过
- 或健康检查未在配置的时间窗口内报告失败

**Note**: 健康检查判定为可选增强, 不在 **005-2** MVP(最小可行产品) 范围内, 留待后续切片实现。

## 7. 诊断与可观察性契约

### 7.1 事件载荷字段

所有 **`TypedSupervisionEvent`(类型化监督事件)** 必须包含以下字段:

```rust
pub struct TypedSupervisionEvent {
    // ... existing fields ...

    /// Work role of the child that triggered this event.
    pub work_role: Option<WorkRole>,

    /// Whether fallback default was used for this child.
    pub used_fallback_default: bool,

    /// Source of the effective policy.
    pub effective_policy_source: Option<PolicySource>,

    // ... existing fields ...
}
```

### 7.2 日志级别要求

**Rule LOG-001**: 角色解析与默认策略选择必须在 **INFO** 级别日志中可见 (至少在每个子任务启动时输出一条)。

**Rule LOG-002**: 冲突警告必须在 **WARN** 级别日志中输出, 包含完整的冲突上下文。

**Rule LOG-003**: 兜底默认启用必须在 **WARN** 级别日志中输出, 标注哪个子任务触发了回退。

### 7.3 示例日志输出

```
INFO supervisor::policy::role_defaults: Resolved work role for child 'my-job'
  - work_role: Job
  - effective_policy_source: RoleDefault
  - used_fallback_default: false

WARN supervisor::policy::role_defaults: Semantic conflict detected for child 'risky-job'
  - Declared role: Job
  - Conflicting override: restart_policy = Permanent
  - Role default expects: on_success_exit = Stop
  - Impact: Behavior may contradict role semantics; consider removing the override or changing the role.
  - used_fallback_default: false
  - effective_policy_source: UserOverride

WARN supervisor::policy::role_defaults: Work role missing for child 'unknown-task', falling back to Worker default
  - used_fallback_default: true
  - effective_policy_source: FallbackDefault
```

## 8. 向后兼容性契约

### 8.1 现有配置兼容性

**Rule COMPAT-001**: 现有不包含 **`work_role`** 字段的配置文件必须能正常加载, 系统内部回落到 **`Worker`** 角色并输出诊断日志。

**Rule COMPAT-002**: 现有不包含 **`sidecar_config`** 的 **`Sidecar`** 角色声明在配置加载阶段拒绝并报错 (此为破坏性变更, 需在迁移文档中说明)。

### 8.2 API 兼容性

**Rule API-001**: 新增的 **`WorkRole`**, **`RoleDefaultPolicy`**, **`EffectivePolicy`** 等结构不得引入 **compatibility exports**(兼容导出), 所有公共 API 必须通过最小集合暴露。

**Rule API-002**: **`ChildSpec`** 新增的 **`work_role`** 与 **`sidecar_config`** 字段必须标记为 **`#[serde(default)]`** 以确保反序列化兼容性。

## 9. 验收测试契约

### 9.1 行为对照表验收

**Test CONTRACT-001**: 为每个角色准备一份最小的示例拓扑, 在只用默认策略且不额外覆写时, 验证成功退出与失败退出触发的自动动作与第 1 节的映射表一致。

**Test CONTRACT-002**: 验证 **`Job`** 角色在成功退出后自动再起比例为 0%。

**Test CONTRACT-003**: 验证 **`Service`** 角色在成功退出后仍保持可用的自动恢复行为。

**Test CONTRACT-004**: 验证 **`Worker`** 角色在失败后限次数重试并在用尽预算后停下或升级。

**Test CONTRACT-005**: 验证 **`Sidecar`** 角色失败时可以单独重启辅助进程且不连带关掉主进程 (当 **`linked_lifecycle: false`**)。

**Test CONTRACT-006**: 验证 **`Supervisor`** 角色外层把整个内层监督树当一个单元来算重启与预算。

### 9.2 冲突检测验收

**Test CONTRACT-007**: 为 **`Job`** 角色显式指定 **`Permanent`** 重启策略, 验证系统输出警告日志并标注冲突点。

**Test CONTRACT-008**: 为 **`Sidecar`** 角色不提供 **`sidecar_config`**, 验证配置加载阶段拒绝并报错。

**Test CONTRACT-009**: 为 **`Sidecar`** 角色指定不存在的 **`primary_child_id`**, 验证配置加载阶段拒绝并报错。

### 9.3 诊断可观察性验收

**Test CONTRACT-010**: 验证所有 **`TypedSupervisionEvent`** 包含 **`work_role`**, **`used_fallback_default`**, **`effective_policy_source`** 字段。

**Test CONTRACT-011**: 验证角色缺失时系统在 **WARN** 级别日志中标注已启用兜底默认。

**Test CONTRACT-012**: 验证冲突警告在 **WARN** 级别日志中输出且包含完整冲突上下文。
