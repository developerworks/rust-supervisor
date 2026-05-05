# Quality Gates

## Baseline Commands

```bash
cargo fmt --check
cargo check
cargo test
cargo doc --no-deps
cargo package --list
scripts/check-coding-standard.sh
scripts/check-maintainability.sh
scripts/generate-sbom.sh
scripts/validate-sbom.sh
cargo publish --dry-run
```

## Coding Standard Gate

`scripts/check-coding-standard.sh` checks required documents, example files, YAML configuration, ASCII punctuation constraints, and No Compatibility constraints.

## Maintainability Gate

`scripts/check-maintainability.sh` checks manual pairs, docs pairs, quality gate pages, parallel governance pages, example count, and validation artifacts.

## SBOM Gate

`scripts/generate-sbom.sh` creates minimal CycloneDX JSON and SPDX JSON release artifacts. `scripts/validate-sbom.sh` checks artifact shape, current crate metadata, `Cargo.lock` digest, and sensitive information leakage.
