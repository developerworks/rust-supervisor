//! Demonstrates the five WorkRole profiles and their effect on restart policy
//! decisions and escalation paths.
//!
//! Each role (Service / Worker / Job / Sidecar / Supervisor) carries built-in
//! defaults for on-success action, on-failure action, budget, and escalation.
//! This example resolves the effective policy for each role and prints the
//! resulting default pack so learners can see how role classification drives
//! policy selection.

use rust_supervisor::policy::role_defaults::{
    EffectivePolicy, OnFailureAction, OnManualStopAction, OnSuccessAction, PolicySource,
    RoleDefaultPolicy, WorkRole,
};

/// Runs the work role demonstration.
fn main() {
    println!("=== WorkRole Defaults Demo ===");
    println!();

    // Iterate over every defined work role.
    for role in [
        WorkRole::Service,
        WorkRole::Worker,
        WorkRole::Job,
        WorkRole::Sidecar,
        WorkRole::Supervisor,
    ] {
        // Resolve the default policy pack for this role.
        let pack = RoleDefaultPolicy::for_role(role);

        println!("--- role={} ---", role.as_str());
        println!(
            "  on_success_exit       = {}",
            label_on_success(pack.on_success_exit)
        );
        println!(
            "  on_failure_exit       = {}",
            label_on_failure(pack.on_failure_exit)
        );
        println!(
            "  on_manual_stop        = {}",
            label_manual_stop(pack.on_manual_stop)
        );
        println!("  on_timeout            = {:?}", pack.on_timeout);
        println!("  default_restart_limit = {:?}", pack.default_restart_limit);
        println!(
            "  default_escalation    = {:?}",
            pack.default_escalation_policy
        );
        println!(
            "  default_backoff       = {:?}",
            pack.default_backoff_policy
        );
        println!();
    }

    // Demonstrate EffectivePolicy::merge for each role.
    println!("=== EffectivePolicy (with merge) ===");
    println!();

    for role in [
        WorkRole::Service,
        WorkRole::Worker,
        WorkRole::Job,
        WorkRole::Sidecar,
        WorkRole::Supervisor,
    ] {
        let effective = EffectivePolicy::merge(Some(role), vec![]);
        println!(
            "role={:12} source={:16} severity={:?} used_fallback={}",
            effective.work_role.as_str(),
            label_source(effective.source),
            effective.severity,
            effective.used_fallback,
        );
    }

    // Show what happens when no role is declared (fallback to Worker).
    println!();
    println!("=== Fallback (no role declared) ===");
    println!();

    let fallback = EffectivePolicy::merge(None, vec![]);
    println!("work_role  = {}", fallback.work_role.as_str());
    println!("source     = {}", label_source(fallback.source));
    println!("used_fallback = {}", fallback.used_fallback);
    println!(
        "on_failure = {}",
        label_on_failure(fallback.policy_pack.on_failure_exit)
    );
}

/// Returns a human-readable label for an OnSuccessAction.
fn label_on_success(action: OnSuccessAction) -> &'static str {
    match action {
        OnSuccessAction::Restart => "restart",
        OnSuccessAction::Stop => "stop",
        OnSuccessAction::NoOp => "no-op",
    }
}

/// Returns a human-readable label for an OnFailureAction.
fn label_on_failure(action: OnFailureAction) -> &'static str {
    match action {
        OnFailureAction::RestartWithBackoff => "restart with backoff",
        OnFailureAction::RestartPermanent => "restart permanent",
        OnFailureAction::StopAndEscalate => "stop and escalate",
    }
}

/// Returns a human-readable label for an OnManualStopAction.
fn label_manual_stop(action: OnManualStopAction) -> &'static str {
    match action {
        OnManualStopAction::StopForever => "stop forever",
        OnManualStopAction::StopUntilExplicitRestart => "stop until explicit restart",
    }
}

/// Returns a human-readable label for the PolicySource.
fn label_source(source: PolicySource) -> &'static str {
    match source {
        PolicySource::RoleDefault => "role_default",
        PolicySource::UserOverride => "user_override",
        PolicySource::FallbackDefault => "fallback_default",
    }
}
