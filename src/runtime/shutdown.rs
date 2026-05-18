//! Shutdown fan-out and timeout management for the supervisor tree.
//!
//! This module provides `shutdown_tree_fanout` which propagates cancellation to
//! every [`ChildSlot`], waits for graceful drain, aborts stragglers, and
//! returns a per-child outcome vector.

use crate::id::types::{ChildId, ChildStartCount, Generation};
use crate::runtime::admission::AdmissionSet;
use crate::runtime::child_slot::{ChildExitSummary, ChildSlot};
use crate::shutdown::report::{
    ChildShutdownOutcome, ChildShutdownOutcomeInput, ChildShutdownStatus,
};
use crate::shutdown::stage::{ShutdownPhase, ShutdownPolicy};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::{Instant, timeout};

// ---------------------------------------------------------------------------
// shutdown_tree_fanout
// ---------------------------------------------------------------------------

/// Propagates shutdown to every slot: cancel, wait up to
/// `graceful_timeout`, abort stragglers, wait up to `abort_wait`, then
/// deactivate and collect outcomes.
///
/// # Arguments
///
/// - `slots`: Mutable map of child slots owned by the runtime.
/// - `policy`: Shutdown timing policy (graceful + abort windows).
/// - `admission`: Admission set to release after each slot finishes.
///
/// # Returns
///
/// Returns a vector of [`ChildShutdownOutcome`] values, one per slot.
pub async fn shutdown_tree_fanout(
    slots: &mut HashMap<ChildId, ChildSlot>,
    policy: &ShutdownPolicy,
    admission: &mut AdmissionSet,
) -> Vec<ChildShutdownOutcome> {
    let global_deadline = Instant::now() + policy.graceful_timeout + policy.abort_wait;
    let graceful_deadline = Instant::now() + policy.graceful_timeout;

    // Phase 1: deliver cancellation to every active slot.
    for slot in slots.values_mut() {
        if slot.has_active_attempt() {
            slot.cancel();
        }
    }

    // Phase 2: wait for cooperative drain within graceful_timeout.
    let child_ids: Vec<ChildId> = slots.keys().cloned().collect();
    for child_id in &child_ids {
        let remaining = remaining_duration(graceful_deadline);
        let completed = drain_one_slot(slots, child_id, remaining).await;
        if completed {
            admission.release(child_id);
        }
    }

    // Phase 3: abort remaining active slots.
    if policy.abort_after_timeout {
        for slot in slots.values_mut() {
            if slot.has_active_attempt() && !slot.abort_requested {
                slot.abort();
            }
        }
    }

    // Phase 4: wait for aborted slots within abort_wait (bounded by global
    // deadline).
    let abort_deadline = graceful_deadline + policy.abort_wait;
    for child_id in &child_ids {
        let remaining = remaining_duration(abort_deadline.min(global_deadline));
        let completed = drain_one_slot(slots, child_id, remaining).await;
        if completed {
            admission.release(child_id);
        }
    }

    // Phase 5: force-deactivate any slot still holding handles.
    for child_id in &child_ids {
        if let Some(slot) = slots.get_mut(child_id)
            && slot.has_active_attempt()
        {
            slot.deactivate(ChildExitSummary {
                exit_code: None,
                exit_reason: "shutdown deadline reached; force-cleared".to_owned(),
                exited_at_unix_nanos: 0,
            });
            admission.release(child_id);
        }
    }

    // Collect outcomes for all slots.
    slots
        .iter()
        .map(|(child_id, slot)| build_slot_outcome(child_id, slot))
        .collect()
}

