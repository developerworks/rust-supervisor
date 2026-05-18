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

`scripts/check-coding-standard.sh` checks these items:

- Required release materials exist.
- Five `examples/*.rs` files exist.
- Main configuration `examples/config/supervisor.yaml` exists.
- Documents avoid common full-width punctuation where ASCII is required.
- README files do not describe compatibility wrappers, migration layers, or deprecated facades.

## Maintainability Gate

`scripts/check-maintainability.sh` checks these items:

- `manual/zh` and `manual/en` tree entries stay isomorphic.
- `docs/zh` and `docs/en` tree entries stay isomorphic for quality gate pages and parallel governance pages.
- Example file count matches the contract.
- Validation artifacts exist.

## SBOM Gate

`scripts/generate-sbom.sh` creates `artifacts/sbom/rust-supervisor.cdx.json` and `artifacts/sbom/rust-supervisor.spdx.json`. `scripts/validate-sbom.sh` checks file presence, JSON shape, package name, `Cargo.lock` digest, and leakage of secrets, tokens, local absolute paths, or build scratch paths.
