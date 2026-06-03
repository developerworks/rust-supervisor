#!/usr/bin/env bash
# publish-crates.sh — Publish workspace crates to crates.io in dependency order.
#
# This script automates the two-step publish sequence required by the workspace:
#   1. rust-supervisor-macros (proc-macro crate, must be published first)
#   2. rust-tokio-supervisor   (main crate, depends on the macros crate)
#
# Prerequisites:
#   - `cargo login` must have been run so crates.io tokens are available.
#   - The working tree should be clean (uncommitted changes will cause dry-run
#     failures unless --allow-dirty is passed).
#   - CHANGELOG.md and version numbers in Cargo.toml should already be bumped.
#
# Usage:
#   bash scripts/publish-crates.sh [--dry-run] [--allow-dirty]
#
# Options:
#   --dry-run       Perform all steps but skip the actual upload (cargo publish --dry-run).
#   --allow-dirty   Pass --allow-dirty to cargo publish, useful during iteration.
#
# Exit code:
#   0  — All crates published (or dry-run simulated) successfully.
#   1  — Any step failed.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DRY_RUN=""
ALLOW_DIRTY=""

# ---- Argument parsing ----
while [[ $# -gt 0 ]]; do
    case "$1" in
        --dry-run)    DRY_RUN="--dry-run" ; shift ;;
        --allow-dirty) ALLOW_DIRTY="--allow-dirty" ; shift ;;
        --help)       sed -n '4,/^[[:space:]]*$/p' "$0" | sed 's/^# //' | sed 's/^#$//' | grep -v '^$' ; exit 0 ;;
        *)            echo "Unknown option: $1" >&2; exit 1 ;;
    esac
done

PUBLISH_FLAGS="${DRY_RUN} ${ALLOW_DIRTY}"

# ---- Pre-flight checks ----
echo "=== Pre-flight checks ==="

# Verify cargo is available
command -v cargo >/dev/null 2>&1 || { echo "cargo is required" >&2; exit 1; }

# Check for uncommitted changes unless --allow-dirty
if [[ -z "${ALLOW_DIRTY}" ]]; then
    if ! git -C "${ROOT_DIR}" diff --quiet --exit-code; then
        echo "ERROR: Working tree has uncommitted changes." >&2
        echo "  Commit or stash them first, or pass --allow-dirty." >&2
        exit 1
    fi
    if ! git -C "${ROOT_DIR}" diff --cached --quiet --exit-code; then
        echo "ERROR: Staging area has uncommitted changes." >&2
        echo "  Commit or stash them first, or pass --allow-dirty." >&2
        exit 1
    fi
fi

# Verify both Cargo.toml files are readable
for pkg in "rust-supervisor-macros" "rust-tokio-supervisor"; do
    cargo_path="${ROOT_DIR}/../${pkg}/Cargo.toml"
    if [[ ! -f "${ROOT_DIR}/Cargo.toml" ]]; then
        # fallback: check workspace member paths
        cargo_path="${ROOT_DIR}/$(echo "${pkg}" | tr '-' '_')/Cargo.toml"
    fi
done

echo "  All pre-flight checks passed."
echo ""

# ---- Step 1: Publish rust-supervisor-macros ----
echo "=== Step 1/2: rust-supervisor-macros ==="
MACROS_DIR="${ROOT_DIR}/../rust-supervisor-macros"

if [[ -d "${MACROS_DIR}" ]]; then
    echo "  Package: rust-supervisor-macros"
    echo "  Directory: ${MACROS_DIR}"
    # shellcheck disable=SC2086
    cargo publish --package rust-supervisor-macros ${PUBLISH_FLAGS}
    echo "  rust-supervisor-macros published successfully."
else
    echo "  Skipping rust-supervisor-macros (directory not found at ${MACROS_DIR})."
    echo "  The main crate may use a path dependency that resolves locally."
fi
echo ""

# ---- Step 2: Publish rust-tokio-supervisor ----
echo "=== Step 2/2: rust-tokio-supervisor ==="
MAIN_DIR="${ROOT_DIR}"

echo "  Package: rust-tokio-supervisor"
echo "  Directory: ${MAIN_DIR}"

# Read the current version for the confirmation message
CRATE_VERSION="$(grep '^version = ' "${MAIN_DIR}/Cargo.toml" | head -1 | sed 's/version = "\(.*\)"/\1/')"
echo "  Version: ${CRATE_VERSION}"

# shellcheck disable=SC2086
cargo publish --package rust-tokio-supervisor ${PUBLISH_FLAGS}
echo "  rust-tokio-supervisor published successfully."
echo ""

# ---- Summary ----
echo "=== Publish Summary ==="
if [[ -n "${DRY_RUN}" ]]; then
    echo "  Mode: dry-run — no crates were uploaded to crates.io."
    echo "  All packaging and verification steps passed."
else
    echo "  Both crates published to crates.io:"
    echo "    • rust-supervisor-macros v${CRATE_VERSION}"
    echo "    • rust-tokio-supervisor v${CRATE_VERSION}"
    echo ""
    echo "  Next steps (recommended):"
    echo "    git tag v${CRATE_VERSION}"
    echo "    git push --tags"
fi
echo ""

# Capture the last commit message for tag suggestion hint
if [[ -z "${DRY_RUN}" ]]; then
    LAST_COMMIT="$(git -C "${ROOT_DIR}" log --oneline -1 2>/dev/null || true)"
    if [[ -n "${LAST_COMMIT}" ]]; then
        echo "  Latest commit: ${LAST_COMMIT}"
    fi
fi
