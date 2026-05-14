# Quality Gates

Language: [中文](../zh/quality-gates.html)

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

## Documentation Synchronization

The manual, engineering docs, README files, examples, quickstart, public API contract, and glossary must stay synchronized. When public APIs, configuration shape, example behavior, or observability signals change, documentation must be updated in the same implementation pass.

## Coding Standard

`scripts/check-coding-standard.sh` checks required release materials, example files, primary configuration, documentation punctuation, and No Compatibility language.

## Maintainability

`scripts/check-maintainability.sh` checks paired manual and docs entries, example count, validation artifacts, the Shutdown Without Orphaned Tasks term, and the rust-config-tree term.

## SBOM And Release

`scripts/generate-sbom.sh` creates minimal CycloneDX JSON and SPDX JSON release artifacts. `scripts/validate-sbom.sh` checks file existence, JSON shape, package name, `Cargo.lock` digest, and sensitive path leakage.
