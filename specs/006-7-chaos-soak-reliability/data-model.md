# Data Model(数据模型): 混沌与浸泡数据模型

**Branch(分支)**: `006-7-chaos-soak-reliability` | **Date(日期)**: 2026-05-19 | **Spec(规格)**: `specs/006-7-chaos-soak-reliability/spec.md`

## 1. 实体关系总览

```text
+--------------------+       +---------------------+
|  ChaosScenario     |       |  ScenarioVerdict    |
|--------------------|       |---------------------|
| scenario_id        | 1..*  | scenario_id         |
| fault_injection    |------>| semver              |
| primary_threshold  |       | passed              |
| secondary_threshold|       | thresholds: Map     |
| expected_exit_code |       | started_at_unix_nanos|
+--------------------+       | duration_ns         |
        |                    | error: Option       |
        |                    +---------------------+
        |
        | uses
        v
+--------------------+       +---------------------+
|  ChaosFixture      |       |  SoakReport         |
|--------------------|       |---------------------|
| fixture_id         |       | window_start_utc    |
| setup()            |       | window_end_utc      |
| inject_fault()     |       | commit_hash         |
| teardown()         |       | hardware_config     |
+--------------------+       | thresholds_table    |
                             | violations: List    |
        +--------------------| exemptions: List    |
        |                    | attachment_hashes   |
        v                    +---------------------+
+---------------------+
|  RateLimiter        |
|---------------------|
| window_duration(1s) |
| token_capacity(100) |
| refill_rate(50/s)   |
| try_acquire()->bool |
+---------------------+

+---------------------+
|  ClientClassification|
|---------------------|
| is_legitimate()->bool|
+---------------------+
```

## 2. 实体定义

### ChaosScenario(混沌场景)

表示一个可复跑的故障波形测试场景.

| 字段                  | 类型                 | 说明                                   |
| --------------------- | -------------------- | -------------------------------------- |
| `scenario_id`         | `string(snake_case)` | 场景唯一标识, 如 `child_panic_storm`   |
| `fault_injection`     | `string`             | 故障注入方式的人类可读描述             |
| `primary_threshold`   | `Threshold`          | 主要通过条件(如 self_panic_count = 0)  |
| `secondary_threshold` | `Threshold`          | 次要通过条件(如 emit 延迟 p99 < 100µs) |
| `expected_exit_code`  | `integer`            | 场景期望退出码(0 表示通过)             |
| `execution_window`    | `Duration`           | 场景最大执行时间窗口                   |

**Threshold(阈值)**: `{ metric: string, value: f64, limit: f64, unit: string }`

### ScenarioVerdict(场景判决书)

表示单次混沌场景运行的判定结果. 序列化为 JSON.

| 字段                    | 类型                           | 必填 | 说明                             |
| ----------------------- | ------------------------------ | ---- | -------------------------------- |
| `scenario_id`           | `string`                       | 是   | 场景标识, snake_case             |
| `semver`                | `string`                       | 是   | 从 `CARGO_PKG_VERSION` 读取      |
| `passed`                | `bool`                         | 是   | 是否通过所有阈值检查             |
| `thresholds`            | `Map<string, ThresholdResult>` | 是   | 每个阈值的实际值/限制值/通过状态 |
| `started_at_unix_nanos` | `u64`                          | 是   | 启动时间的 unix 时间戳(纳秒)     |
| `duration_ns`           | `u64`                          | 是   | 执行耗时(纳秒)                   |
| `error`                 | `Option<string>`               | 否   | 错误信息, 失败时填充             |

**ThresholdResult**: `{ value: f64, limit: f64, passed: bool }`

### SoakReport(浸泡报告)

表示一次浸泡测试的运行结果.

| 字段                | 类型                  | 必填 | 说明                   |
| ------------------- | --------------------- | ---- | ---------------------- |
| `window_start_utc`  | `string(ISO 8601)`    | 是   | 测试窗起始时间         |
| `window_end_utc`    | `string(ISO 8601)`    | 是   | 测试窗结束时间         |
| `commit_hash`       | `string`              | 是   | supervisor commit hash |
| `hardware_config`   | `string`              | 是   | 硬件配置描述           |
| `thresholds_table`  | `ThresholdRow[]`      | 是   | 阈值对照表             |
| `violations`        | `Violation[]`         | 否   | 越界条目列表           |
| `exemptions`        | `Exemption[]`         | 否   | 豁免工单列表           |
| `attachment_hashes` | `Map<string, string>` | 否   | 附件 sha256 哈希       |

**ThresholdRow(阈值行)**: `{ metric: string, p99: f64, avg: f64, max: f64, limit: f64, passed: bool }`

**Violation(越界)**: `{ metric: string, actual_value: f64, limit: f64, blocking: bool, exemption_ticket: Option<string> }`

