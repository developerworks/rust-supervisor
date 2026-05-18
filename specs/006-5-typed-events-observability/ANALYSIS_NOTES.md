# 分析笔记: Phase 1 准备阶段

## T001: What 枚举现有变体

阅读 `src/event/payload.rs`. `What` 枚举共有约 35 个变体, 覆盖以下类别:

- **子任务生命周期**: `ChildStarting`, `ChildRunning`, `ChildReady`, `ChildHeartbeat`, `ChildFailed`, `ChildPanicked`, `ChildStopping`, `ChildStopped`, `ChildUnhealthy`, `ChildQuarantined`
- **重启**: `BackoffScheduled`, `ChildRestarting`, `ChildRestarted`, `ChildRestartFenceEntered`, `ChildRestartFenceAbortRequested`, `ChildRestartFenceReleased`, `ChildRestartConflict`, `ChildAttemptStaleReport`, `ChildRestartFencePendingDrained`
- **关闭**: `ShutdownRequested`, `ShutdownPhaseChanged`, `ShutdownCompleted`, `ChildShutdownCancelDelivered`, `ChildShutdownGraceful`, `ChildShutdownAborted`, `ChildShutdownLateReport`
- **控制**: `CommandAccepted`, `CommandCompleted`, `ChildControlCancelDelivered`, `ChildControlStopCompleted`, `ChildControlStopFailed`, `ChildControlOperationChanged`, `ChildControlCommandCompleted`
- **运行时**: `RuntimeControlLoopStarted`, `RuntimeControlLoopShutdownRequested`, `RuntimeControlLoopCompleted`, `RuntimeControlLoopFailed`, `RuntimeControlLoopJoinCompleted`
- **策略**: `Meltdown`, `BudgetExhausted`, `GroupFuseTriggered`, `SubscriberLagged`
- **健康**: `ChildHeartbeatStale`, `ChildRuntimeRestartLimitUpdated`, `ChildRuntimeStateRemoved`

**data-model.md 中缺失的迁移弧(尚未在 What 中):**

- `HealthCheckPassed`, `HealthCheckFailed`, `Paused`, `Resumed`, `Quarantined` 作为类型化变体
- `BudgetDenied`, `GenerationFenced`
- `BackpressureAlert`, `BackpressureDegradation`, `AuditRecorded`

**SupervisorEvent 结构** 已有字段: `config_version`, `correlation_id`, `sequence`, `when`, `where`, `what`, `policy`, `scopes_triggered`, `lead_scope`, `effective_protective_action`, `cold_start_reason`, `hot_loop_reason`. 缺少 `schema_id`.

## T002: 005-1 契约分析

阅读 `specs/005-1-failure-policy-reliability/contracts/pipeline-and-events.md`.

关键发现:

- **6 阶段管线**: `classify exit`(分类退出) → `record failure window`(记录失败窗口) → `evaluate budget`(评估预算) → `decide action`(决定动作) → `emit typed event`(发出类型化事件) → `execute action`(执行动作)
- **退出类别**: `success`(成功), `nonzero_exit`(非零退出), `panic`(崩溃), `timeout`(超时), `external_cancel`(外部取消), `manual_stop`(人工停止)
- **保护档位序**: `restart_allowed`(允许) → `restart_queued`(排队) → `restart_denied`(拒绝) → `supervision_paused`(暂停) → `escalated`(升级) → `supervised_stop`(停止)
- **评估预算输入**: `restart_limit`, `escalation_policy`, `MeltdownTracker` 计数(child/group/supervisor)
- **评估预算输出**: `effective_protective_action`, `scopes_triggered`, `lead_scope`
- **退避策略**: `full jitter`(全抖动), `decorrelated jitter`(去相关抖动), 分散度 CV ≥ 1.3
- **冷启动**: 窗口 60 秒, 最多 5 次重启
- **热循环**: 窗口 10 秒, 最少 3 次重启
- **节流闸门归属**: `"supervisor_global"` 或 `"group:{group_id}"`
- **别名映射**: `restart_execution_plan` → `StrategyExecutionPlan`, `MeltdownTracker` → `MeltdownTracker`

**本切片需补充的变体**: 需要在契约的别名映射表和本切片的 `What` 枚举中同时添加 `BudgetDenied`, `GenerationFenced`, `HealthCheckPassed`, `HealthCheckFailed`, `Paused`, `Resumed`, `Quarantined`, `BackpressureAlert`, `BackpressureDegradation`, `AuditRecorded`.

## T003: ObservabilityPipeline 分析

阅读 `src/observe/pipeline.rs`.

关键发现:

- `ObservabilityPipeline` 持有: `EventJournal`(事件日志), `MetricsFacade`(指标门面), `TestRecorder`(测试记录器), audit channel(审计通道)
- **扇出机制**: 每个事件被推送到 journal, 发射为 metrics, 记录到 tracing, 写入 audit
- `TestRecorder` 捕获: `events`, `pipeline_stage_diagnostics`, `logs`, `spans`, `tracing_events`, `metrics`, `audits`, `subscriber_lag`
- **背压检测**: 通过 subscriber lag 检测(`SubscriberLagged` 事件)
- **缓冲区监控**: 尚无显式 per-subscriber 缓冲区占用率监控(本切片将添加)
- **管线诊断**: `PipelineStageDiagnostic` 已有 `evaluated: bool` 和 `skip_reason: Option<String>` 字段(006-4 添加)
