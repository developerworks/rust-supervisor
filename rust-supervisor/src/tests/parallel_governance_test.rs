//! Parallel governance integration tests.
//!
//! These tests keep lead-agent supervision evidence present.

use std::fs;
use std::path::Path;

/// Verifies that parallel development ownership and review evidence exists.
#[test]
fn parallel_governance_record_exists() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let record = fs::read_to_string(root.join("artifacts/validation/documentation-ownership.md"))
        .expect("read ownership record");

    assert!(record.contains("Worker D"));
    assert!(record.contains("public API(公开接口)"));
    assert!(record.contains("验证命令"));
}
