# 四阶段关闭挂起风险治理 - 分析与实施方案

生成日期: 2026-05-23 | 归属切片: 006-3-lifecycle-shutdown-realism / 006-6-config-dynamic-children

---

## 第一部分：风险分析

### 1. 协作式调度的致命伤

`JoinHandle::abort()` 只是在 tokio 的任务调度标识上设一个标志位，任务本身至少需要到达一个 `.await` 的 poll 点才能感知。这是理解整个挂起问题的前提。

**如果子任务中存在 `loop { ... }` 且没有 `.await`（如密集计算），或者调用了未用 `tokio::task::spawn_blocking` 包裹的同步 I/O，该线程将被永久霸占。**

#### 补充防范手段

- **强制 Yield 注入**：在业务逻辑的计算密集型循环中，要求开发者显式插入 `tokio::task::yield_now().await`，人为创造调度点，让 `abort()` 有机会介入。
- **非阻塞 I/O 强制 Lint**：通过配置 `clippy.toml` 禁用标准库的阻塞式 I/O，从编译期掐断同步阻塞的可能。

### 2. 状态机调度的隐患：pause 与锁饥饿

强行暂停（Pause）一个异步任务是一个极其危险的控制原语。如果任务在暂停时恰好持有了 `tokio::sync::Mutex`，或者持有了某个有界 Channel 的发送端，暂停它不仅会饿死该任务，还会引发隐式的级联死锁。

当前实现中 `pause` 的语义是"仅阻止自动重启"（`ChildControlOperation::Paused`），不触发生命周期变化。但**如果任务内部有 `select!` 监听多个 channel**，paused 状态的子任务仍然会消耗消息队列的容量--这是另一个隐式级联风险：如果每个子任务都通过有界 channel 与外界通信，paused 的任务会积压消息，最终导致 channel 满，阻塞 sender 侧的拓扑更新操作。

#### 补充防范手段

- **无锁设计与 Actor 模型**：在被监督的子任务中，建议废弃跨任务的共享状态锁（`Arc<Mutex<T>>`），全面转向基于消息传递（Message Passing）的 Actor 模式。
- **锁的持有期断言**：如果必须使用锁，绝对不能跨 `.await` 点持有同步锁（`std::sync::Mutex`）。对于异步锁，可以结合 `tokio-console` 实时监控长时间持有的资源。

### 3. 性能瓶颈与 OOM 风险

OOM 风险在 Rust 代码层面是现实问题。当前 `ObservabilityPipeline` 的 journal 虽然有 `capacity` 配置，但观测数据全在进程内存中。Crash loop 场景下事件日志的累积速度可能超出消费速度。

#### 补充防范手段

- **有界 Ring Buffer 日志**：事件日志绝对不能使用无界的 `Vec` 或不断追加的队列。必须在内存中强制使用固定容量的 Ring Buffer（例如 `crossbeam_queue::ArrayQueue`），当发生持续崩溃时，旧日志会被自动覆盖，并通过背压（Backpressure）机制限制重启速率。
- Ring Buffer 需要确认写入在满时是否为自动丢弃模式（`try_push` / `try_send`），而非阻塞模式（`push` / `send`）。

---

### 4. 关闭流程超时分析

#### 已有的超时（子任务级别）

当前 `execute_shutdown` 的辅助方法中已有以下超时：

| 位置                               | 超时覆盖范围                                      | 保护边界                     |
| ---------------------------------- | ------------------------------------------------- | ---------------------------- |
| `drain_graceful_children()`        | 每个子任务 `timeout(remaining, wait_for_report)`  | 只到 `graceful_timeout` 截止 |
| `abort_remaining_children()`       | 每个子任务 `timeout(abort_wait, wait_for_report)` | 只到 `abort_wait` 截止       |
| `shutdown_tree_fanout()` Phase 2/4 | 同上逻辑                                          | 同上                         |

#### 仍然缺失的全局保护

