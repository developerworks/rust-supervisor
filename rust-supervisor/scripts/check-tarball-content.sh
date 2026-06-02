#!/usr/bin/env bash
# Checks that `cargo package --list` output meets the DeliveryBundle requirements
# from specs/006-8-product-bundle-runbooks/spec.md FR-001.
set -euo pipefail

PASS=true

echo "=== Tarball Content Check ==="

# 1. Capture package listing
PACKAGE_LIST=$(cargo package --list --allow-dirty 2>/dev/null) || {
    echo "FAIL: cargo package --list failed"
    exit 1
}

# 2. Check src/ exists and is non-empty
SRC_COUNT=$(echo "$PACKAGE_LIST" | grep -c "^src/" || true)
if [ "$SRC_COUNT" -gt 0 ]; then
    echo "PASS: src/ directory found ($SRC_COUNT files)"
else
    echo "FAIL: src/ directory missing or empty"
    PASS=false
fi

# 3. Check examples/ exists and is non-empty
EXAMPLES_COUNT=$(echo "$PACKAGE_LIST" | grep -c "^examples/" || true)
if [ "$EXAMPLES_COUNT" -gt 0 ]; then
    echo "PASS: examples/ directory found ($EXAMPLES_COUNT files)"
else
    echo "FAIL: examples/ directory missing or empty"
    PASS=false
fi

# 4. Check manual/ exists and is non-empty
MANUAL_COUNT=$(echo "$PACKAGE_LIST" | grep -c "^manual/" || true)
if [ "$MANUAL_COUNT" -gt 0 ]; then
    echo "PASS: manual/ directory found ($MANUAL_COUNT files)"
else
    echo "FAIL: manual/ directory missing or empty"
    PASS=false
fi

# 5. Check Cargo.toml is included
if echo "$PACKAGE_LIST" | grep -q "^Cargo.toml$"; then
    echo "PASS: Cargo.toml included"
else
    echo "FAIL: Cargo.toml not in package"
    PASS=false
fi

# 6. Check no private registry references in Cargo.toml
if grep -n 'registry\s*=\s*"' Cargo.toml 2>/dev/null | grep -v 'crates.io' | grep -v 'rust-lang.org'; then
    echo "FAIL: Non-public registry reference found in Cargo.toml"
    PASS=false
else
    echo "PASS: No private registry references"
fi

# 7. Check no absolute path dependencies
if grep -n 'path\s*=\s*"/' Cargo.toml 2>/dev/null; then
    echo "FAIL: Absolute path dependency found in Cargo.toml"
    PASS=false
else
    echo "PASS: No absolute path dependencies"
fi

echo "---"
if [ "$PASS" = true ]; then
    echo "Result: PASS"
    exit 0
else
    echo "Result: FAIL"
    exit 1
fi
