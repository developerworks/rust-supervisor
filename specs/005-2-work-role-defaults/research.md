# Research(研究结论): 005-2 工作角色默认值

本文承接 `plan.md` 中 Technical Context(技术背景), 冻结 Phase 0(研究阶段) 取舍, 不留 NEEDS CLARIFICATION(需要澄清).

## 1. 工作角色声明语法与存储位置

- **Decision(决定)**: 在 `src/spec/child.rs` 的 **`ChildSpec`(子任务规格)** 结构中新增可选字段 **`work_role: Option<WorkRole>`**; **`WorkRole`(工作任务角色)** 定义为带 **`serde`(序列化)** 与 **`JsonSchema`** 派生的枚举类型, 包含五个变体: **`Service`(常驻服务)**, **`Worker`(工作任务)**, **`Job`(一次性作业)**, **`Sidecar`(辅助任务)**, **`Supervisor`(嵌套监督器)**; 配置加载时若该字段缺失或为 **`None`**, 系统回落到保守兜底默认 **`Worker`(工作任务)** 并在诊断日志中标注已启用安全回退.
- **Rationale(理由)**: 规格 **FR-001** 要求五类角色有明确的默认行为; **Edge Cases**(边界情况) 要求角色缺失时回落到保守默认且必须输出诊断信息; 将角色声明放在 **`ChildSpec`** 符合模块所有权原则, 避免在运行时分散硬编码.
- **Alternatives considered(曾考虑的备选)**: 把角色声明放在配置文件的顶层而非子任务规格; 被拒绝, 因为每个受监督单元的角色可能不同, 必须在子任务粒度声明.

## 2. RoleDefaultPolicy(角色默认策略包) 字段设计

- **Decision(决定)**: **`RoleDefaultPolicy`(角色默认策略包)** 包含以下字段:
  - **`on_success_exit`: OnSuccessAction** - 成功退出时的动作 (**`Restart`(重启)**, **`Stop`(停止)**, \*\*`NoOp`(无操作))
  - **`on_failure_exit`: OnFailureAction** - 失败退出时的动作 (**`RestartWithBackoff`(带退避重启)**, **`RestartPermanent`(永久重启)**, \*\*`StopAndEscalate`(停止并升级))
  - **`on_manual_stop`: OnManualStopAction** - 人工停止时的动作 (**`StopForever`(永久停止)**, \*\*`StopUntilExplicitRestart`(停止直到显式重启))
  - **`on_timeout`: OnTimeoutAction** - 超时时的动作 (**`RestartWithBackoff`(带退避重启)**, \*\*`StopAndEscalate`(停止并升级))
  - **`on_budget_exhausted`: OnBudgetExhaustedAction** - 预算耗尽时的动作 (**`StopAndEscalate`(停止并升级)**, \*\*`Quarantine`(隔离))
  - **`default_restart_limit: Option<RestartLimit>`** - 默认重启次数限制
  - **`default_escalation_policy: Option<EscalationPolicy>`** - 默认升级策略
  - **`default_backoff_policy: Option<BackoffPolicy>`** - 默认退避策略
  - **`success_exit_codes: Vec<i32>`** - 视为成功退出的退出码列表 (默认 `[0]`)

  五个角色默认值由 `src/policy/role_defaults.rs` 中的 `RoleDefaultPolicy::for_role()` 返回, 内部使用私有默认构造函数, 不公开按角色命名的默认常量:
  - **`Service`(常驻服务)**: 成功退出后允许重启以保持在线, 失败后带退避重启, 预算耗尽后升级
  - **`Worker`(工作任务)**: 成功退出后停止, 失败后限次数重试并拉长间隔, 预算耗尽后停止并升级
  - **`Job`(一次性作业)**: 成功退出后停止 (不得自动重启), 失败后有限重试, 预算耗尽后停止并升级
  - **`Sidecar`(辅助任务)**: 成功退出后允许单独重启辅助进程, 不连带关掉主进程除非配置显式绑定生命周期
  - **`Supervisor`(嵌套监督器)**: 外层把内层监督树作为单一单元核算重启与预算

