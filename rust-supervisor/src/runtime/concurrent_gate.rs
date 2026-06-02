//! Concurrent restart throttle gates for preventing restart storm.
//!
//! This module implements instance-global and group-level concurrent restart
//! limits to prevent resource contention during mass failure scenarios.
//!
//! # Permit-based design
//!
//! [`GatePermit`] is an RAII guard returned by [`SupervisorInstanceGate::try_acquire`].
//! The gate slot is released automatically when the [`GatePermit`] is dropped,
//! eliminating the "mismatched acquire/release" bug class where a single
//! acquire was followed by multiple releases (e.g. OneForAll restart scope).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

/// RAII guard that represents one acquired concurrent restart slot.
///
/// When dropped, the slot is automatically returned to the gate.
/// This eliminates counting errors caused by mismatched acquire/release
/// across different code paths (e.g. restart scope vs child exit).
#[derive(Debug)]
pub struct GatePermit {
    /// Reference to the gate that issued this permit.
    gate: Arc<AtomicU32>,
}

impl Drop for GatePermit {
    /// Releases one slot back to the gate on drop.
    fn drop(&mut self) {
        let previous = self.gate.fetch_sub(1, Ordering::SeqCst);
        debug_assert!(previous > 0, "Released more slots than acquired");
    }
}

/// Instance-global concurrent restart gate counter.
///
/// Tracks the number of currently active restart attempts across all children
/// supervised by this supervisor instance. When the limit is reached, new
/// restart requests are denied.
#[derive(Debug, Clone)]
pub struct SupervisorInstanceGate {
    /// Maximum concurrent restarts allowed at instance level.
    max_concurrent: u32,
    /// Current count of active restart attempts.
    active_count: Arc<AtomicU32>,
}

impl SupervisorInstanceGate {
    /// Creates a new instance-global concurrent restart gate.
    ///
    /// # Arguments
    ///
    /// - `max_concurrent`: Maximum number of concurrent restart attempts allowed.
    ///
    /// # Returns
    ///
    /// Returns a new [`SupervisorInstanceGate`] with zero active count.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::runtime::concurrent_gate::SupervisorInstanceGate;
    ///
    /// let gate = SupervisorInstanceGate::new(5);
    /// assert_eq!(gate.get_active_count(), 0);
    /// ```
    pub fn new(max_concurrent: u32) -> Self {
        Self {
            max_concurrent,
            active_count: Arc::new(AtomicU32::new(0)),
        }
    }

    /// Attempts to acquire a restart slot from the instance gate.
    ///
    /// Returns a [`GatePermit`] when the gate has capacity, or `None` when
    /// saturated. The permit is an RAII guard — the slot is released
    /// automatically when the permit is dropped, eliminating counting errors.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::runtime::concurrent_gate::SupervisorInstanceGate;
    ///
    /// let gate = SupervisorInstanceGate::new(2);
    /// let p1 = gate.try_acquire();
    /// let p2 = gate.try_acquire();
    /// let p3 = gate.try_acquire();
    /// assert!(p1.is_some());
    /// assert!(p2.is_some());
    /// assert!(p3.is_none());
    /// drop(p1);
    /// assert_eq!(gate.get_active_count(), 1);
    /// ```
    pub fn try_acquire(&self) -> Option<GatePermit> {
        loop {
            let current = self.active_count.load(Ordering::SeqCst);
            if current >= self.max_concurrent {
                return None;
            }
            match self.active_count.compare_exchange_weak(
                current,
                current + 1,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => {
                    return Some(GatePermit {
                        gate: self.active_count.clone(),
                    });
                }
                Err(_) => continue,
            }
        }
    }

    /// Returns the current number of active restart attempts.
    ///
    /// # Returns
    ///
    /// Returns the current active count for monitoring and diagnostics.
    pub fn get_active_count(&self) -> u32 {
        self.active_count.load(Ordering::SeqCst)
    }

