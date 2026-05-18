#!/usr/bin/env bash
# verify-msrv.sh — Verify that the crate compiles with its declared MSRV.
#
# Steps:
#   1. Extract rust-version from Cargo.toml
#   2. Check if the corresponding rustc toolchain is installed via rustup
#   3. If missing, install it via rustup
#   4. Run cargo check with that specific toolchain
#   5. Print result and exit with appropriate status
#
# SC-004: Failure must exit in <=5 steps and print a manual section reference.
#
# Usage: bash scripts/verify-msrv.sh [manual-section-ref]

set -euo pipefail

MANUAL_REF="${1:-docs/zh/quickstart.md#msrv}"

# Step 1: Extract MSRV
MSRV=$(grep 'rust-version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
if [ -z "$MSRV" ]; then
    echo "ERROR: rust-version not found in Cargo.toml"
    echo "See: $MANUAL_REF"
    exit 1
fi
echo "Step 1: MSRV is $MSRV (from Cargo.toml)"

# Step 2: Check if toolchain is installed
if rustup toolchain list 2>/dev/null | grep -q "$MSRV"; then
    echo "Step 2: toolchain $MSRV already installed"
else
    echo "Step 2: toolchain $MSRV not found"
    # Step 3: Install
    echo "Step 3: installing toolchain $MSRV via rustup"
    rustup toolchain install "$MSRV"
fi

# Step 4: Compile with MSRV toolchain
echo "Step 4: running cargo +$MSRV check --all-targets"
if cargo +"$MSRV" check --all-targets 2>&1; then
    # Step 5: Success
    echo "Step 5: MSRV OK — crate compiles with Rust $MSRV"
    exit 0
else
    # Step 5: Failure
    echo "Step 5: MSRV FAIL — compilation failed with Rust $MSRV"
    echo "Upgrade your rustc to at least $MSRV. See: $MANUAL_REF"
    exit 1
fi