/// Builds a shutdown outcome for one slot.
fn build_slot_outcome(child_id: &ChildId, slot: &ChildSlot) -> ChildShutdownOutcome {
    let status = if slot.has_active_attempt() {
        ChildShutdownStatus::AbortFailed
    } else if slot.abort_requested {
        ChildShutdownStatus::Aborted
    } else if slot.attempt_cancel_delivered {
        ChildShutdownStatus::Graceful
    } else {
        ChildShutdownStatus::AlreadyExited
    };
    let reason = slot
        .last_exit
        .as_ref()
        .map(|e| e.exit_reason.clone())
        .unwrap_or_else(|| "no active attempt".to_owned());
    ChildShutdownOutcome::new(ChildShutdownOutcomeInput {
        child_id: child_id.clone(),
        path: slot.path.clone(),
        generation: slot.generation.unwrap_or(Generation::initial()),
        child_start_count: slot.attempt.unwrap_or(ChildStartCount::first()),
        status,
        cancel_delivered: slot.attempt_cancel_delivered,
        exit: None,
        phase: ShutdownPhase::Completed,
        reason,
    })
}

// ---------------------------------------------------------------------------
// reconcile_shutdown_slots
// ---------------------------------------------------------------------------

/// Result of scanning all slots for orphaned handles.
#[derive(Debug, Clone)]
pub struct SlotReconcileResult {
    /// Slots that still hold active handles after shutdown.
    pub orphan_slots: Vec<ChildId>,
    /// Total number of slots checked.
    pub total_slots_checked: usize,
    /// Whether all slots are clean.
    pub verified_clean: bool,
}

/// Scans all slots after shutdown and reports any that still hold handles.
///
/// # Arguments
///
/// - `slots`: Map of child slots after shutdown.
///
/// # Returns
///
/// Returns a [`SlotReconcileResult`] listing orphaned slots.
pub fn reconcile_shutdown_slots(slots: &HashMap<ChildId, ChildSlot>) -> SlotReconcileResult {
    let mut orphan_slots: Vec<ChildId> = Vec::new();
    let total_slots_checked = slots.len();

    for (child_id, slot) in slots {
        if slot.has_active_attempt()
            || slot.cancellation_token.is_some()
            || slot.completion_receiver.is_some()
        {
            orphan_slots.push(child_id.clone());
        }
    }

    let verified_clean = orphan_slots.is_empty();
    SlotReconcileResult {
        orphan_slots,
        total_slots_checked,
        verified_clean,
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns the remaining duration until `deadline`, or `None` if already past.
fn remaining_duration(deadline: Instant) -> Option<Duration> {
    let now = Instant::now();
    if now >= deadline {
        None
    } else {
        Some(deadline - now)
    }
}

/// Waits for one slot's active attempt to complete. Returns `true` when the
/// slot finished (or had nothing to wait for).
///
/// Extracts the completion receiver from the slot before awaiting to avoid
/// borrow conflicts between `wait_for_report` and `deactivate`.
async fn drain_one_slot(
    slots: &mut HashMap<ChildId, ChildSlot>,
    child_id: &ChildId,
    remaining: Option<Duration>,
) -> bool {
    // Take the slot out of the map.
    let Some(mut slot) = slots.remove(child_id) else {
        return false;
    };
    if !slot.has_active_attempt() {
        slots.insert(child_id.clone(), slot);
        return true;
    }

    // Extract the completion receiver so we can await it without holding a
    // borrow on `slot`.
    let mut receiver = match slot.completion_receiver.take() {
        Some(rx) => rx,
        None => {
            slots.insert(child_id.clone(), slot);
            return true;
        }
    };

    // Await in a scoped block so the borrow on `receiver` is released before
    // we move it back into `slot`.
    let awaited = {
        let wait_future = crate::child_runner::runner::wait_for_report(&mut receiver);
        match remaining {
            Some(dur) => timeout(dur, wait_future).await.ok(),
            None => None,
        }
    };

    match awaited {
        Some(Ok(report)) => {
            let summary = ChildExitSummary::from_report(&report, 0u128);
            slot.deactivate(summary);
            slots.insert(child_id.clone(), slot);
            true
        }
        Some(Err(_e)) => {
            slot.deactivate(ChildExitSummary {
                exit_code: None,
                exit_reason: "completion receiver error".to_owned(),
                exited_at_unix_nanos: 0,
            });
            slots.insert(child_id.clone(), slot);
            true
        }
        None => {
            // Timeout — put receiver back and reinsert.
            slot.completion_receiver = Some(receiver);
            slots.insert(child_id.clone(), slot);
            false
        }
    }
}