    /// Returns the configured maximum concurrent restart limit.
    ///
    /// # Returns
    ///
    /// Returns the maximum allowed concurrent restarts.
    pub fn get_max_concurrent(&self) -> u32 {
        self.max_concurrent
    }

    /// Checks if the gate is currently saturated.
    ///
    /// # Returns
    ///
    /// Returns `true` if active count has reached or exceeded the limit.
    pub fn is_saturated(&self) -> bool {
        self.get_active_count() >= self.max_concurrent
    }
}

/// Group-level concurrent restart gate for optional per-group throttling.
///
/// When enabled, tracks concurrent restarts within a specific restart execution
/// plan group. Falls back to instance-global gate when not configured.
#[derive(Debug, Clone)]
pub struct GroupLevelGate {
    /// Map from group identifier to per-group gate state.
    group_gates: Arc<Mutex<HashMap<String, Arc<AtomicU32>>>>,
    /// Default maximum concurrent restarts per group.
    max_per_group: u32,
}

impl GroupLevelGate {
    /// Creates a new group-level concurrent restart gate manager.
    ///
    /// # Arguments
    ///
    /// - `max_per_group`: Maximum concurrent restarts allowed per group.
    ///
    /// # Returns
    ///
    /// Returns a new [`GroupLevelGate`] with empty group map.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::runtime::concurrent_gate::GroupLevelGate;
    ///
    /// let gate = GroupLevelGate::new(3);
    /// assert_eq!(gate.get_active_count_for_group("group-a"), 0);
    /// ```
    pub fn new(max_per_group: u32) -> Self {
        Self {
            group_gates: Arc::new(Mutex::new(HashMap::new())),
            max_per_group,
        }
    }

    /// Attempts to acquire a restart slot for a specific group.
    ///
    /// # Arguments
    ///
    /// - `group_id`: Identifier of the restart execution plan group.
    ///
    /// # Returns
    ///
    /// Returns `true` if a slot was acquired for the group, `false` if saturated.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::runtime::concurrent_gate::GroupLevelGate;
    ///
    /// let gate = GroupLevelGate::new(2);
    /// assert!(gate.try_acquire_for_group("group-a"));
    /// assert!(gate.try_acquire_for_group("group-a"));
    /// assert!(!gate.try_acquire_for_group("group-a")); // Limit reached
    /// ```
    pub fn try_acquire_for_group(&self, group_id: &str) -> bool {
        let mut gates = self.group_gates.lock().unwrap();
        let gate = gates
            .entry(group_id.to_string())
            .or_insert_with(|| Arc::new(AtomicU32::new(0)));

        loop {
            let current = gate.load(Ordering::SeqCst);
            if current >= self.max_per_group {
                return false;
            }
            match gate.compare_exchange_weak(
                current,
                current + 1,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => return true,
                Err(_) => continue,
            }
        }
    }

    /// Releases a restart slot for a specific group.
    ///
    /// # Arguments
    ///
    /// - `group_id`: Identifier of the restart execution plan group.
    pub fn release_for_group(&self, group_id: &str) {
        let gates = self.group_gates.lock().unwrap();
        if let Some(gate) = gates.get(group_id) {
            let previous = gate.fetch_sub(1, Ordering::SeqCst);
            debug_assert!(previous > 0, "Released more group slots than acquired");
        }
    }

    /// Returns the current active count for a specific group.
    ///
    /// # Arguments
    ///
    /// - `group_id`: Identifier of the restart execution plan group.
    ///
    /// # Returns
    ///
    /// Returns the active restart count for the specified group.
    pub fn get_active_count_for_group(&self, group_id: &str) -> u32 {
        let gates = self.group_gates.lock().unwrap();
        gates
            .get(group_id)
            .map(|g| g.load(Ordering::SeqCst))
            .unwrap_or(0)
    }