- **Rationale(理由)**: 规格 **FR-001** 明确要求至少覆盖成功退出、失败退出、人为停止、超时与预算耗尽五类情形; **Key Entities**(关键实体) 定义了 **`RoleDefaultPolicy`** 为绑定到某一 **`WorkRole`** 的一套成功、失败与保护参数组合; 字段设计参考了 **005-1** 的 **`restart_execution_plan`** 与 **`MeltdownTracker`** 结构.
- **Alternatives considered(曾考虑的备选)**: 用配置驱动所有默认值而非代码内固定构造函数; 被拒绝, 因为默认值必须是确定且可审计的, 配置只能用于覆盖而非定义默认.

## 3. 配置覆盖优先级与合并规则

- **Decision(决定)**: 采用三层优先级模型:
  1. **最高优先级**: 用户在 **`ChildSpec`** 中显式指定的策略字段 (如 **`restart_policy`**, **`backoff_policy`**)
  2. **中等优先级**: **`RoleDefaultPolicy`** 中与角色匹配的默认值
  3. **最低优先级**: 全局保守兜底默认 (当角色缺失时使用 **`Worker`** 角色默认; 当角色字段存在但值未知时, 配置加载必须失败并给出可读错误)

  合并规则: 用户显式指定的字段完全覆盖对应维度的默认值; 未指定的字段从角色默认包中填充; 若角色默认包中某字段也为 **`None`**, 则回落到全局默认. 当用户显式覆写与角色语义明显不一致时 (例如为 **`Job`** 角色指定 **`Permanent`** 重启策略), 系统在配置加载阶段输出警告日志并标注冲突点, 但仍允许加载 (严格度选择: 警告而非拒绝, 以便渐进迁移).

- **Rationale(理由)**: 规格 **Edge Cases** 要求"当用户显式覆写默认且覆写与角色语义不一致时, 系统必须输出醒目的警告或拒绝加载"; 选择警告而非拒绝是为了向后兼容已有配置; 三层优先级模型符合最小惊讶原则.
- **Alternatives considered(曾考虑的备选)**: 严格模式 (拒绝加载冲突配置); 被拒绝, 因为会破坏现有部署的兼容性, 需在后续版本通过配置开关逐步过渡.

## 4. SuccessExitSemantics(成功退出语义) 定义位置

- **Decision(决定)**: 当前切片的 **`SuccessExitSemantics`(成功退出语义)** 由 **`TaskResult::Succeeded`(任务成功结果)** 或等价的 typed success fact(类型化成功事实) 进入 **`ExitClassification::Success`(退出成功分类)** 表达. **`RoleDefaultPolicy.success_exit_codes`(角色默认策略包成功退出码集合)** 保留为内部策略数据, 默认值为 `[0]`, 但当前不作为 **`ChildSpec`(子任务规格)** 的用户覆写入口. 原始进程退出码覆写必须等后续切片把退出码接入 **`TaskExit`(任务退出事实)** 或等价运行时退出模型后再实现. **`HealthPolicy`(健康策略)** 参与成功退出语义属于可选增强, 当前不在 **005-2** 闭环内.

- **Rationale(理由)**: 规格 **Key Entities** 定义 **`SuccessExitSemantics`** 为"写明哪种退出算业务上的成功, 用来阻止对 **`job`(一次性作业)** 的多余重启"; 当前运行时已经用类型化成功结果区分成功与失败, 但没有公开的 **`ChildSpec.success_exit_codes`(子任务规格成功退出码集合)** 字段. 因此, 当前文档必须把用户覆写承诺降为后续切片, 避免对外契约超过实现.
- **Alternatives considered(曾考虑的备选)**: 立即新增 **`ChildSpec.success_exit_codes`(子任务规格成功退出码集合)** 并接入退出分类; 被拒绝, 因为当前切片目标是角色默认策略, 原始进程退出码建模会扩大运行时事实边界.

## 5. Sidecar 主服务绑定语法