| 缺口                                         | 描述                                                                                       | 严重程度 |
| -------------------------------------------- | ------------------------------------------------------------------------------------------ | -------- |
| `execute_shutdown()` 方法体无 `timeout` 包裹 | 如果内部 helper 意外阻塞，永不到达 `Ok(report)`                                            | 🔴       |
| `run_control_loop` 主循环无心跳              | 如果 `execute_control` 卡死，控制循环停止处理所有消息                                      | 🔴       |
| 全局截止时间无 Margin                        | `drain_graceful_children()` 串行 per-child timeout，调度器抖动可能导致正常任务被误判为超时 | 🟡       |

### 5. Phase 5 的 "孤儿化" 语义

`AbortHandle::abort()` 不能强杀 OS 线程，`take()` + `deactivate()` 只是让 supervisor 的内存状态忘记这个任务的存在。底层的 Tokio `JoinHandle` 不再追踪它，但 OS 线程被死循环或阻塞 I/O 永久霸占。

**收益与风险的权衡**：

- 对于一个 supervisor 进程，泄漏 1 个线程意味着最多损失 1 个线程槽位
- 比 "整个 supervisor tree 死锁" 好一个数量级--后者是所有线程和子任务一起挂起
- 操作员可以通过健康检查发现线程泄漏，然后优雅重启 supervisor

#### 强制孤儿化的时序模型

```
┌─ Supervisor 控制循环 ──────────────────────┐
│                                             │
│  execute_shutdown() {                       │
│      Phase 1: cancel()       → 发送取消信号  │
│      Phase 2: timeout(wait)  → 超时          │
│      Phase 3: abort()        → 设置 abort 位  │
│      Phase 4: timeout(wait)  → 仍然超时       │
│      ─────────────────────────────────────   │
│      Phase 5: force_kill {                  │
│          drop(receiver);      // 断开监听    │
│          slot.deactivate();   // 清空槽位    │
│          slot.clear_instance();// 忘记任务    │
│          // JoinHandle 被 drop              │
│          // OS 线程继续运行但 supervisor 不再关心  │
│      }                                      │
│  }                                           │
│                                              │
│  控制循环返回 → 继续服务其他子任务 ✅          │
└──────────────────────────────────────────────┘

┌─ 被孤立的 OS 线程 ─────────────────────────┐
│  loop { heavy_computation() }                │
│  // 没有 .await，没有被 tokio 调度            │
│  // JoinHandle 已被 drop                     │
│  // 线程继续运行直到进程退出                    │
└──────────────────────────────────────────────┘
```

#### 架构决策：exit 还是继续运行？

| 方案               | 行为                                | 适用条件                                        |
| ------------------ | ----------------------------------- | ----------------------------------------------- |
| **方案 B（默认）** | 标记 `Failed` + 继续运行健康子树    | `orphan_count < MAX_ORPHAN_THRESHOLD`（默认 3） |
| **方案 A（降级）** | 受控 `std::process::exit(EX_OSERR)` | `orphan_count >= MAX_ORPHAN_THRESHOLD`          |

三阶段状态：

- 🟢 `orphan_count == 0`: `RuntimeHealthReport.state = "healthy"`
- 🟡 `orphan_count ∈ [1, MAX_ORPHAN_THRESHOLD)`: `RuntimeHealthReport.state = "degraded"`，发射 degraded 事件，继续服务
- 🔴 `orphan_count >= MAX_ORPHAN_THRESHOLD`: 受控关闭 + `std::process::exit`

### 6. 容错余量 (Margin)

`graceful_timeout + abort_wait` 之后必须加一个 **5 秒的 Margin**：

```
effective_global_deadline = graceful_timeout + abort_wait + force_kill_margin
                          = 5s + 1s + 5s
                          = 11s
```

**为什么不能直接设成 `graceful + abort`？**

- Tokio 的 `timeout` 基于 `Instant::now()` 判断，但任务的实际就绪可能因调度延迟而在超时边界处漂移几毫秒
- `drain_graceful_children()` 是逐个遍历等待的（串行 per-child timeout），如果某个子任务恰好在超时边界返回，下一个子任务可能整体落后于计划
- Margin 的本质是给"合法但刚好卡边的任务"一个缓冲窗口，避免 Type I Error（误杀正常任务）

如果 `execute_shutdown` 在 Margin 窗口内自然完成，则全局超时永不触发--这就是兜底与正常路径的优雅共存。