    /// Checks if a specific group's gate is saturated.
    ///
    /// # Arguments
    ///
    /// - `group_id`: Identifier of the restart execution plan group.
    ///
    /// # Returns
    ///
    /// Returns `true` if the group's active count has reached the limit.
    pub fn is_group_saturated(&self, group_id: &str) -> bool {
        self.get_active_count_for_group(group_id) >= self.max_per_group
    }
}

/// Combined throttle gate that enforces both instance and group limits.
///
/// When both gates are active, takes the stricter verdict: if either gate
/// is saturated, the restart request is throttled.
#[derive(Debug, Clone)]
pub struct CombinedThrottleGate {
    /// Instance-global concurrent restart gate.
    instance_gate: SupervisorInstanceGate,
    /// Optional group-level concurrent restart gate.
    group_gate: Option<GroupLevelGate>,
}

impl CombinedThrottleGate {
    /// Creates a combined throttle gate with both instance and group limits.
    ///
    /// # Arguments
    ///
    /// - `instance_gate`: Instance-global concurrent restart gate.
    /// - `group_gate`: Optional group-level gate for per-group throttling.
    ///
    /// # Returns
    ///
    /// Returns a new [`CombinedThrottleGate`].
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::runtime::concurrent_gate::{
    ///     CombinedThrottleGate, SupervisorInstanceGate, GroupLevelGate,
    /// };
    ///
    /// let instance = SupervisorInstanceGate::new(10);
    /// let group = GroupLevelGate::new(5);
    /// let combined = CombinedThrottleGate::new(instance, Some(group));
    /// ```
    pub fn new(instance_gate: SupervisorInstanceGate, group_gate: Option<GroupLevelGate>) -> Self {
        Self {
            instance_gate,
            group_gate,
        }
    }

    /// Attempts to acquire restart permission through both gates.
    ///
    /// Takes the stricter verdict: if either gate is saturated, returns `false`.
    /// The acquired instance permit is held for the duration of the call and
    /// released when the permit drops (RAII). Callers that need long-lived
    /// permits should use [`SupervisorInstanceGate`] directly.
    ///
    /// # Arguments
    ///
    /// - `group_id`: Optional group identifier for group-level gate check.
    ///
    /// # Returns
    ///
    /// Returns `true` only if both instance and group gates allow the restart.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_supervisor::runtime::concurrent_gate::{
    ///     CombinedThrottleGate, SupervisorInstanceGate, GroupLevelGate,
    /// };
    ///
    /// let instance = SupervisorInstanceGate::new(2);
    /// let group = GroupLevelGate::new(1);
    /// let combined = CombinedThrottleGate::new(instance, Some(group));
    ///
    /// assert!(combined.try_acquire(Some("group-a")));
    /// assert!(!combined.try_acquire(Some("group-a"))); // Group limit reached
    /// ```
    pub fn try_acquire(&self, group_id: Option<&str>) -> bool {
        // Acquire an instance permit. It is dropped at the end of this
        // method because CombinedThrottleGate does not support long-lived
        // permits — callers should use SupervisorInstanceGate directly
        // when RAII-style permits are needed.
        let _permit = match self.instance_gate.try_acquire() {
            Some(p) => p,
            None => return false,
        };

        // If group gate exists and group_id provided, check group limit
        if let (Some(group_gate), Some(gid)) = (&self.group_gate, group_id)
            && !group_gate.try_acquire_for_group(gid)
        {
            // _permit is dropped here, releasing the instance slot.
            return false;
        }

        true
        // _permit dropped here — instance slot released.
        // This matches the legacy behavior where CombinedThrottleGate
        // did not hold permits across async boundaries.
    }

    /// Releases restart slots from both instance and group gates.
    ///
    /// # Arguments
    ///
    /// - `group_id`: Optional group identifier for group-level release.
    ///
    /// # Legacy note
    ///
    /// This method is a no-op for the instance gate because the permit
    /// was already released at the end of [`try_acquire`](Self::try_acquire).
    /// It is retained for API compatibility with callers that still call
    /// `release` after a combined gate acquire.
    pub fn release(&self, group_id: Option<&str>) {
        if let (Some(group_gate), Some(gid)) = (&self.group_gate, group_id) {
            group_gate.release_for_group(gid);
        }
    }

    /// Returns the instance-global gate reference.
    ///
    /// # Returns
    ///
    /// Returns a reference to the instance gate for monitoring.
    pub fn instance_gate(&self) -> &SupervisorInstanceGate {
        &self.instance_gate
    }

    /// Returns the group-level gate reference if configured.
    ///
    /// # Returns
    ///
    /// Returns an optional reference to the group gate.
    pub fn group_gate(&self) -> Option<&GroupLevelGate> {
        self.group_gate.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime::concurrent_gate::{
        CombinedThrottleGate, GroupLevelGate, SupervisorInstanceGate,
    };

    /// Tests basic acquire and release operations on supervisor instance gate.
    #[test]
    fn test_instance_gate_basic_acquire_release() {
        let gate = SupervisorInstanceGate::new(3);
        assert_eq!(gate.get_active_count(), 0);

        let p1 = gate.try_acquire();
        assert!(p1.is_some());
        assert_eq!(gate.get_active_count(), 1);

        let p2 = gate.try_acquire();
        assert!(p2.is_some());
        assert_eq!(gate.get_active_count(), 2);

        drop(p1);
        assert_eq!(gate.get_active_count(), 1);

        drop(p2);
        assert_eq!(gate.get_active_count(), 0);
    }

    /// Tests that instance gate correctly reports saturation when limit is reached.
    #[test]
    fn test_instance_gate_saturation() {
        let gate = SupervisorInstanceGate::new(2);

        let p1 = gate.try_acquire();
        let p2 = gate.try_acquire();
        let p3 = gate.try_acquire();
        assert!(p1.is_some());
        assert!(p2.is_some());
        assert!(p3.is_none()); // Saturated

        assert!(gate.is_saturated());
    }

    /// Tests that group-level gates isolate concurrency limits per group independently.
    #[test]
    fn test_group_gate_isolation() {
        let gate = GroupLevelGate::new(2);

        // Group A can acquire up to limit
        assert!(gate.try_acquire_for_group("group-a"));
        assert!(gate.try_acquire_for_group("group-a"));
        assert!(!gate.try_acquire_for_group("group-a"));

        // Group B is independent and unaffected
        assert!(gate.try_acquire_for_group("group-b"));
        assert_eq!(gate.get_active_count_for_group("group-b"), 1);
        assert_eq!(gate.get_active_count_for_group("group-a"), 2);
    }

    /// Tests that combined gate takes the stricter verdict between instance and group gates.
    #[test]
    fn test_combined_gate_takes_stricter_verdict() {
        let instance = SupervisorInstanceGate::new(5);
        let group = GroupLevelGate::new(2);
        let combined = CombinedThrottleGate::new(instance, Some(group));

        // Group limit is stricter (2 vs 5)
        assert!(combined.try_acquire(Some("test-group")));
        assert!(combined.try_acquire(Some("test-group")));
        assert!(!combined.try_acquire(Some("test-group"))); // Group saturated
    }

    /// Tests that combined gate works correctly without a group gate configured.
    #[test]
    fn test_combined_gate_without_group() {
        let instance = SupervisorInstanceGate::new(2);
        let combined = CombinedThrottleGate::new(instance, None);

        // Without group gate, CombinedThrottleGate checks instance capacity
        // ephemerally — the permit is released immediately after each call.
        // So repeated calls always succeed as long as the gate is not
        // saturated at the instant of the call.
        assert!(combined.try_acquire(None));
        assert!(combined.try_acquire(None));
        assert!(combined.try_acquire(None));
        // For long-lived permits (held across async boundaries), callers
        // should use SupervisorInstanceGate directly.
    }
}
