# Data Model(数据模型): 工作角色与默认策略包

本文定义 **005-2 Work Role Defaults**(工作角色默认值) 功能涉及的核心数据结构、枚举类型与字段义务。所有结构必须实现 **`serde`(序列化)** 的 **`Serialize`** 与 **`Deserialize`** trait(特性), 以及 **`schemars`** 的 **`JsonSchema`** trait 以便生成配置 schema(模式)。

## 1. WorkRole(工作任务角色) 枚举

**Location(位置)**: `src/policy/role_defaults.rs`

```rust
/// Work role classification for supervised children.
///
/// Each role binds to a distinct `RoleDefaultPolicy` that defines
/// default supervision behavior across success, failure, manual stop,
/// timeout, and budget exhaustion scenarios.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkRole {
    /// Long-running service that should stay online.
    Service,
    /// Background worker with bounded retry semantics.
    Worker,
    /// One-shot job that must not auto-restart on success.
    Job,
    /// Auxiliary sidecar process attached to a primary service.
    Sidecar,
    /// Nested supervisor tree treated as a single unit.
    Supervisor,
}
```

**Field Obligations(字段义务)**:

- 必须支持从字符串反序列化 (蛇形命名法: `service`, `worker`, `job`, `sidecar`, `supervisor`)
- 必须实现 **`Display`** trait 以便在日志中输出人类可读的角色名称
- 必须提供 **`as_str(&self) -> &'static str`** 方法返回低基数标签

**Validation Rules(验证规则)**:

- 未知角色字符串在反序列化时必须返回错误, 不得静默回落到某个默认值
- 配置加载阶段若 **`ChildSpec.work_role`** 为 **`None`**, 系统内部使用 **`WorkRole::Worker`** 作为保守兜底, 但必须在诊断日志中标注

## 2. SidecarConfig(边车配置) 结构

**Location(位置)**: `src/policy/role_defaults.rs`

```rust
/// Configuration for sidecar attachment to a primary service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SidecarConfig {
    /// Child ID of the primary service this sidecar attaches to.
    pub primary_child_id: ChildId,
    /// Whether lifecycle events are linked (default: false).
    #[serde(default)]
    pub linked_lifecycle: bool,
}
```

**Field Obligations(字段义务)**:

- **`primary_child_id`**: 必须引用同一监督树内存在的子任务标识; 配置加载阶段验证目标存在性
- **`linked_lifecycle`**: 默认为 `false`, 表示允许边车单独重启而不牵动主服务; 设为 `true` 时主服务停止会连带停止边车

**Validation Rules(验证规则)**:

- 若 **`WorkRole`** 为 **`Sidecar`** 但未提供 **`sidecar_config`**, 配置加载阶段拒绝并报错
- **`primary_child_id`** 指向的子任务本身不能是 **`Sidecar`** 角色 (禁止链式边车)
- **`primary_child_id`** 指向的子任务必须在当前监督树中存在

## 3. OnSuccessAction(成功退出动作) 枚举

**Location(位置)**: `src/policy/role_defaults.rs`

```rust
/// Action taken when a child exits successfully.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OnSuccessAction {
    /// Restart the child to keep it online.
    Restart,
    /// Stop the child permanently.
    Stop,
    /// Take no automatic action (caller decides).
    NoOp,
}
```

**Semantic Mapping(语义映射)**:

- **`Service`** → **`Restart`** (保持在线)
- **`Worker`** → **`Stop`** (任务完成即停止)
- **`Job`** → **`Stop`** (一次性作业成功后不得重启)
- **`Sidecar`** → **`Restart`** (辅助进程应保持可用)
- **`Supervisor`** → **`Restart`** (嵌套监督器应保持运行)

## 4. OnFailureAction(失败退出动作) 枚举

**Location(位置)**: `src/policy/role_defaults.rs`

```rust
/// Action taken when a child exits with failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OnFailureAction {
    /// Restart with backoff policy applied.
    RestartWithBackoff,
    /// Restart indefinitely (permanent restart mode).
    RestartPermanent,
    /// Stop and escalate to parent or shutdown tree.
    StopAndEscalate,
}
```

**Semantic Mapping(语义映射)**:

- **`Service`** → **`RestartWithBackoff`** (带退避重启)
- **`Worker`** → **`RestartWithBackoff`** (限次数重试)
- **`Job`** → **`RestartWithBackoff`** (有限重试后停止)
- **`Sidecar`** → **`RestartWithBackoff`** (单独重启辅助进程)
- **`Supervisor`** → **`RestartWithBackoff`** (外层核算预算后重启内层树)

## 5. OnManualStopAction(人工停止动作) 枚举