---

## 第二部分：修订后的实施方案（工程矩阵）

### 执行矩阵总览

| #   | 改进项                                    | 优先级 | 阶段    | 依赖   | 并行   | 预估工时 | 风险影响            |
| --- | ----------------------------------------- | ------ | ------- | ------ | ------ | -------- | ------------------- |
| 1   | `ShutdownPolicy` 新增 `force_kill_margin` | P0     | Phase 0 | -      | -      | 4h       | 🔴 直接决定超时精度 |
| 2   | `execute_shutdown` 注入全局硬超时         | P0     | Phase 0 | Step 1 | -      | 4h       | 🔴 阻止全局死锁     |
| 3   | `emergency_force_kill` + orphan 计数器    | P0     | Phase 0 | -      | Step 2 | 8h       | 🔴 切断线程泄漏传播 |
| 4   | 主动降级检测 + 受控进程退出               | P0     | Phase 0 | Step 3 | -      | 6h       | 🔴 孤儿溢出保护     |
| 5   | Clippy 阻塞 I/O Lint                      | P1     | Phase 1 | -      | 全部   | 2h       | 🟡 编译期阻断       |
| 6   | Ring Buffer 背压确认                      | P1     | Phase 1 | -      | Step 5 | 3h       | 🟡 防止 OOM         |
| 7   | Kani 状态机证明                           | P2     | Phase 2 | -      | 全部   | 16h      | 🟢 数学验证         |
| 8   | Yield 点编码契约                          | P2     | Phase 2 | -      | 全部   | 2h       | 🟢 编码规范         |

---

### Step 1 - `ShutdownPolicy` 新增 `force_kill_margin` 字段

**优先级**: P0 | **阶段**: Phase 0 | **预估工时**: 4h

#### 修改文件

| 文件                          | 变更类型 | 说明                                                                                    |
| ----------------------------- | -------- | --------------------------------------------------------------------------------------- |
| `src/shutdown/stage.rs`       | 修改     | `ShutdownPolicy` 新增 `force_kill_margin: Duration` 字段                                |
| `src/shutdown/stage.rs`       | 新增     | `effective_global_deadline()` 方法: `graceful_timeout + abort_wait + force_kill_margin` |
| `src/runtime/supervisor.rs`   | 修改     | `shutdown_policy_from_spec()` 更新构造调用，传递默认值 `Duration::from_secs(5)`         |
| `src/spec/supervisor.rs`      | 修改     | `SupervisorSpec` 暴露 `force_kill_margin` 配置字段                                      |
| `src/shutdown/coordinator.rs` | 无变动   | `ShutdownCoordinator` 本身不依赖此字段，仅透传                                          |

#### 变更内容

```diff
  pub struct ShutdownPolicy {
      pub graceful_timeout: Duration,
      pub abort_wait: Duration,
      pub abort_after_timeout: bool,
+     /// Extra grace beyond graceful_timeout + abort_wait before the
+     /// global hard deadline is enforced.
+     /// Recommended default: 5 seconds.
+     pub force_kill_margin: Duration,
  }

  impl ShutdownPolicy {
-     pub fn new(graceful: Duration, abort: Duration, abort_after: bool) -> Self
+     pub fn new(graceful: Duration, abort: Duration, abort_after: bool, margin: Duration) -> Self

+     pub fn effective_global_deadline(&self) -> Duration {
+         self.graceful_timeout + self.abort_wait + self.force_kill_margin
+     }
  }
```

#### 验证

```rust
#[test]
fn effective_deadline_includes_margin() {
    let policy = ShutdownPolicy::new(
        Duration::from_secs(5),
        Duration::from_secs(1),
        true,
        Duration::from_secs(5),
    );
    assert_eq!(policy.effective_global_deadline(), Duration::from_secs(11));
}
```

---

### Step 2 - `execute_shutdown` 注入全局硬超时

**优先级**: P0 | **阶段**: Phase 0 | **依赖**: Step 1 | **预估工时**: 4h

#### 修改文件

| 文件                          | 变更类型 | 说明                                                                          |
| ----------------------------- | -------- | ----------------------------------------------------------------------------- |
| `src/runtime/control_loop.rs` | 修改     | `execute_shutdown()` 最外层注入 `tokio::time::timeout(global_deadline, body)` |

