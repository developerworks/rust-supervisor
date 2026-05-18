# ADR-008: 使用 Typed Error 和明确 Policy Decision

- **日期**: 2026-05-05
- **状态**: Accepted

## 背景

策略决定依赖失败类别: recoverable, configuration-fatal, bug-fatal, external, timeout, panic, cancellation. 字符串或泛型错误会迫使策略引擎猜测.

## 可选方案

- 方案 A: 到处使用 `anyhow::Error`. 方便但对重启治理来说不透明.
- 方案 B: 依赖 panic 表达任务失败. 不可接受, 预期失败需要结构化结果.
- 方案 C: 使用 typed error + 明确 policy decision enum.

## 决策

选择方案 C.

## 理由

- `TaskFailureKind` 枚举精确区分失败类别.
- `RestartDecision` 枚举明确输出: `DoNotRestart`, `RestartAfter(delay)`, `Quarantine`, `EscalateToParent`, `ShutdownTree`.
- Policy engine 无需猜测意图.

## 后果

- 正面: 策略引擎直接消费 typed decision, 无歧义.
- 正面: 测试可精确断言指定失败类别的处理结果.
- 负面: 需要维护 `TaskFailureKind` 和 `RestartDecision` 枚举.

## 关联

- 关联 Spec: `specs/001-create-supervisor-core/contracts/public-api.md`