**Location(位置)**: `src/policy/role_defaults.rs`

```rust
/// Action taken when a child receives an explicit stop request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OnManualStopAction {
    /// Stop permanently until explicitly restarted.
    StopForever,
    /// Stop but allow future restarts by caller.
    StopUntilExplicitRestart,
}
```

**Semantic Mapping(语义映射)**:

- 所有角色 → **`StopForever`** (人工停止优先于自动恢复)

## 6. OnTimeoutAction(超时动作) 枚举

**Location(位置)**: `src/policy/role_defaults.rs`

```rust
/// Action taken when a child exceeds its execution timeout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OnTimeoutAction {
    /// Restart with backoff policy applied.
    RestartWithBackoff,
    /// Stop and escalate to parent or shutdown tree.
    StopAndEscalate,
}
```

**Semantic Mapping(语义映射)**:

- **`Service`** → **`RestartWithBackoff`**
- **`Worker`** → **`RestartWithBackoff`**
- **`Job`** → **`StopAndEscalate`** (超时视为作业失败)
- **`Sidecar`** → **`RestartWithBackoff`**
- **`Supervisor`** → **`RestartWithBackoff`**

## 7. OnBudgetExhaustedAction(预算耗尽动作) 枚举

**Location(位置)**: `src/policy/role_defaults.rs`

```rust
/// Action taken when restart budget is exhausted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OnBudgetExhaustedAction {
    /// Stop and escalate to parent or shutdown tree.
    StopAndEscalate,
    /// Quarantine the child or scope without escalating.
    Quarantine,
}
```

**Semantic Mapping(语义映射)**:

- **`Service`** → **`StopAndEscalate`**
- **`Worker`** → **`StopAndEscalate`**
- **`Job`** → **`StopAndEscalate`**
- **`Sidecar`** → **`StopAndEscalate`**
- **`Supervisor`** → **`StopAndEscalate`** (外层核算后升级)

## 8. RoleDefaultPolicy(角色默认策略包) 结构

**Location(位置)**: `src/policy/role_defaults.rs`

```rust
/// Default policy bundle bound to a specific work role.
///
/// This structure defines the baseline supervision behavior for each
/// role. User overrides in `ChildSpec` take precedence over these defaults.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RoleDefaultPolicy {
    /// Action on successful exit.
    pub on_success_exit: OnSuccessAction,
    /// Action on failure exit.
    pub on_failure_exit: OnFailureAction,
    /// Action on explicit manual stop.
    pub on_manual_stop: OnManualStopAction,
    /// Action on execution timeout.
    pub on_timeout: OnTimeoutAction,
    /// Action when restart budget is exhausted.
    pub on_budget_exhausted: OnBudgetExhaustedAction,
    /// Default restart limit (None means use global default).
    pub default_restart_limit: Option<RestartLimit>,
    /// Default escalation policy (None means use global default).
    pub default_escalation_policy: Option<EscalationPolicy>,
    /// Default backoff policy (None means use global default).
    pub default_backoff_policy: Option<BackoffPolicy>,
    /// Exit codes considered successful (default: [0]).
    #[serde(default = "default_success_exit_codes")]
    pub success_exit_codes: Vec<i32>,
}
```

**Field Obligations(字段义务)**:

- **`on_success_exit`**: 决定成功退出后是否自动重启; **`Job`** 角色必须为 **`Stop`**
- **`on_failure_exit`**: 决定失败后的重启策略; 所有角色默认不得使用 **`RestartPermanent`** 除非显式覆写
- **`on_manual_stop`**: 人工停止必须优先于任何自动恢复逻辑
- **`on_timeout`**: 超时视为一种特殊失败, 动作可与普通失败不同
- **`on_budget_exhausted`**: 预算耗尽后的最终处置, 通常为升级或隔离
- **`default_restart_limit`**: 角色特定的重启次数限制; **`None`** 表示回落到全局默认
- **`default_escalation_policy`**: 角色特定的升级策略; **`None`** 表示回落到全局默认
- **`default_backoff_policy`**: 角色特定的退避策略; **`None`** 表示回落到全局默认
- **`success_exit_codes`**: 定义哪些退出码视为成功; 默认 `[0]`, 用户可覆盖以支持多退出码语义

**Helper Function(辅助函数)**:

```rust
fn default_success_exit_codes() -> Vec<i32> {
    vec![0]
}
```

## 9. 五类角色的默认策略包常量

**Location(位置)**: `src/policy/role_defaults.rs`