#### 变更逻辑

```
execute_shutdown(requested_by, reason, event_sender):
    if cached_report:
        return idempotent result

    global_deadline = self.shutdown.policy.effective_global_deadline()

    match timeout(global_deadline, self.shutdown_body(requested_by, reason, event_sender)).await:
        Ok(Ok(result)) => Ok(result)
        Ok(Err(e)) => Err(e)
        Err(_elapsed) =>
            // 超时路径:
            // 1. 确保状态机推进到 Completed
            self.advance_shutdown_phase(event_sender)  // 直至 Completed
            self.shutdown.complete()
            // 2. 发射诊断事件
            event_sender.send("shutdown_global_timeout")
            // 3. 构造失败的 ShutdownResult
            Err(SupervisorError::fatal("shutdown global timeout exceeded"))
```

#### 注意点

- `shutdown_body` 可以提取为私有 async 方法以保持代码结构清晰
- 超时后必须**强制推进状态机到 `Completed`**，否则后续操作可能误判相位
- 超时返回值是 `Result<Result<ShutdownResult, SupervisorError>, Elapsed>`，两层 Result 的嵌套需要正确处理

---

### Step 3 - `emergency_force_kill` + orphan 计数器

**优先级**: P0 | **阶段**: Phase 0 | **并行**: 可与 Step 2 并行 | **预估工时**: 8h

#### 修改文件

| 文件                          | 变更类型 | 说明                                                                         |
| ----------------------------- | -------- | ---------------------------------------------------------------------------- |
| `src/runtime/shutdown.rs`     | 修改     | `shutdown_tree_fanout()` 中 Phase 5 重构为 `emergency_force_kill()` 辅助函数 |
| `src/runtime/control_loop.rs` | 修改     | `RuntimeControlState` 新增 `orphan_count: u64` 字段                          |
| `src/runtime/lifecycle.rs`    | 修改     | `RuntimeHealthReport` 新增 `orphan_count` 字段                               |
| `src/runtime/child_slot.rs`   | 无变动   | 复用现有 `deactivate()` + `clear_instance()`                                 |

#### `emergency_force_kill()` 函数签名

```rust
/// Orphans one slot that cannot be drained or aborted within policy
/// timeouts. Returns the orphaned slot's child_id for event emission.
///
/// This function:
/// 1. Deactivates the slot (clears handles, records exit summary)
/// 2. Clears all instance fields to guarantee clean state
/// 3. Returns a diagnostic string for event emission
fn emergency_force_kill(
    slots: &mut HashMap<ChildId, ChildSlot>,
    child_id: &ChildId,
    orphan_count: &mut u64,
) -> Option<String> {
    let slot = slots.get_mut(child_id)?;
    if !slot.has_active_attempt() {
        return None;
    }

    let generation = slot.generation.map_or(0, |g| g.value);
    let attempt = slot.attempt.map_or(0, |a| a.value);

    slot.deactivate(ChildExitSummary {
        exit_code: None,
        exit_reason: "shutdown force kill timeout; task orphaned".to_owned(),
        exited_at_unix_nanos: 0,
    });
    slot.clear_instance();

    *orphan_count += 1;

    Some(format!(
        "child_orphaned:{}:generation={}:attempt={}",
        child_id, generation, attempt
    ))
}
```

#### `RuntimeControlState` 新增

```rust
pub struct RuntimeControlState {
    // ... existing fields ...
    /// Active count of orphaned child tasks that could not be
    /// stopped within policy timeouts.
    pub orphan_count: u64,
}
```

---

### Step 4 - 主动降级检测 + 受控进程退出

**优先级**: P0 | **阶段**: Phase 0 | **依赖**: Step 3 | **预估工时**: 6h

#### 修改文件

| 文件                          | 变更类型 | 说明                                                                    |
| ----------------------------- | -------- | ----------------------------------------------------------------------- |
| `src/runtime/control_loop.rs` | 修改     | `execute_shutdown()` 执行孤儿检测逻辑                                   |
| `src/runtime/lifecycle.rs`    | 修改     | `RuntimeHealthReport` / `RuntimeControlPlaneState` 增加 `Degraded` 状态 |
| `src/shutdown/stage.rs`       | 修改     | `ShutdownPolicy` 新增 `max_orphan_threshold: u32` 字段                  |

