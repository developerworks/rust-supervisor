# Spring Boot-Style DDD in Rust

Use this when the team wants a familiar Spring Boot mental model while implementing in Rust.

This is a bridge style, not a recommendation to copy Spring annotations or Java frameworks into Rust.

## When To Use

- Migrating a Java/Spring Boot team to Rust.
- Keeping controller-service-repository vocabulary for onboarding.
- Building a CRUD-heavy backend that still needs aggregate invariants.
- Designing a modular monolith with explicit application modules.

## Layer Mapping

Spring-like vocabulary:

```text
Controller -> Application Service -> Domain Model -> Repository -> Infrastructure
```

Rust mapping:

```text
interface/http -> application/handlers -> domain/aggregate -> application/ports -> infrastructure/adapters
```

Recommended module layout:

```text
src/
  ordering/
    mod.rs
    api/
      mod.rs
      dto.rs
      routes.rs
    application/
      mod.rs
      commands.rs
      services.rs
      ports.rs
    domain/
      mod.rs
      order.rs
      value_objects.rs
      events.rs
      errors.rs
    infrastructure/
      mod.rs
      persistence/
        mod.rs
        order_record.rs
        order_repository_pg.rs
      messaging/
        outbox.rs
```

## Application Service Pattern

Application services are the closest Rust equivalent to Spring `@Service` use-case services.

```rust
pub struct SubmitOrderService<R, E> {
    orders: R,
    events: E,
}

impl<R, E> SubmitOrderService<R, E>
where
    R: OrderRepository,
    E: EventPublisher<OrderEvent>,
{
    pub fn execute(&self, command: SubmitOrder) -> Result<(), SubmitOrderError> {
        let mut order = self.orders.load(command.order_id)?.ok_or(SubmitOrderError::NotFound)?;
        let events = order.submit()?;
        self.orders.save(&order)?;
        self.events.publish_after_commit(events)?;
        Ok(())
    }
}
```

Guidance:

- Use constructor injection, not global service locators.
- Use generics for library-friendly code and `Arc<dyn Trait + Send + Sync>` for runtime composition when needed.
- Keep transactions in the application layer.
- Keep domain invariants in aggregates.

## Repository Pattern

Spring Data guidance maps cleanly to “one repository per aggregate root”.

Rules:

- `OrderRepository` handles `Order`, not `OrderLine`.
- Persistence records can be separate from domain aggregates.
- Avoid generic CRUD repository traits unless the domain truly treats many aggregates uniformly.
- Query/read models can be separate from aggregate repositories.

Example:

```rust
pub trait OrderRepository {
    fn find_by_id(&self, id: OrderId) -> Result<Option<Order>, RepositoryError>;
    fn save(&self, order: &Order) -> Result<(), RepositoryError>;
}
```

## Domain Events

Spring-like event handling maps to Rust in two stages:

1. Domain aggregate records or returns domain events.
2. Application service persists state and publishes events after transaction commit.

Use an outbox for integration events when reliability matters.

Do not publish external messages directly from aggregate methods.

## Modular Monolith Style

Spring Modulith encourages domain-driven application modules. The Rust equivalent is a bounded-context module or crate with limited public API.

Rules:

- Each context exposes only application commands, queries, and selected domain types.
- Cross-context calls go through explicit ports or published events.
- Do not import another context’s infrastructure module.
- Use integration tests to verify module contracts.

## DTO and Mapping Rules

Spring Boot often has request DTOs, response DTOs, JPA entities, and domain objects. In Rust:

- API DTOs live in `api/dto.rs`.
- Persistence records live in `infrastructure/persistence`.
- Domain types live in `domain`.
- Mapping is explicit and fallible when validation is needed.

Avoid deriving API or database serialization directly on aggregates unless the domain type is intentionally the wire contract.

## Transaction Strategy

Spring `@Transactional` maps to an explicit transaction boundary in Rust.

Application service should:

- open transaction;
- load aggregate;
- execute aggregate method;
- save aggregate;
- write outbox events;
- commit;
- dispatch outbox asynchronously.

This preserves the Spring mental model while keeping Rust dependencies explicit.

## Dependency Injection

Spring uses container injection. Rust should use explicit composition:

```rust
pub struct Services {
    pub submit_order: SubmitOrderService<PgOrderRepository, OutboxPublisher>,
}
```

Or dynamic ports:

```rust
pub struct SubmitOrderService {
    orders: Arc<dyn OrderRepository + Send + Sync>,
    events: Arc<dyn EventPublisher<OrderEvent> + Send + Sync>,
}
```

Guidance:

- Prefer explicit constructors.
- Avoid runtime reflection or macro-heavy DI unless there is a clear operational payoff.
- Keep composition in `main`, `bootstrap`, or an infrastructure assembly module.

## Anti-Patterns

- Copying Spring annotations as Rust macros without a clear Rust benefit.
- Treating `Service` as a dump for all business logic.
- Making repositories table-oriented instead of aggregate-root-oriented.
- Letting HTTP DTOs become domain entities.
- Depending on infrastructure from domain because Spring projects often inject repositories everywhere.
- Publishing events before commit without outbox or transaction-aware delivery.
- Creating a trait for every class just to mimic Java interfaces.

## Sources

- Spring Modulith: application modules, module verification, module testing, events.
- Spring Data Relational: repository, aggregate, aggregate root, one repository per aggregate root.
- Spring Framework transaction-bound events: transaction-aware event listener phases.
- Microsoft Learn tactical DDD: domain/application services and domain events across aggregate boundaries.
