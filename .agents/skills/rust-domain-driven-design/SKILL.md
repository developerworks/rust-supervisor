---
name: rust-domain-driven-design
description: Use when designing, reviewing, or refactoring Rust applications with Domain-Driven Design (DDD), including bounded contexts, aggregates, value objects, repositories, application services, ports/adapters, domain events, and comparisons between Rust-native DDD and Spring Boot-style layered DDD. Use for architecture proposals, code review checklists, module layout recommendations, or migration guidance that must follow mainstream DDD and Rust best practices rather than project-specific conventions.
---

# Rust Domain Driven Design

## Core Rule

Use independent DDD and Rust guidance. Do not infer rules from the current repository, its modules, naming, business domain, or existing architecture unless the user explicitly asks to adapt this skill to that codebase.

Prefer industry-established DDD patterns:

- Strategic DDD: bounded context, ubiquitous language, context map, anti-corruption layer.
- Tactical DDD: aggregate root, entity, value object, domain service, repository, factory, domain event.
- Rust idioms: type safety, private fields, smart constructors, explicit errors, ownership-aware APIs, module privacy, and small crates/modules.

Reject ad-hoc patterns that merely rename CRUD services as DDD.

## Workflow

1. Identify the requested mode:
   - Use `references/rust-native-ddd.md` for Rust-first architecture and implementation guidance.
   - Use `references/spring-style-ddd.md` when the user asks for Spring Boot-like layering, Java/Spring migration, controller-service-repository style, or annotation-inspired organization.
2. Start from domain language, not files:
   - list bounded contexts;
   - name aggregates and invariants;
   - define commands, events, and read models;
   - only then propose modules, traits, structs, and adapters.
3. Keep the domain pure:
   - no HTTP, SQL, ORM, broker, runtime, logger, clock, random generator, or async executor in aggregate methods;
   - inject time/IDs/policies as values or domain services when needed;
   - publish or return domain events, dispatch them outside the aggregate.
4. Choose repository ports deliberately:
   - define one repository per aggregate root when persistence abstraction is useful;
   - avoid one trait per table or per trivial dependency;
   - keep persistence DTOs separate when storage shape would leak into invariants.
5. End with validation:
   - domain unit tests for invariants;
   - application-service tests with fake ports;
   - adapter integration tests for database, HTTP, broker, and outbox behavior.

## Output Shape

When answering architecture questions, include:

- chosen option: `Rust-native` or `Spring-style`;
- bounded contexts and aggregate roots;
- module/crate layout;
- dependency direction;
- transaction and event strategy;
- testing strategy;
- explicit tradeoffs and anti-patterns to avoid.

When reviewing code, prioritize:

- aggregate boundary violations;
- anemic domain models;
- public mutable fields;
- repositories operating on child entities instead of aggregate roots;
- infrastructure leaking into domain;
- domain events dispatched before transaction commit;
- async/concurrency primitives embedded in domain entities;
- over-abstracted traits with only one implementation and no boundary value.

## Source Baseline

Use these as the baseline, not project-local custom practice:

- Eric Evans, *Domain-Driven Design*.
- Vaughn Vernon, *Implementing Domain-Driven Design*.
- Vlad Khononov, *Learning Domain-Driven Design*.
- Rust official Book and Rust API Guidelines.
- Microsoft Learn tactical DDD guidance.
- Spring official Spring Modulith, Spring Data, and transaction event documentation.