#### 三阶段状态机

```
orphan_count == 0
    → RuntimeHealthReport.state = "healthy"
    → 正常运行

orphan_count ∈ [1, MAX_ORPHAN_THRESHOLD)
    → RuntimeHealthReport.state = "degraded"
    → 发射 degraded 事件, 继续服务
    → 操作员可主动修复（排查阻塞子任务）

orphan_count >= MAX_ORPHAN_THRESHOLD
    → RuntimeControlPlane 标记 ShuttingDown
    → 发射 fatal_orphan_overflow 事件
    → std::process::exit(EX_OSERR)
```

#### 变更内容

```diff
  impl ShutdownPolicy {
      pub fn new(graceful: Duration, abort: Duration, abort_after: bool, margin: Duration) -> Self
+     pub fn max_orphan_threshold(&self) -> u32 { self.max_orphan_threshold }
+     pub fn set_max_orphan_threshold(&mut self, val: u32) { ... }
  }

+ /// 在 ShutdownPolicy 中新增:
+ ///     max_orphan_threshold: u32,  // 默认 3
```

#### 触发路径

```rust
// execute_shutdown() 返回前:
if self.orphan_count >= self.shutdown.policy.max_orphan_threshold as u64 {
    event_sender.send(format!(
        "fatal_orphan_overflow:count={}:threshold={}",
        self.orphan_count,
        self.shutdown.policy.max_orphan_threshold,
    ));
    // 注意: 此处健康子任务已完成关闭, 不会中断请求
    std::process::exit(EX_OSERR);
}
```

---

### Step 5 - Clippy 阻塞 I/O Lint

**优先级**: P1 | **阶段**: Phase 1 | **并行**: 可全并行 | **预估工时**: 2h

#### 新增文件

| 文件          | 说明                     |
| ------------- | ------------------------ |
| `clippy.toml` | 项目根目录，禁用阻塞调用 |

#### 变更内容

```toml
# clippy.toml
# 禁止在异步上下文中使用阻塞 I/O 和同步锁。
# 子任务代码必须使用 tokio::fs, tokio::net, tokio::sync::Mutex,
# tokio::time::sleep 替代。
disallowed-methods = [
    "std::fs::read",
    "std::fs::write",
    "std::fs::File",
    "std::net::TcpStream",
    "std::thread::sleep",
    "std::sync::Mutex",
]
```

#### CI 集成

在 `.github/workflows/` 下的 main workflow 中添加：

```yaml
- name: Clippy (blocking I/O lint)
  run: cargo clippy -- -D warnings
```

---

### Step 6 - Ring Buffer 背压确认

**优先级**: P1 | **阶段**: Phase 1 | **并行**: 可全并行 | **预估工时**: 3h

#### 审查文件

| 文件                  | 说明           |
| --------------------- | -------------- |
| `src/journal/ring.rs` | 环形缓冲区实现 |

#### 审查要点

1. 确认写入接口在满时的行为：
   - ✅ `try_push` / `try_send` → 自动丢弃旧条目，不阻塞
   - ❌ `push` / `send`（阻塞）→ 改为 `try_push` 或 `force_push`
2. 确认读取接口不会在高水位下 OOM
3. 添加单元测试：

```rust
#[test]
fn ring_buffer_oldest_overwritten_on_full() {
    let mut buf = RingBuffer::new(4);
    for i in 0..4 { buf.push(i); }
    buf.push(4);
    assert_eq!(buf.len(), 4);
    assert_eq!(buf.iter().next(), Some(&1));
}

#[test]
fn ring_buffer_no_panic_on_overwrite_storm() {
    let mut buf = RingBuffer::new(100);
    for i in 0..10_000 { buf.push(i); }
    assert_eq!(buf.len(), 100);
}
```

---

### Step 7 - Kani 状态机证明

**优先级**: P2 | **阶段**: Phase 2 | **并行**: 可全并行 | **预估工时**: 16h

