//! Demonstrates group isolation boundaries and cross-group dependency edges.
//!
//! Groups let operators isolate fault boundaries so a meltdown in one group
//! does not cascade into another. Dependency edges control propagation
//! semantics: Full (cascade to dependent), EscalateOnly (notify only), or
//! None (fully independent).
//!
//! This example declares two groups (risk-engine, payment-gateway), connects
//! them with a dependency edge, and uses the isolation policy to check
//! whether a failure in one group affects the other.

use rust_supervisor::policy::group::{
    GroupDependencyEdge, GroupIsolationPolicy, PropagationPolicy,
};

/// Runs the group isolation demonstration.
fn main() {
    println!("=== Group Isolation Demo ===");
    println!();

    // Define the sample group names.
    let payment_group = "payment-gateway";
    let risk_group = "risk-engine";

    // Build three dependency edges with different propagation modes.
    let edge_full = GroupDependencyEdge {
        from_group: payment_group.to_owned(),
        to_group: risk_group.to_owned(),
        propagation: PropagationPolicy::Full,
    };

    // Build the escalate-only dependency edge.
    let edge_escalate = GroupDependencyEdge {
        from_group: payment_group.to_owned(),
        to_group: risk_group.to_owned(),
        propagation: PropagationPolicy::EscalateOnly,
    };

    // Build the independent dependency edge.
    let edge_none = GroupDependencyEdge {
        from_group: payment_group.to_owned(),
        to_group: risk_group.to_owned(),
        propagation: PropagationPolicy::None,
    };

    // Print the declared group topology.
    println!("Groups declared:");
    println!("  [1] {risk_group}  - handles risk scoring");
    println!("  [2] {payment_group} - handles payment processing");
    println!();
    println!("Dependency: {payment_group} depends on {risk_group}");
    println!();

    // Evaluate propagation for each edge type.
    println!("=== Propagation Evaluation ===");
    println!();

    // Evaluate each propagation mode.
    for (label, edge) in [
        ("Full", &edge_full),
        ("EscalateOnly", &edge_escalate),
        ("None", &edge_none),
    ] {
        let iso = GroupIsolationPolicy::new(vec![edge.clone()]);
        let affected = iso.affected_by(payment_group, risk_group);

        // Print the propagation evaluation for this edge.
        println!("--- PropagationPolicy::{label} ---");
        println!("  from_group = {}", edge.from_group);
        println!("  to_group   = {}", edge.to_group);
        println!("  propagation = {label}");
        println!("  {payment_group} affected by {risk_group} failure = {affected}");
        println!();
    }

    // Demonstrate same-group vs cross-group semantics.
    println!("=== Same-Group vs Cross-Group ===");
    println!();

    // Build a policy for same-group and cross-group checks.
    let iso = GroupIsolationPolicy::new(vec![edge_full]);

    // Check same-group propagation.
    let same = iso.affected_by(risk_group, risk_group);
    println!("  {risk_group} affected by {risk_group} (same group)     = {same}");

    // Check cross-group propagation.
    let cross = iso.affected_by(payment_group, risk_group);
    println!("  {payment_group} affected by {risk_group} (Full edge)   = {cross}");

    // Check an unrelated group.
    let unrelated = iso.affected_by(payment_group, "monitoring");
    println!("  {payment_group} affected by 'monitoring' (no edge) = {unrelated}");

    // Print the group isolation summary.
    println!();
    println!("=== Summary ===");
    println!("GroupIsolationPolicy::affected_by checks a directed DAG of dependency edges.");
    println!("PropagationPolicy::Full         -> isolation cascades to dependents.");
    println!("PropagationPolicy::EscalateOnly -> no local isolation, parent notified.");
    println!("PropagationPolicy::None         -> groups are fully independent.");
    println!("Same-group failures always affect the group itself.");
}