**Exemption(豁免)**: `{ ticket_id: string, metric: string, reason: string, expiry: string(ISO 8601) }`

### RateLimiter(速率限制器)

IPC 服务端连接速率控制.

| 字段                     | 类型       | 默认值 | 说明           |
| ------------------------ | ---------- | ------ | -------------- |
| `window_duration`        | `Duration` | 1s     | 固定窗口时长   |
| `token_capacity`         | `u32`      | 100    | 令牌桶容量     |
| `refill_rate`            | `f64`      | 50.0   | 每秒令牌恢复数 |
| `tokens`                 | `f64`      | -      | 当前可用令牌数 |
| `last_refill_unix_nanos` | `u64`      | -      | 上次恢复时间戳 |

**方法**: `try_acquire() -> bool` 尝试获取一个令牌, 成功返回 true, 失败返回 false.

### ClientClassification(客户端分类)

判断 IPC 客户端是否合法的分类器.

| 字段            | 类型     | 说明                                |
| --------------- | -------- | ----------------------------------- |
| `payload`       | `string` | 客户端发送的原始 payload            |
| `is_legitimate` | `bool`   | 是否符合 dashboard IPC 协议握手格式 |

**合法判定**: payload 必须是合法 JSON, 包含 `target_id` 字段, 且字段值类型为字符串.

## 3. 数据流

### 混沌场景执行流

```text
chaos_suite.rs (test entry)
  |
  v
ScenarioRouter::run_all()
  |  for each scenario_id in FR-001 list:
  v
ScenarioRunner::run(scenario_id)
  |
  +-> FixtureSetup::setup()          -- 创建临时目录, 构建 SupervisorSpec
  +-> FaultInjector::inject()        -- 注入特定故障(panic/block/cancel 等)
  +-> Supervisor::start_with_policy() -- 启动监督器
  +-> MetricsCollector::collect()    -- 采集阈值指标
  +-> AssertionEngine::verify()      -- 验证阈值是否满足
  +-> VerdictWriter::write_json()    -- 输出 JSON 判决书到 stdout
  +-> FixtureSetup::teardown()       -- 清理资源
```

### 浸泡测试执行流

```text
soak_suite.rs (test entry, #[ignore])
  |
  v
SoakRuntime::run(Duration::from_secs(86400))
  |
  +-> SteadyTrafficGenerator::start()  -- 启动合成稳态流量(1000 req/s)
  +-> MetricsCollector::start()        -- 启动指标采集(每秒采样)
  |     |  latency_p99 (1s sliding window)
  |     |  rss_mb (every 60s; 平台相关: Linux 读 /proc/self/status, macOS 调 libc::proc_pidinfo)
  |     |  fd_count (every 60s)
  |     |  event_gap_total (every 60s)
  +-> [24h window]
  +-> ShutdownSequence::run(100x)      -- 合成关停 100 次
  |     |  shutdown_success_ratio >= 0.99
  +-> ReportGenerator::generate()      -- 生成 SoakReport Markdown
  +-> ReportWriter::write(path)        -- 写入 artifacts/validation/
```

## 4. 序列化格式

### JSON 判决书(单条场景)

```json
{
  "scenario_id": "child_panic_storm",
  "semver": "0.1.2",
  "passed": true,
  "thresholds": {
    "self_panic_count": { "value": 0, "limit": 0, "passed": true },
    "emit_latency_p99_us": { "value": 42, "limit": 100, "passed": true }
  },
  "started_at_unix_nanos": 1716000000000000000,
  "duration_ns": 60000000000,
  "error": null
}
```

### SoakReport Markdown(浸泡报告)

```markdown
# SoakReport

## Metadata

- **Window**: 2026-05-19T00:00:00Z - 2026-05-20T00:00:00Z
- **Commit**: abcdef1234567890
- **Hardware**: macOS Apple Silicon, 16GB

## Thresholds

| Metric                 | p99  | Avg   | Max   | Limit | Passed |
| ---------------------- | ---- | ----- | ----- | ----- | ------ |
| p99_latency_ms         | 23.5 | 12.1  | 156.3 | 50.0  | true   |
| rss_growth_mb_per_hour | 1.2  | 0.8   | 3.1   | 5.0   | true   |
| fd_count_drift         | 2    | 1     | 5     | 10    | true   |
| event_gap_total        | 0    | 0     | 0     | 0     | true   |
| shutdown_success_ratio | 0.99 | 0.995 | 1.0   | 0.99  | true   |

## Violations

(none)

## Exemptions

(none)

## Attachments

- p99_latency_curve.png: sha256:aabb...
- rss_curve.png: sha256:ccdd...
- fd_count_curve.png: sha256:eeff...
```