#### 新增文件

| 文件                                                  | 说明          |
| ----------------------------------------------------- | ------------- |
| `src/tests/kani/shutdown_coordinator_verification.rs` | Kani 证明代码 |

#### 安装

```bash
cargo install kani-verifier
cargo kani --tests shutdown_coordinator_verification
```

#### 证明内容

| 证明项             | 断言                                                            | 覆盖度 |
| ------------------ | --------------------------------------------------------------- | ------ |
| 相位单调性         | `request_stop()` 后相位 ≠ `Idle`                                | 100%   |
| 相位推进完整性     | `advance()` 不会跳过任何阶段，`next()` 覆盖所有 6 个相位        | 100%   |
| 相位不可倒退       | `advance()` 序列中相位值单调不减                                | 100%   |
| 幂等性             | 多次 `request_stop()` 返回相同的 `cause`，`idempotent` 标志正确 | 100%   |
| `Completed` 终止性 | 到达 `Completed` 后 `next()` 返回 `None`                        | 100%   |

#### Kani 证明代码（完整版）

```rust
#[cfg(kani)]
mod verification {
    use crate::shutdown::coordinator::*;
    use crate::shutdown::stage::*;

    #[kani::proof]
    fn phase_transition_monotonic() {
        let policy = ShutdownPolicy::new(
            kani::any(),
            kani::any(),
            kani::any(),
            kani::any(),
        );
        let mut coord = ShutdownCoordinator::new(policy);
        let cause = ShutdownCause::new("test", "verify");

        // 1. Idle → request_stop → phase != Idle
        let r1 = coord.request_stop(cause.clone());
        assert!(r1.phase != ShutdownPhase::Idle);
        assert!(!r1.idempotent);

        // 2. 幂等性: 第二次 request_stop 返回 idempotent=true
        let r2 = coord.request_stop(cause.clone());
        assert!(r2.idempotent);
        assert_eq!(r1.cause, r2.cause);

        // 3. 多次 advance, 相位单调前进
        let mut prev = coord.phase();
        for _ in 0..6 {
            coord.advance();
            let curr = coord.phase();
            assert!(prev.next() == Some(curr) || prev == curr);
            prev = curr;
        }

        // 4. Completed 后 next() 返回 None
        coord.complete();
        assert!(coord.phase().next().is_none());
    }
}
```

---

### Step 8 - Yield 点编码契约

**优先级**: P2 | **阶段**: Phase 2 | **并行**: 可全并行 | **预估工时**: 2h

#### 新增文件

| 文件                            | 说明                |
| ------------------------------- | ------------------- |
| `scripts/check-yield-points.sh` | CI 中执行的检查脚本 |

#### 脚本内容

```bash
#!/usr/bin/env bash
# 检查 src/ 下 Rust 源码中 loop 块是否在 50 行内含有 yield 点
# 非阻断告警, CI 中仅输出警告信息

src_dir="src"
violations=0

while IFS= read -r file; do
    grep -n 'loop\s*{' "$file" | while IFS=: read -r line_no content; do
        tail -n +"$line_no" "$file" | head -n 50 | grep -q 'yield_now\|\.await'
        if [ $? -ne 0 ]; then
            echo "WARNING: $file:$line_no: loop without yield point in next 50 lines"
            ((violations++))
        fi
    done
done < <(find "$src_dir" -name '*.rs')

if [ "$violations" -gt 0 ]; then
    echo "⚠️  Found $violations loop(s) without yield points (non-blocking)"
fi
```

#### CI 集成

```yaml
- name: Yield point check
  run: bash scripts/check-yield-points.sh
  continue-on-error: true # 非阻断告警
```

---

## 执行的时序模型

```
时间线
├── 0ms         execute_shutdown 进入
├── 0ms         Phase 1: cancel() 投递到所有 active slot
├── 0 ~ 5000ms  Phase 2: Graceful Drain (graceful_timeout = 5s)
│                 每子任务 timeout(remaining, wait_for_report)
├── 5000ms      Phase 3: abort() 投递到残余 slot
├── 5000~6000ms Phase 4: Abort Wait (abort_wait = 1s)
│                 每子任务 timeout(1s, wait_for_report)
├── 6000ms      Phase 5: emergency_force_kill()
│                 drop(receiver) + deactivate() + clear_instance()
│                 emit child_orphaned 事件 + orphan_count++
├── 6000~11000ms  容错余量 (force_kill_margin = 5s)
└── 11000ms     全局硬超时触发 → force-return ExitReport::failed
```

