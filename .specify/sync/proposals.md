# Drift Resolution Proposals(漂移修复提案)

Generated(生成时间): 2026-05-17T20:53:16+08:00
Based on(基于): drift-report from 2026-05-17T20:49:14+08:00
Approval Mode(批准模式): auto(自动批准)

## Summary(摘要)

| Resolution Type(修复类型)        | Count(数量) |
| -------------------------------- | ----------: |
| Backfill(Code -> Spec)(代码回填规格) |           3 |
| Align(Spec -> Code)(规格对齐代码)    |           0 |
| Human Decision(人工决策)             |           0 |
| New Specs(新规格)                   |           0 |
| Remove from Spec(从规格移除)          |           0 |

## Proposals(提案)

### Proposal P1: 005-2-work-role-defaults/SC-002

**Direction(方向)**: BACKFILL(Code -> Spec)(代码回填规格)

**Current State(当前状态)**:
- Spec says(规格说明): `data-model.md(数据模型文档)` 和 `contracts/role-defaults.md(角色默认契约文档)` 仍描述公开常量和旧的 `UserPolicyOverrides(用户策略覆写)` 合并签名.
- Code does(代码行为): 当前实现使用私有默认构造函数, `RoleDefaultPolicy::for_role()` 和 `EffectivePolicy::merge(role, Vec<String>)`.

**Proposed Resolution(建议修复)**:

将 `data-model.md` 与 `contracts/role-defaults.md` 改为当前实现口径:
- 删除 `SERVICE_DEFAULT`, `WORKER_DEFAULT`, `JOB_DEFAULT`, `SIDECAR_DEFAULT`, `SUPERVISOR_DEFAULT` 公开常量描述.
- 改写为 `RoleDefaultPolicy::for_role()` 调用私有默认构造函数.
- 将旧 `UserPolicyOverrides(用户策略覆写)` 签名改为 `Vec<String>` 覆写字段标记.
- 将数据流中 "Role Missing or Unknown" 改为 "Role Missing" 触发 `Worker(工作任务)` 兜底, "Unknown Role" 触发配置错误.

**Rationale(理由)**: 当前代码和测试已经证明实际 API(应用程序接口)可用. 公开常量无法表达包含 `Vec` 的完整默认值, 保留旧文档会误导使用者.

**Confidence(置信度)**: HIGH(高)

**Action(动作)**:
- [X] Approve(批准)
- [ ] Reject(拒绝)
- [ ] Modify(修改)

---

### Proposal P2: 005-2-work-role-defaults/EC-001

**Direction(方向)**: BACKFILL(Code -> Spec)(代码回填规格)

**Current State(当前状态)**:
- Spec says(规格说明): 角色缺失或未知时均回落到保守默认.
- Code does(代码行为): 缺失 `work_role(工作任务角色)` 回落到 `Worker(工作任务)` 并输出 `WARN(警告)` 日志. 未知 `work_role` 由 `serde(序列化)` 枚举反序列化拒绝, 不进入兜底路径.

**Proposed Resolution(建议修复)**:

将规格的边界情况改为:
- 缺失 `work_role` 时, 系统回落到 `Worker(工作任务)` 默认并输出诊断.
- 未知 `work_role` 字符串属于配置错误, 必须在加载阶段拒绝并返回可读错误.

**Rationale(理由)**: 未知角色通常来自拼写错误或未登记的新角色. 静默回落会掩盖配置意图, 严格拒绝比自动兜底更安全.

**Confidence(置信度)**: HIGH(高)

**Action(动作)**:
- [X] Approve(批准)
- [ ] Reject(拒绝)
- [ ] Modify(修改)

---

### Proposal P3: 005-2-work-role-defaults/CONTRACT-SUCCESS-002

**Direction(方向)**: BACKFILL(Code -> Spec)(代码回填规格)

**Current State(当前状态)**:
- Spec says(规格说明): 用户可在 `ChildSpec(子任务规格)` 中通过 `success_exit_codes(成功退出码集合)` 字段覆盖默认成功退出码列表.
- Code does(代码行为): `RoleDefaultPolicy(角色默认策略包)` 包含内部 `success_exit_codes`, 但 `ChildSpec` 没有外部覆写字段, `TaskExit(任务退出)` 也不携带原始退出码.

**Proposed Resolution(建议修复)**:

将契约改为:
- 当前版本的成功退出由任务运行时返回的 `TaskResult::Succeeded(任务成功结果)` 或等价成功事实决定.
- `RoleDefaultPolicy.success_exit_codes` 是角色默认策略内部字段, 当前不作为 `ChildSpec` 的用户覆写入口.
- 原始进程退出码覆写作为后续切片处理, 本切片不承诺 `ChildSpec.success_exit_codes`.

**Rationale(理由)**: 当前运行时抽象没有传递原始进程退出码. 在没有退出码事实来源时暴露用户覆写字段会形成假能力.

**Confidence(置信度)**: HIGH(高)

**Action(动作)**:
- [X] Approve(批准)
- [ ] Reject(拒绝)
- [ ] Modify(修改)
