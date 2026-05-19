# Engineering Docs

## Document Map

- `adr/`: architecture decision records — 12 decisions covering supervisor model, Tokio primitives, three-directory architecture, IPC security, policy pipeline, etc.
- `architecture.md`: system architecture, module dependency graph, data flow, and key design decisions.
- `change-log.md`: documentation change log.
- `context-map.md`: project context map — code, docs, specs, tests, CI/CD overview.
- `environment.md`: development environment setup, toolchain, dependencies, and CI/CD configuration.
  | `operations.md`: operations guide — deployment, health checks, incident response runbooks, chaos/soak test execution, release matrix validation, performance tuning.
  | `product-roadmap.md`: product roadmap, milestone planning, and feature slice status (006-1 to 006-8 completed).
- `quality-gates.md`: quality gates and release readiness checks.
- `security.md`: security documentation — IPC control points C1-C9, supply chain security, audit trail.
- `parallel-governance.md`: parallel implementation governance and documentation ownership.

## Core Contract

Engineering implementation must follow the public API contract. Examples may use only APIs owned by this project.

No Compatibility: engineering docs must not describe legacy wrappers, old migration layers, or deprecated facades.

Shutdown documentation must use Shutdown Without Orphaned Tasks. Configuration documentation must use rust-config-tree v0.1.9 for the centralized YAML configuration boundary.

## Release Contract

Release materials must cover README, LICENSE, CHANGELOG, manual pages, engineering docs, examples, SBOM artifacts, and validation artifacts.
