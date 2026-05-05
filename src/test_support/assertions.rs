//! Test assertions for supervisor diagnostics.
//!
//! The functions in this module are intentionally small and panic with direct
//! messages so failing tests point at missing observability facts.

use crate::event::payload::SupervisorEvent;
use crate::journal::ring::EventJournal;
use crate::observe::pipeline::TestRecorder;
use crate::state::child::ChildLifecycleState;
use crate::state::supervisor::SupervisorState;
use crate::summary::builder::RunSummary;

/// Asserts that events were emitted in strict sequence order.
///
/// # Arguments
///
/// - `events`: Events to inspect.
///
/// # Returns
///
/// This function does not return a value.
///
/// # Examples
///
/// ```
/// let events: Vec<rust_supervisor::event::payload::SupervisorEvent> = Vec::new();
/// rust_supervisor::test_support::assertions::assert_event_sequences_increase(&events);
/// ```
pub fn assert_event_sequences_increase(events: &[SupervisorEvent]) {
    for pair in events.windows(2) {
        assert!(
            pair[0].sequence.value < pair[1].sequence.value,
            "event sequence must increase"
        );
    }
}

/// Asserts that a journal retained a specific number of events.
///
/// # Arguments
///
/// - `journal`: Journal to inspect.
/// - `expected`: Expected retained event count.
///
/// # Returns
///
/// This function does not return a value.
pub fn assert_journal_len(journal: &EventJournal, expected: usize) {
    assert_eq!(journal.len(), expected, "unexpected event journal length");
}

/// Asserts that a run summary contains at least one recent event.
///
/// # Arguments
///
/// - `summary`: Summary to inspect.
///
/// # Returns
///
/// This function does not return a value.
pub fn assert_summary_has_recent_events(summary: &RunSummary) {
    assert!(
        !summary.recent_events.is_empty(),
        "run summary must include recent events"
    );
}

/// Asserts that a child path has the expected lifecycle state.
///
/// # Arguments
///
/// - `state`: Supervisor state to inspect.
/// - `path`: Child path text.
/// - `expected`: Expected lifecycle state.
///
/// # Returns
///
/// This function does not return a value.
pub fn assert_child_state(state: &SupervisorState, path: &str, expected: ChildLifecycleState) {
    let child = state
        .children
        .get(path)
        .unwrap_or_else(|| panic!("missing child state for {path}"));
    assert_eq!(child.state, expected, "unexpected child state");
}

/// Asserts that shutdown left no running children.
///
/// # Arguments
///
/// - `state`: Final supervisor state to inspect.
///
/// # Returns
///
/// This function does not return a value.
pub fn assert_shutdown_without_orphaned_tasks(state: &SupervisorState) {
    let running = state
        .children
        .values()
        .filter(|child| matches!(child.state, ChildLifecycleState::Running))
        .count();
    assert_eq!(running, 0, "shutdown left running children");
}

/// Asserts that a test recorder saw at least one metric sample.
///
/// # Arguments
///
/// - `recorder`: Observability test recorder.
///
/// # Returns
///
/// This function does not return a value.
pub fn assert_recorder_has_metrics(recorder: &TestRecorder) {
    assert!(
        !recorder.metrics.is_empty(),
        "observability recorder must contain metrics"
    );
}
