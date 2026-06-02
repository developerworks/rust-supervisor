//! Role macro compile-fail integration tests.
//!
//! This test harness verifies that malformed role contracts fail at compile
//! time with a targeted macro diagnostic.

/// Verifies that roles without required lifecycle items are rejected.
#[test]
fn role_macros_reject_missing_required_items() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/role_contracts/ui/service_missing_run.rs");
    tests.compile_fail("tests/role_contracts/ui/worker_missing_work.rs");
    tests.compile_fail("tests/role_contracts/ui/job_missing_run.rs");
    tests.compile_fail("tests/role_contracts/ui/sidecar_missing_run.rs");
    tests.compile_fail("tests/role_contracts/ui/sidecar_missing_primary.rs");
    tests.compile_fail("tests/role_contracts/ui/supervisor_missing_build_tree.rs");
}
