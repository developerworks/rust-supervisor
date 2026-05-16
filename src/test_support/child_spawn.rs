//! Debug-only integration test hooks for deterministic child spawn failures.

use crate::id::types::ChildId;

#[cfg(debug_assertions)]
use std::sync::Mutex;

/// Shared payload describing which child identifier should simulate the next deterministic spawn failures.
#[cfg(debug_assertions)]
struct SpawnFailureHookState {
    /// Remaining synchronous spawn failures triggered by [`take_child_spawn_failure_attempt`].
    remaining: usize,
    /// Stable child targeted by deterministic spawn failures from tests.
    child_id: ChildId,
}

/// Mutex protecting deterministic spawn instrumentation when multiple tests run concurrently.
#[cfg(debug_assertions)]
static SPAWN_FAILURE_HOOK: Mutex<Option<SpawnFailureHookState>> = Mutex::new(None);

/// Arms synchronous spawn failures for upcoming [`ChildRunner::spawn_once`](crate::child_runner::runner::ChildRunner::spawn_once)
/// registrations that match `child_id`.
///
/// # Arguments
///
/// - `child_id`: Child identifier whose spawned work should surface the hook first.
/// - `count`: Number of matching spawn attempts that must fail before unsetting the hook.
///
/// # Returns
///
/// This function does not return a value.
///
/// # Notes
///
/// The hook scopes failures to [`ChildId`](crate::id::types::ChildId) because `cargo test` runs test targets
/// in parallel and an unscoped global spawn counter races with unrelated supervisors issuing `spawn_once`
/// concurrently.
///
/// The hook is compiled out of release builds (`not(debug_assertions)`), so production binaries never consult it.
pub fn fail_next_child_spawns_for(child_id: ChildId, count: usize) {
    #[cfg(debug_assertions)]
    {
        let mut guard = SPAWN_FAILURE_HOOK
            .lock()
            .expect("spawn hook mutex should remain valid for debug integration tests");
        *guard = Some(SpawnFailureHookState {
            remaining: count,
            child_id,
        });
    }
    #[cfg(not(debug_assertions))]
    {
        let _ = child_id;
        let _ = count;
    }
}

/// Returns whether the incoming [`ChildRunner::spawn_once`](crate::child_runner::runner::ChildRunner::spawn_once) call must fail fast for integration tests targeting `child_id`.
///
/// # Arguments
///
/// - `child_id`: Stable child issuing the synchronous spawn invitation.
///
/// # Returns
///
/// Returns `true` when instrumentation decides the deterministic failure hook covers `child_id`.
pub(crate) fn take_child_spawn_failure_attempt(child_id: &ChildId) -> bool {
    #[cfg(debug_assertions)]
    {
        let Ok(mut guard) = SPAWN_FAILURE_HOOK.lock() else {
            return false;
        };
        let Some(mut state) = guard.take() else {
            return false;
        };
        if state.child_id != *child_id {
            *guard = Some(state);
            return false;
        }
        if state.remaining == 0 {
            return false;
        }
        state.remaining = state.remaining.saturating_sub(1);
        if state.remaining > 0 {
            *guard = Some(state);
        }
        true
    }
    #[cfg(not(debug_assertions))]
    {
        let _ = child_id;
        false
    }
}
