# Contract(契约): GroupIsolation(分组隔离) API

**Feature(功能)**: `006-4-restart-policy-production`

## GroupDependencyEdge

```rust
/// Declares a failure propagation dependency between groups.
pub struct GroupDependencyEdge {
    /// The group that depends on another group.
    pub from_group: String,
    /// The group that is depended on.
    pub to_group: String,
    /// How failures propagate from `to_group` to `from_group`.
    pub propagation: PropagationPolicy,
}
```

## PropagationPolicy

```rust
/// Failure propagation policy across group boundaries.
pub enum PropagationPolicy {
    /// No propagation — groups are fully isolated.
    None,
    /// Escalate to parent supervisor only, do not affect current group.
    EscalateOnly,
    /// Full propagation — current group also enters meltdown.
    Full,
}
```

## GroupIsolationPolicy

```rust
/// Evaluates whether a failure in one group affects another group.
pub struct GroupIsolationPolicy {
    dependencies: Vec<GroupDependencyEdge>,
}

impl GroupIsolationPolicy {
    /// Creates an isolation policy from declared dependency edges.
    pub fn new(dependencies: Vec<GroupDependencyEdge>) -> Self;

    /// Checks whether `my_group` is affected by a failure in `failed_group`.
    ///
    /// Returns `true` when a dependency edge explicitly allows propagation,
    /// or when `my_group` is the same as `failed_group`.
    pub fn affected_by(&self, my_group: &str, failed_group: &str) -> bool;
}
```

## MeltdownTracker 分组维度扩展

```rust
impl MeltdownTracker {
    /// Records a failure against a specific group.
    ///
    /// # Arguments
    ///
    /// - `now`: Current monotonic instant.
    /// - `child_id`: Child that failed.
    /// - `group_name`: Group the child belongs to.
    ///
    /// # Returns
    ///
    /// Returns the [`MeltdownOutcome`] scoped to the group.
    pub fn track_group_failure(
        &mut self,
        now: Instant,
        child_id: &ChildId,
        group_name: &str,
    ) -> MeltdownOutcome;

    /// Checks whether a group fuse has fired.
    pub fn group_fuse_active(&self, group_name: &str) -> bool;

    /// Propagates a fuse to dependent groups according to isolation policy.
    ///
    /// Returns the list of groups that are now also affected.
    pub fn propagate_fuse(
        &mut self,
        failed_group: &str,
        isolation: &GroupIsolationPolicy,
    ) -> Vec<String>;
}
```

## 不变式

1. 在未声明显式 `GroupDependencyEdge` 的前提下, `affected_by(group_a, group_b)` 在 `group_a != group_b` 时必须返回 `false`
2. 当 `affected_by` 返回 `true` 且 propagation 为 `None` 时, `propagate_fuse` 不得产生新的受影响分组
3. 同一分组内所有 child 共享一个 group 级别熔断计数器
4. Supervisor 级熔断是所有 group 熔断的并集

## 与现有 MeltdownTracker 的集成

现有 `MeltdownTracker` 已支持:
- `child_max_restarts`/`child_window` — child 级熔断
- `group_max_failures`/`group_window` — group 级熔断(需扩展为按组名索引)
- `supervisor_max_failures`/`supervisor_window` — supervisor 级熔断

增强点:
- `group_counters: HashMap<String, GroupCounter>` — 按组名分别计数
- `propagate_fuse()` 方法 — 依赖边传播
- `group_fuse_active()` 查询 — 分组隔离断言
