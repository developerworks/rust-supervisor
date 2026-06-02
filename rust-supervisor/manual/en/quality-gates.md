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

`scripts/check-coding-standard.sh` checks required release materials, example files, primary configuration, documentation punctuation, and No Compatibility language. Chinese-language docs in this repository must use ASCII punctuation.

## Maintainability

`scripts/check-maintainability.sh` checks isomorphic `manual/zh` and `manual/en` entries, isomorphic `docs/zh` and `docs/en` entries for quality gate and parallel governance pages, example count against the contract, validation artifacts, the Shutdown Without Orphaned Tasks term, and the rust-config-tree term.

## SBOM And Release

`scripts/generate-sbom.sh` creates `artifacts/sbom/rust-supervisor.cdx.json` and `artifacts/sbom/rust-supervisor.spdx.json`. `scripts/validate-sbom.sh` checks file presence, JSON shape, package name, `Cargo.lock` digest, and leakage of secrets, tokens, local absolute paths, or build scratch paths.
