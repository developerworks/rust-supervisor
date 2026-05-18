#!/usr/bin/env bash
# release-check.sh — Run all shallow and middle release gates.
#
# Prints each gate_id and passed/failed status. Exits non-zero if any
# gate fails.
#
# Usage: bash scripts/release-check.sh

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

FAILED=0
GATE_OUTCOME=""

run_gate() {
    local gate_id="$1"
    shift
    echo -n "  [$gate_id] "
    if "$@" > /tmp/gate_${gate_id}.log 2>&1; then
        echo -e "${GREEN}passed${NC}"
        GATE_OUTCOME="${GATE_OUTCOME}${gate_id},shallow,,passed,,,,/tmp/gate_${gate_id}.log,$(date -u +%Y-%m-%dT%H:%M:%SZ)\n"
    else
        echo -e "${RED}failed${NC}"
        GATE_OUTCOME="${GATE_OUTCOME}${gate_id},shallow,,failed,,,,/tmp/gate_${gate_id}.log,$(date -u +%Y-%m-%dT%H:%M:%SZ)\n"
        FAILED=1
    fi
}

echo "=== Shallow Gates ==="
run_gate "fmt"              cargo fmt --check
run_gate "check"            cargo check --all-targets
run_gate "clippy"           cargo clippy --all-targets -- -D warnings
run_gate "test"             cargo test
run_gate "doc"              cargo doc --no-deps --document-private-items
run_gate "publish_dry_run"  cargo publish --dry-run

# ---- Middle Gates ----
echo ""
echo "=== Middle Gates ==="
run_gate "dependency_audit" cargo audit --deny warnings
run_gate "license_check"     cargo deny check licenses
run_gate "advisory_check"    cargo deny check advisories
run_gate "semver_checks"     cargo semver-checks
run_gate "msrv_verify"       bash scripts/verify-msrv.sh

# ---- Deep Gates Summary ----
echo ""
echo "=== Deep Gates Summary ==="
CSV="artifacts/quality-gate-outcome.csv"
if [ -f "$CSV" ]; then
    echo "  Checking $CSV for deep-tier rows..."
    bash scripts/fill-quality-gate.sh --verify
else
    echo "  WARNING: $CSV not found — deep quality gate outcomes unavailable."
    echo "  Run nightly gates or provide an exemption ticket for each deep gate."
fi

echo ""
if [ "$FAILED" -eq 0 ]; then
    echo -e "${GREEN}All gates passed.${NC}"
else
    echo -e "${RED}One or more gates failed.${NC}"
fi
exit $FAILED