- **Decision(决定)**: 在 **`ChildSpec`** 中新增可选字段 **`sidecar_config: Option<SidecarConfig>`**, 其中 **`SidecarConfig`** 包含:
  - **`primary_child_id: ChildId`** - 所附属的主服务子任务标识
  - **`linked_lifecycle: bool`** - 是否绑定生命周期 (默认 `false`, 即允许辅助进程单独重启而不牵动主进程)

  若 **`WorkRole`** 为 **`Sidecar`** 但未声明 **`sidecar_config`**, 或 **`primary_child_id`** 指向的子任务不存在, 配置加载阶段拒绝并报错; 若 **`primary_child_id`** 指向的子任务本身也是 **`Sidecar`**, 同样拒绝 (禁止链式边车).

- **Rationale(理由)**: 规格 **Assumptions**(假设) 明确写道"若配置描述里有多颗候选主服务节点, **`sidecar`(辅助任务)** 必须在配置中显式声明所附属的 **`primary`(主实例)** 标识, 否则验收场景中的'主服务仍健康'视为未定义并应在加载阶段拒绝"; **FR-001** 验收场景 4 要求 sidecar 失败时默认不连带关掉主进程除非配置显式绑定生命周期.
- **Alternatives considered(曾考虑的备选)**: 通过命名约定隐式推断主服务 (例如 sidecar 名称以主服务名称为前缀); 被拒绝, 因为隐式推断不够明确且容易出错.

## 6. 与 005-1 失败流水线的集成点

- **Decision(决定)**: **005-2** 在以下三个点与 **005-1** 的失败流水线集成:
  1. **`evaluate budget`(评估预算)** 阶段之前: 读取子任务的 **`WorkRole`**, 查找对应的 **`RoleDefaultPolicy`**, 与用户显式配置合并得到最终生效策略
  2. **`decide action`(决定动作)** 阶段: 使用合并后的策略决定重启、停止或升级动作; 角色默认不得覆盖用户显式的 **`manual_stop`(人工停止)** 或 **`external_cancel`(外部取消)** 请求
  3. **`execute action`(执行动作)** 阶段: 执行动作时写入结构化事件载荷, 载明 **`WorkRole`** 与是否启用了兜底默认

  **005-2** 不得分叉第二条失败旁路, 所有角色默认必须通过 **005-1** 定义的统一流水线执行.

- **Rationale(理由)**: 规格 **Dependency Note**(依赖说明) 明确要求"本切片可在 **`005-1`** 未完成前先起草; 合并验收时, 角色默认必须能够改写流水线里 **`decide action`(决定动作)** 与 **`execute action`(执行动作)** 的最终结论"; **005-1** research.md 第 5 节也写明"**`005-2`** 只替换 **`RoleDefaultPolicy`(角色默认策略包)** 写入 **`decide action`** 的输入, 不得分叉第二条失败旁路".
- **Alternatives considered(曾考虑的备选)**: 为每个角色实现独立的决策路径; 被拒绝, 因为会破坏模块化且增加维护成本.

## 7. 诊断与可观察性

- **Decision(决定)**: 在 **`TypedSupervisionEvent`(类型化监督事件)** 中新增以下字段:
  - **`work_role: Option<WorkRole>`** - 子任务的工作角色
  - **`used_fallback_default: bool`** - 是否使用了兜底默认 (仅角色缺失时)
  - **`effective_policy_source: PolicySource`** - 生效策略的来源 (**`RoleDefault`(角色默认)**, **`UserOverride`(用户覆写)**, \*\*`FallbackDefault`(兜底默认))

  类型化事件必须包含角色信息与策略来源, 以便排查配置是否与意图一致; 缺失角色兜底和语义冲突必须通过 **`WARN`(警告级别)** 日志给出可读原因, 结构性配置错误必须在加载阶段拒绝.

- **Rationale(理由)**: 规格 **Constitution Alignment** 的 **Diagnostics**(诊断) 要求"事件与错误信息须载明可唯一识别的 **`WorkRole`(工作任务角色)** 以及是否启用了兜底默认"; **SC-003** 要求"对于显式冲突覆写, 100% 案例必须在拒绝加载或警告正文里写明可读原因".
- **Alternatives considered(曾考虑的备选)**: 只在调试级别日志中输出角色信息; 被拒绝, 因为角色信息是诊断配置问题的关键, 必须在 info 级别可见.
