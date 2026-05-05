//! Foundational model integration tests.
//!
//! These tests cover the identifier and declaration primitives used elsewhere.

use rust_supervisor::id::types::{Attempt, ChildId, Generation, SupervisorPath};
use rust_supervisor::spec::supervisor::SupervisorSpec;

/// Verifies stable identifier and path behavior.
#[test]
fn identifiers_and_paths_are_stable() {
    let child_id = ChildId::new("worker");
    let path = SupervisorPath::root().join(child_id.to_string());

    assert_eq!(path.to_string(), "/worker");
    assert_eq!(path.parent().expect("parent").to_string(), "/");
    assert_eq!(Attempt::first().next().value, 2);
    assert_eq!(Generation::initial().next().value, 1);
}

/// Verifies that an empty root supervisor declaration is valid.
#[test]
fn root_supervisor_spec_validates() {
    let spec = SupervisorSpec::root(Vec::new());

    assert!(spec.validate().is_ok());
    assert_eq!(spec.path.to_string(), "/");
}
