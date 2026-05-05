#!/usr/bin/env sh
set -eu

fail() {
    printf '%s\n' "error: $1" >&2
    exit 1
}

require_file() {
    [ -f "$1" ] || fail "missing required file: $1"
}

require_file artifacts/sbom/rust-supervisor.cdx.json
require_file artifacts/sbom/rust-supervisor.spdx.json

grep -q '"bomFormat": "CycloneDX"' artifacts/sbom/rust-supervisor.cdx.json || fail "CycloneDX file has invalid shape"
grep -q '"spdxVersion": "SPDX-2.3"' artifacts/sbom/rust-supervisor.spdx.json || fail "SPDX file has invalid shape"
grep -q '"name": "rust-supervisor"' artifacts/sbom/rust-supervisor.cdx.json || fail "CycloneDX package name is missing"
grep -q '"name": "rust-supervisor-0.1.0"' artifacts/sbom/rust-supervisor.spdx.json || fail "SPDX document name is missing"
grep -q 'cargo.lock.cksum' artifacts/sbom/rust-supervisor.cdx.json || fail "Cargo.lock checksum summary is missing"
grep -q '"components": \[' artifacts/sbom/rust-supervisor.cdx.json || fail "CycloneDX components are missing"
grep -q '"name": "tokio"' artifacts/sbom/rust-supervisor.cdx.json || fail "CycloneDX missing tokio dependency"
grep -q '"name": "serde"' artifacts/sbom/rust-supervisor.cdx.json || fail "CycloneDX missing serde dependency"
grep -q '"name": "rust-config-tree"' artifacts/sbom/rust-supervisor.cdx.json || fail "CycloneDX missing rust-config-tree dependency"
grep -q '"name": "tokio"' artifacts/sbom/rust-supervisor.spdx.json || fail "SPDX missing tokio dependency"
grep -q '"name": "serde"' artifacts/sbom/rust-supervisor.spdx.json || fail "SPDX missing serde dependency"
grep -q '"name": "rust-config-tree"' artifacts/sbom/rust-supervisor.spdx.json || fail "SPDX missing rust-config-tree dependency"

if grep -R -n -E 'secret|token|/Users/|/tmp/|target/' artifacts/sbom; then
    fail "SBOM contains forbidden sensitive or local path text"
fi

printf '%s\n' "SBOM validation passed"
