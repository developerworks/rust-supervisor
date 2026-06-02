//! SBOM integration tests.
//!
//! These tests verify the generated software bill of materials artifact shape.

use std::fs;
use std::path::Path;

/// Verifies that CycloneDX and SPDX SBOM artifacts exist.
#[test]
fn sbom_artifacts_have_expected_shape() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cdx =
        fs::read_to_string(root.join("artifacts/sbom/rust-supervisor.cdx.json")).expect("read cdx");
    let spdx = fs::read_to_string(root.join("artifacts/sbom/rust-supervisor.spdx.json"))
        .expect("read spdx");

    assert!(cdx.contains("\"bomFormat\": \"CycloneDX\""));
    assert!(spdx.contains("\"spdxVersion\": \"SPDX-2.3\""));
    assert!(cdx.contains("cargo.lock.cksum"));
}

/// Verifies that SBOM artifacts include locked crate dependencies.
#[test]
fn sbom_artifacts_include_locked_crates() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cdx =
        fs::read_to_string(root.join("artifacts/sbom/rust-supervisor.cdx.json")).expect("read cdx");
    let spdx = fs::read_to_string(root.join("artifacts/sbom/rust-supervisor.spdx.json"))
        .expect("read spdx");
    let cdx_json: serde_json::Value = serde_json::from_str(&cdx).expect("parse cdx");
    let spdx_json: serde_json::Value = serde_json::from_str(&spdx).expect("parse spdx");
    let cdx_components = cdx_json["components"].as_array().expect("cdx components");
    let spdx_packages = spdx_json["packages"].as_array().expect("spdx packages");

    assert!(cdx_components.len() > 10);
    assert!(spdx_packages.len() > 10);

    for crate_name in ["tokio", "serde", "rust-config-tree", "tracing", "uuid"] {
        assert!(
            cdx_components
                .iter()
                .any(|component| component["name"] == crate_name)
        );
        assert!(
            spdx_packages
                .iter()
                .any(|package| package["name"] == crate_name)
        );
    }
}
