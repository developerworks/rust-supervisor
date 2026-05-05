//! Documentation synchronization integration tests.
//!
//! These tests keep README and user docs aligned with current examples.

use std::fs;
use std::path::Path;

/// Verifies that public docs mention the active configuration and examples.
#[test]
fn docs_reference_current_config_and_examples() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let docs = [
        "README.md",
        "README.zh.md",
        "manual/zh/index.md",
        "manual/en/index.md",
        "docs/zh/index.md",
        "docs/en/index.md",
    ]
    .into_iter()
    .map(|path| fs::read_to_string(root.join(path)).expect("read doc"))
    .collect::<Vec<_>>()
    .join("\n");

    assert!(docs.contains("rust-config-tree(集中配置树) v0.1.9"));
    assert!(docs.contains("supervisor_quickstart"));
    assert!(docs.contains("Shutdown Without Orphaned Tasks(关闭后不留下孤儿任务)"));
}
