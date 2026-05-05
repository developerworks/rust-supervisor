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
