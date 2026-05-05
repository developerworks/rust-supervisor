# Engineering Docs

## Document Map

- `quality-gates.md`: quality gates and release readiness checks.
- `parallel-governance.md`: parallel implementation governance and documentation ownership.

## Core Contract

Engineering implementation must follow the public API contract. Examples may use only APIs owned by this project.

No Compatibility: engineering docs must not describe legacy wrappers, old migration layers, or deprecated facades.

Shutdown documentation must use Shutdown Without Orphaned Tasks. Configuration documentation must use rust-config-tree v0.1.9 for the centralized YAML configuration boundary.

## Release Contract

Release materials must cover README, LICENSE, CHANGELOG, manual pages, engineering docs, examples, SBOM artifacts, and validation artifacts.