```rust
impl RoleDefaultPolicy {
    /// Default policy pack for Service role.
    pub const SERVICE_DEFAULT: Self = Self {
        on_success_exit: OnSuccessAction::Restart,
        on_failure_exit: OnFailureAction::RestartWithBackoff,
        on_manual_stop: OnManualStopAction::StopForever,
        on_timeout: OnTimeoutAction::RestartWithBackoff,
        on_budget_exhausted: OnBudgetExhaustedAction::StopAndEscalate,
        default_restart_limit: None, // Use global default
        default_escalation_policy: None, // Use global default
        default_backoff_policy: None, // Use global default
        success_exit_codes: vec![0],
    };

    /// Default policy pack for Worker role.
    pub const WORKER_DEFAULT: Self = Self {
        on_success_exit: OnSuccessAction::Stop,
        on_failure_exit: OnFailureAction::RestartWithBackoff,
        on_manual_stop: OnManualStopAction::StopForever,
        on_timeout: OnTimeoutAction::RestartWithBackoff,
        on_budget_exhausted: OnBudgetExhaustedAction::StopAndEscalate,
        default_restart_limit: None, // Use global default
        default_escalation_policy: None, // Use global default
        default_backoff_policy: None, // Use global default
        success_exit_codes: vec![0],
    };

    /// Default policy pack for Job role.
    pub const JOB_DEFAULT: Self = Self {
        on_success_exit: OnSuccessAction::Stop,
        on_failure_exit: OnFailureAction::RestartWithBackoff,
        on_manual_stop: OnManualStopAction::StopForever,
        on_timeout: OnTimeoutAction::StopAndEscalate,
        on_budget_exhausted: OnBudgetExhaustedAction::StopAndEscalate,
        default_restart_limit: None, // Use global default
        default_escalation_policy: None, // Use global default
        default_backoff_policy: None, // Use global default
        success_exit_codes: vec![0],
    };

    /// Default policy pack for Sidecar role.
    pub const SIDECAR_DEFAULT: Self = Self {
        on_success_exit: OnSuccessAction::Restart,
        on_failure_exit: OnFailureAction::RestartWithBackoff,
        on_manual_stop: OnManualStopAction::StopForever,
        on_timeout: OnTimeoutAction::RestartWithBackoff,
        on_budget_exhausted: OnBudgetExhaustedAction::StopAndEscalate,
        default_restart_limit: None, // Use global default
        default_escalation_policy: None, // Use global default
        default_backoff_policy: None, // Use global default
        success_exit_codes: vec![0],
    };

    /// Default policy pack for Supervisor role.
    pub const SUPERVISOR_DEFAULT: Self = Self {
        on_success_exit: OnSuccessAction::Restart,
        on_failure_exit: OnFailureAction::RestartWithBackoff,
        on_manual_stop: OnManualStopAction::StopForever,
        on_timeout: OnTimeoutAction::RestartWithBackoff,
        on_budget_exhausted: OnBudgetExhaustedAction::StopAndEscalate,
        default_restart_limit: None, // Use global default
        default_escalation_policy: None, // Use global default
        default_backoff_policy: None, // Use global default
        success_exit_codes: vec![0],
    };
}
```

**Lookup Function(查找函数)**:

```rust
impl RoleDefaultPolicy {
    /// Returns the default policy pack for the given work role.
    pub fn for_role(role: WorkRole) -> Self {
        match role {
            WorkRole::Service => Self::SERVICE_DEFAULT,
            WorkRole::Worker => Self::WORKER_DEFAULT,
            WorkRole::Job => Self::JOB_DEFAULT,
            WorkRole::Sidecar => Self::SIDECAR_DEFAULT,
            WorkRole::Supervisor => Self::SUPERVISOR_DEFAULT,
        }
    }
}
```

## 10. PolicySource(策略来源) 枚举

**Location(位置)**: `src/policy/role_defaults.rs`

```rust
/// Source of the effective policy after merging role defaults and user overrides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PolicySource {
    /// Effective policy comes entirely from role defaults.
    RoleDefault,
    /// Effective policy includes user overrides.
    UserOverride,
    /// Effective policy uses fallback default due to missing or unknown role.
    FallbackDefault,
}
```

**Usage(用法)**: 写入 **`TypedSupervisionEvent`** 载荷, 用于诊断与审计

## 11. EffectivePolicy(生效策略) 结构

**Location(位置)**: `src/policy/role_defaults.rs`

```rust
/// Merged policy after combining role defaults with user overrides.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EffectivePolicy {
    /// The work role that contributed the base defaults.
    pub work_role: WorkRole,
    /// The merged policy pack.
    pub policy_pack: RoleDefaultPolicy,
    /// Source of the effective policy.
    pub source: PolicySource,
    /// Whether fallback default was used.
    pub used_fallback: bool,
    /// List of fields overridden by user (empty if source is RoleDefault).
    pub overridden_fields: Vec<String>,
}
```

