#!/usr/bin/env bash
# verify-attestation.sh — Verify supply chain attestation against release record.
#
# Reads supply_chain_attestation.path and sha256 from the release record JSON.
# Recomputes hashes of all listed artifacts and compares item by item.
# Outputs MATCH/MISMATCH per artifact.
#
# Usage: bash scripts/verify-attestation.sh <release-record.json>

set -euo pipefail

RECORD="${1:-artifacts/release-record.json}"
FAILED=0

if [ ! -f "$RECORD" ]; then
    echo "MISMATCH: release record not found at $RECORD"
    exit 1
fi

ATTEST_PATH=$(jq -r '.supply_chain_attestation.path' "$RECORD")
ATTEST_EXPECTED_HASH=$(jq -r '.supply_chain_attestation.sha256' "$RECORD")

if [ "$ATTEST_PATH" = "null" ] || [ ! -f "$ATTEST_PATH" ]; then
    echo "MISMATCH: attestation file not found at $ATTEST_PATH"
    exit 1
fi

# Verify attestation file hash
ACTUAL_ATTEST_HASH=$(sha256sum "$ATTEST_PATH" | awk '{print $1}')
if [ "$ACTUAL_ATTEST_HASH" = "$ATTEST_EXPECTED_HASH" ]; then
    echo "MATCH: $ATTEST_PATH (sha256: $ACTUAL_ATTEST_HASH)"
else
    echo "MISMATCH: $ATTEST_PATH"
    echo "  Expected: $ATTEST_EXPECTED_HASH"
    echo "  Actual:   $ACTUAL_ATTEST_HASH"
    FAILED=1
fi

# Verify each artifact listed in attestation
echo ""
echo "Verifying individual artifacts..."
for artifact in $(jq -r '.artifacts | keys[]' "$ATTEST_PATH"); do
    ARTIFACT_PATH=$(jq -r ".artifacts.${artifact}.path" "$ATTEST_PATH")
    ARTIFACT_EXPECTED_HASH=$(jq -r ".artifacts.${artifact}.sha256" "$ATTEST_PATH")

    if [ "$ARTIFACT_PATH" = "null" ] || [ "$ARTIFACT_EXPECTED_HASH" = "null" ]; then
        echo "  SKIP: $artifact (path or hash missing in attestation)"
        continue
    fi

    if [ ! -f "$ARTIFACT_PATH" ]; then
        echo "  MISMATCH: $artifact — file not found at $ARTIFACT_PATH"
        FAILED=1
        continue
    fi

    ACTUAL_ARTIFACT_HASH=$(sha256sum "$ARTIFACT_PATH" | awk '{print $1}')
    if [ "$ACTUAL_ARTIFACT_HASH" = "$ARTIFACT_EXPECTED_HASH" ]; then
        echo "  MATCH: $artifact ($ARTIFACT_PATH)"
    else
        echo "  MISMATCH: $artifact ($ARTIFACT_PATH)"
        echo "    Expected: $ARTIFACT_EXPECTED_HASH"
        echo "    Actual:   $ACTUAL_ARTIFACT_HASH"
        FAILED=1
    fi
done

if [ "$FAILED" -eq 0 ]; then
    echo ""
    echo "All attestation artifacts verified."
    exit 0
else
    echo ""
    echo "One or more attestation artifacts mismatched."
    exit 1
fi