---

## 当前代码调查结果

| 缺口                                   | 搜索条件                                    | 结果      |
| -------------------------------------- | ------------------------------------------- | --------- |
| `force_kill_margin` 是否已存在         | `grep -r force_kill_margin src/`            | ❌ 不存在 |
| `orphan_count` 是否已存在              | `grep -r orphan_count src/`                 | ❌ 不存在 |
| 全局 `timeout` 包裹 `execute_shutdown` | `grep -r 'timeout.*execute_shutdown' src/`  | ❌ 不存在 |
| `emergency_force_kill` 是否存在        | `grep -r emergency_force_kill src/`         | ❌ 不存在 |
| `clippy.toml` 是否存在                 | `ls clippy.toml`                            | ❌ 不存在 |
| `Degraded` 状态是否存在                | `grep -r Degraded src/runtime/lifecycle.rs` | ❌ 不存在 |
| `max_orphan_threshold` 是否存在        | `grep -r max_orphan src/`                   | ❌ 不存在 |
| Kani 测试是否存在                      | `grep -r 'kani::proof' src/tests/`          | ❌ 不存在 |
| Yield 脚本是否存在                     | `ls scripts/check-yield-points.sh`          | ❌ 不存在 |

**结论：8 个缺口全部未实现。** 所有改动均为新增代码，不与现有逻辑冲突。

---

## 关键决策汇总

| 决策                      | 选择                         | 理由                                   |
| ------------------------- | ---------------------------- | -------------------------------------- |
| 默认 force_kill_margin    | `Duration::from_secs(5)`     | 防止调度器抖动导致 Type I Error        |
| 默认 MAX_ORPHAN_THRESHOLD | `3`                          | 适合 multi_thread runtime (CPU x 8~16) |
| 超时后状态机推进          | 必须强制 advance → Completed | 防止后续操作误判相位                   |
| std::process::exit 时机   | orphan_count ≥ threshold     | 99% 子任务健康时不盲目重启             |
| Clippy Lint 严格度        | `-D warnings`（阻断）        | 编译期切断所有阻塞调用                 |
| Kani 运行时机             | 手动 / 发布检查              | 不在常规 CI 中避免慢速                 |
| Yield 点检查              | 非阻断告警                   | 历史遗留代码需要迁移窗口               |
| 超时兜底 vs 形式化验证    | 工程兜底优先                 | 状态机证明不能解决运行时阻塞           |

---

## 依赖关系图

```
Step 1 ───── Step 2
  │              │
  │              └── 依赖 Step 1（获取 global_deadline）
  │
  └────────── 无其他依赖

Step 3 ───── Step 4
                │
                └── 依赖 Step 3（orphan_count 字段）

Step 5 ──── Step 6 ──── Step 7 ──── Step 8
                                        │
                                        └── 所有 P1/P2 可全并行

可并行批次:
  Batch A: Step 1, Step 3, Step 5, Step 6, Step 7, Step 8  ← 第一天启动
  Batch B: Step 2, Step 4                                   ← 第二天启动
```

---

## 工时汇总

| 阶段     | 步骤     | 预估工时 | 实际可用窗口            |
| -------- | -------- | -------- | ----------------------- |
| Phase 0  | Step 1-4 | 22h      | 3-5 天                  |
| Phase 1  | Step 5-6 | 5h       | 1 天（与 Phase 0 重叠） |
| Phase 2  | Step 7-8 | 18h      | 可延迟到发布窗口        |
| **合计** | **8 步** | **45h**  | **约 1 周**             |

---

## 考虑推迟的项

- **Actor 模型重构**（`src/task/actor.rs`）：P3，涉及大规模重构，不应与 P0 混入同一发布周期
- **Verus 引入**：不推荐。对当前代码库侵入性过高，收益有限
