# Parallel Governance

## Parallel Scope

Implementation is split across Worker A, Worker B, Worker C, and Worker D. The lead agent reviews subagent output, corrects API drift, fills missing tests, and runs final validation.

## Collaboration Rules

- Task boundaries come from `specs/001-create-supervisor-core/tasks.md`.
- Validation paths come from `specs/001-create-supervisor-core/quickstart.md`.
- Public API names come from `specs/001-create-supervisor-core/contracts/public-api.md`.
- Examples must follow final APIs and must not create compatibility exports.
- When compile drift, API drift, or documentation drift appears, the lead agent must correct it in the same integration pass.

## Completion Evidence

Completion evidence is written to `artifacts/validation/documentation-ownership.md`.
