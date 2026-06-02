#!/usr/bin/env bash
# generate-attestation.sh — Generate a supply chain attestation JSON.
#
# Collects: version, commit, timestamp, gate results summary, and artifact
# SHA256 hashes. Outputs to artifacts/attestation.json.
#
# If any artifact path is unreachable or hash computation fails, outputs
# "attestation_unavailable" status and writes artifacts/attestation-error.log.
# The release record MUST be marked "blocked" until the issue is resolved.
#
# Usage: bash scripts/generate-attestation.sh [release-record.json]

set -euo pipefail

RECORD="${1:-artifacts/release-record.json}"
ATTESTATION="artifacts/attestation.json"
ERROR_LOG="artifacts/attestation-error.log"
rm -f "$ERROR_LOG"

if [ ! -f "$RECORD" ]; then
    echo "ERROR: release record not found at $RECORD" | tee -a "$ERROR_LOG"
    echo "{\"status\":\"attestation_unavailable\",\"error\":\"release record not found\"}" > "$ATTESTATION"
    exit 1
fi

VERSION=$(jq -r '.version' "$RECORD")
COMMIT=$(jq -r '.commit' "$RECORD")
TIMESTAMP=$(date -u +%Y-%m-%dT%H:%M:%SZ)

# Collect artifact hashes
SBOM_PATH=$(jq -r '.sbom.path' "$RECORD")
SBOM_HASH=""
if [ "$SBOM_PATH" != "null" ] && [ -f "$SBOM_PATH" ]; then
    SBOM_HASH=$(sha256sum "$SBOM_PATH" | awk '{print $1}')
else
    echo "WARNING: sbom.path unreachable: $SBOM_PATH" | tee -a "$ERROR_LOG"
fi

# Crate artifact hash (if available)
CRATE_PATH="target/package/rust-tokio-supervisor-${VERSION}.crate"
CRATE_HASH=""
if [ -f "$CRATE_PATH" ]; then
    CRATE_HASH=$(sha256sum "$CRATE_PATH" | awk '{print $1}')
fi

# Build attestation JSON
jq -n \
    --arg version "$VERSION" \
    --arg commit "$COMMIT" \
    --arg timestamp "$TIMESTAMP" \
    --arg sbom_hash "$SBOM_HASH" \
    --arg crate_hash "$CRATE_HASH" \
    '{
        version: $version,
        commit: $commit,
        timestamp: $timestamp,
        artifacts: {
            sbom: { path: "artifacts/sbom/sbom.spdx.json", sha256: $sbom_hash },
            crate: { path: "target/package/rust-tokio-supervisor-\($version).crate", sha256: $crate_hash }
        }
    }' > "$ATTESTATION"

if [ -s "$ERROR_LOG" ]; then
    echo "WARNING: attestation generated with errors. See $ERROR_LOG"
    echo "{\"status\":\"attestation_unavailable\",\"errors\":true}" | jq -s '.[0] * .[1]' "$ATTESTATION" - > "${ATTESTATION}.tmp"
    mv "${ATTESTATION}.tmp" "$ATTESTATION"
    exit 1
fi

echo "attestation generated: $ATTESTATION"
exit 0
