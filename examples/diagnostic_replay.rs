//! Demonstrates journal replay, metric derivation, and run summary building.

// Import typed task failure values.
use rust_supervisor::error::types::{TaskFailure, TaskFailureKind};
// Import policy and event payload values.
use rust_supervisor::event::payload::{
    // Import policy decisions.
    PolicyDecision,
    // Import state transitions.
    StateTransition,
    // Import supervisor event envelopes.
    SupervisorEvent,
    // Import event payload variants.
    What,
    // Import event location values.
    Where,
    // Finish importing event payload values.
};
// Import event timing values.
use rust_supervisor::event::time::{CorrelationId, EventSequence, EventTime, When};
// Import identifier and attempt values.
use rust_supervisor::id::types::{ChildId, ChildStartCount, Generation, SupervisorPath};
// Import fixed-capacity event journal.
use rust_supervisor::journal::ring::EventJournal;
// Import metrics facade.
use rust_supervisor::observe::metrics::MetricsFacade;
// Import child state values.
use rust_supervisor::state::child::{ChildLifecycleState, ChildState};
// Import supervisor state values.
use rust_supervisor::state::supervisor::{ShutdownState, SupervisorState};
// Import run summary builder.
use rust_supervisor::summary::builder::RunSummaryBuilder;
// Import UUID values for deterministic correlation IDs.
use uuid::Uuid;

// Run the diagnostic replay example.
/// Runs the diagnostic replay example.
fn main() {
    // Build the child identifier.
    let child_id = ChildId::new("market_feed");
    // Build the child path.
    let child_path = SupervisorPath::root().join("market_feed");
    // Build a typed task failure.
    let failure = TaskFailure::new(
        // Set the failure kind.
        TaskFailureKind::Timeout,
        // Set the low-cardinality failure category.
        "external_dependency",
        // Set the diagnostic failure message.
        "market feed heartbeat timed out",
        // Finish the typed task failure.
    );
    // Build the restart policy decision.
    let policy = PolicyDecision::new(
        // Set the decision label.
        "RestartAfter",
        // Set the restart delay.
        Some(500),
        // Set the diagnostic decision reason.
        Some("heartbeat timeout is restartable".to_owned()),
        // Finish the policy decision.
    );

    // Create the fixed-capacity event journal.
    let mut journal = EventJournal::new(8);
    // Push the running event.
    journal.push(event(
        // Set the event sequence.
        1,
        // Clone the child identifier.
        child_id.clone(),
        // Set the child display name.
        "Market Feed",
        // Build the running event payload.
        What::ChildRunning {
            // Attach the state transition.
            transition: Some(StateTransition::new("Starting", "Running")),
            // Finish the running event payload.
        },
        // Attach no policy decision.
        None,
        // Finish the running event.
    ));
    // Push the failed event.
    journal.push(event(
        // Set the event sequence.
        2,
        // Clone the child identifier.
        child_id.clone(),
        // Set the child display name.
        "Market Feed",
        // Build the failed event payload.
        What::ChildFailed {
            // Attach the typed failure.
            failure: failure.clone(),
            // Finish the failed event payload.
        },
        // Attach the restart policy decision.
        Some(policy.clone()),
        // Finish the failed event.
    ));
    // Push the backoff event.
    journal.push(event(
        // Set the event sequence.
        3,
        // Clone the child identifier.
        child_id.clone(),
        // Set the child display name.
        "Market Feed",
        // Build the backoff payload.
        What::BackoffScheduled { delay_ms: 500 },
        // Attach the restart policy decision.
        Some(policy.clone()),
        // Finish the backoff event.
    ));
    // Push the restarted event.
    journal.push(event(
        // Set the event sequence.
        4,
        // Clone the child identifier.
        child_id.clone(),
        // Set the child display name.
        "Market Feed",
        // Build the restarted payload.
        What::ChildRestarted { restart_count: 1 },
        // Attach the restart policy decision.
        Some(policy.clone()),
        // Finish the restarted event.
    ));

    // Build the final child state.
    let child = ChildState::declared(child_path, child_id, "Market Feed")
        // Set the child lifecycle state.
        .with_lifecycle_state(ChildLifecycleState::Running, EventSequence::new(1))
        // Record the latest failure.
        .record_failure(failure, EventSequence::new(2))
        // Record the latest policy decision.
        .with_policy_decision(policy, 1);
    // Build the final supervisor state.
    let final_state = SupervisorState::new(SupervisorPath::root(), EventSequence::new(4), 1)
        // Add the child state.
        .with_child(child)
        // Mark shutdown as completed for the replay.
        .with_shutdown_state(ShutdownState::Completed)
        // Attach the last journal sequence.
        .with_journal_sequence(EventSequence::new(4));
    // Build the diagnostic run summary.
    let summary = RunSummaryBuilder::new(8).build(
        // Read recent events from the journal.
        &journal,
        // Attach the final current state.
        final_state,
        // Attach the shutdown cause.
        Some("diagnostic replay complete".to_owned()),
        // Finish the summary construction.
    );
    // Create the metrics facade.
    let metrics = MetricsFacade::new();

    // Print summary counters.
    println!(
        // Provide the output template.
        "summary restart_count={} failure_count={} recent_events={}",
        // Print the restart count.
        summary.restart_count,
        // Print the failure count.
        summary.failure_count,
        // Print the recent event count.
        summary.recent_events.len(),
        // Finish summary output.
    );
    // Iterate over recent journal events.
    for event in journal.recent(8) {
        // Print event names and derived metrics.
        println!(
            // Provide the output template.
            "event={} metrics={:?}",
            // Print the event name.
            event.what.name(),
            // Print metrics derived from the event.
            metrics.samples_for_event(&event),
            // Finish event output.
        );
        // Finish the journal replay loop.
    }
    // End the diagnostic replay example.
}

// Build a deterministic supervisor event.
/// Builds one deterministic supervisor event.
fn event(
    // Receive the event sequence number.
    sequence: u64,
    // Receive the child identifier.
    child_id: ChildId,
    // Receive the child display name.
    child_name: &str,
    // Receive the event payload.
    what: What,
    // Receive the optional policy decision.
    policy: Option<PolicyDecision>,
    // Return the built supervisor event.
) -> SupervisorEvent {
    // Build the event sequence value.
    let sequence_value = EventSequence::new(sequence);
    // Build the base event envelope.
    let event = SupervisorEvent::new(
        // Build the event time wrapper.
        When::new(EventTime::deterministic(
            // Set deterministic wall-clock time.
            sequence as u128,
            // Set deterministic monotonic time.
            sequence as u128,
            // Set deterministic uptime.
            sequence,
            // Set the initial child generation.
            Generation::initial(),
            // Set the first attempt.
            ChildStartCount::first(),
            // Finish deterministic event time.
        )),
        // Build the event location.
        Where::new(SupervisorPath::root().join(&child_id.value)).with_child(child_id, child_name),
        // Attach the payload.
        what,
        // Attach the sequence.
        sequence_value,
        // Attach a deterministic correlation identifier.
        CorrelationId::from_uuid(Uuid::nil()),
        // Attach the configuration version.
        1,
        // Finish the base event envelope.
    );

    // Attach policy data when it exists.
    match policy {
        // Attach the provided policy decision.
        Some(policy) => event.with_policy(policy),
        // Return the event without policy data.
        None => event,
        // Finish policy attachment.
    }
    // Finish the deterministic event builder.
}