**Field Obligations(字段义务)**:

- **`work_role`**: 原始声明的角色 (若缺失则为 **`WorkRole::Worker`**)
- **`policy_pack`**: 合并后的最终策略包
- **`source`**: 策略来源枚举
- **`used_fallback`**: 是否使用了兜底默认 (角色缺失或未知时为 `true`)
- **`overridden_fields`**: 用户显式覆盖的字段名列表, 用于诊断冲突

**Merge Function(合并函数)**:

```rust
impl EffectivePolicy {
    /// Merges role defaults with user overrides from ChildSpec.
    ///
    /// # Arguments
    ///
    /// - `role`: Declared work role (or None for fallback).
    /// - `user_overrides`: Optional user-specified policy fields from ChildSpec.
    ///
    /// # Returns
    ///
    /// Returns an `EffectivePolicy` with merged fields and source attribution.
    pub fn merge(role: Option<WorkRole>, user_overrides: Option<UserPolicyOverrides>) -> Self {
        // Implementation details deferred to Phase 2
        todo!()
    }
}
```

## 12. ChildSpec 扩展字段

**Location(位置)**: `src/spec/child.rs`

现有 **`ChildSpec`** 结构需新增以下可选字段:

```rust
/// Child specification for supervised units.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ChildSpec {
    // ... existing fields ...

    /// Optional work role classification.
    ///
    /// If None, the system falls back to Worker role with diagnostic logging.
    #[serde(default)]
    pub work_role: Option<WorkRole>,

    /// Optional sidecar configuration (required if work_role is Sidecar).
    #[serde(default)]
    pub sidecar_config: Option<SidecarConfig>,

    // ... existing fields ...
}
```

**Validation Rules(验证规则)**:

- 若 **`work_role`** 为 **`Some(WorkRole::Sidecar)`** 且 **`sidecar_config`** 为 **`None`**, 配置加载阶段拒绝并报错
- 若 **`sidecar_config`** 存在但 **`work_role`** 不是 **`Sidecar`**, 发出警告 (配置不一致)

## 13. TypedSupervisionEvent 扩展字段

**Location(位置)**: `src/event/payload.rs`

现有 **`TypedSupervisionEvent`** 结构需新增以下可选字段:

```rust
/// Typed supervision event payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TypedSupervisionEvent {
    // ... existing fields ...

    /// Work role of the child that triggered this event.
    #[serde(default)]
    pub work_role: Option<WorkRole>,

    /// Whether fallback default was used for this child.
    #[serde(default)]
    pub used_fallback_default: bool,

    /// Source of the effective policy.
    #[serde(default)]
    pub effective_policy_source: Option<PolicySource>,

    // ... existing fields ...
}
```

**Usage(用法)**: 在 **`evaluate budget`** 与 **`decide action`** 阶段填充, 供事件管道转发至日志、指标与 dashboard(仪表板)

## 14. 数据流图

```
Configuration Loading (配置加载)
    ↓
ChildSpec.work_role 解析
    ↓
┌─────────────────────────────┐
│ Role Missing or Unknown?    │
│  Yes → Use Worker + Log     │
│  No  → Use Declared Role    │
└─────────────────────────────┘
    ↓
Lookup RoleDefaultPolicy (查找角色默认策略包)
    ↓
Merge with User Overrides (与用户覆写合并)
    ↓
┌─────────────────────────────┐
│ Conflict Detection          │
│  Warn if Semantics Mismatch │
└─────────────────────────────┘
    ↓
EffectivePolicy 生成
    ↓
Runtime Control Loop (运行时控制循环)
    ↓
┌─────────────────────────────┐
│ evaluate budget             │
│ decide action               │
│ execute action              │
└─────────────────────────────┘
    ↓
TypedSupervisionEvent 写入 (含 work_role, used_fallback, policy_source)
    ↓
Observability Pipeline (可观察性管道)
    ↓
Logs, Metrics, Dashboard (日志、指标、仪表板)
```

## 15. 与 005-1 数据模型的边界

- **005-2** 不修改 **`MeltdownTracker`**, **`RestartLimit`**, **`EscalationPolicy`**, **`BackoffPolicy`** 等现有结构
- **005-2** 仅提供 **`RoleDefaultPolicy`** 作为 **`005-1`** 流水线中 **`decide action`** 阶段的输入增强
- **005-2** 新增的 **`EffectivePolicy`** 结构在 **`evaluate budget`** 之前计算, 结果传递给 **`005-1`** 的决策引擎
