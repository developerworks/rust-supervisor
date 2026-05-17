#!/usr/bin/env bash
# verify-sbom.sh — Verify that the SBOM file hash matches the release record.
#
# Reads sbom.path, sbom.sha256, and sbom.format_version from the release
# record JSON. Computes the actual SHA256 of the SBOM file and compares.
# If format_version differs from the current tool output, prints a warning
# and suggests consulting sbom-migration.md.
#
# Usage: bash scripts/verify-sbom.sh <release-record.json>

set -euo pipefail

RECORD="${1:-artifacts/release-record.json}"

if [ ! -f "$RECORD" ]; then
    echo "MISMATCH: release record not found at $RECORD"
    exit 1
fi

SBOM_PATH=$(jq -r '.sbom.path' "$RECORD")
SBOM_EXPECTED_HASH=$(jq -r '.sbom.sha256' "$RECORD")
SBOM_FORMAT_VERSION=$(jq -r '.sbom.format_version' "$RECORD")

if [ "$SBOM_PATH" = "null" ] || [ "$SBOM_EXPECTED_HASH" = "null" ]; then
    echo "MISMATCH: sbom.path or sbom.sha256 missing in $RECORD"
    exit 1
fi

if [ ! -f "$SBOM_PATH" ]; then
    echo "MISMATCH: SBOM file not found at $SBOM_PATH"
    exit 1
fi

# Compute actual hash
ACTUAL_HASH=$(sha256sum "$SBOM_PATH" | awk '{print $1}')

# Check format version (print warning if the record version != expected)
# For now, we only check that the field is present.
if [ "$SBOM_FORMAT_VERSION" != "null" ] && [ -n "$SBOM_FORMAT_VERSION" ]; then
    echo "  SBOM format version (from record): $SBOM_FORMAT_VERSION"
    # If sbom-migration.md exists, print a hint
    SBOM_DIR=$(dirname "$SBOM_PATH")
    if [ -f "$SBOM_DIR/sbom-migration.md" ]; then
        echo "  NOTE: sbom-migration.md found — review schema changes."
    fi
fi

if [ "$ACTUAL_HASH" = "$SBOM_EXPECTED_HASH" ]; then
    echo "MATCH: $SBOM_PATH (sha256: $ACTUAL_HASH)"
    exit 0
else
    echo "MISMATCH: $SBOM_PATH"
    echo "  Expected: $SBOM_EXPECTED_HASH"
    echo "  Actual:   $ACTUAL_HASH"
    exit 1
fi
