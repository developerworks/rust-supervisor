# Rust-Native DDD Best Practices

Use this when the goal is idiomatic Rust first, not Java/Spring familiarity.

## Principles

- Model bounded contexts before module layout.
- Make invalid states unrepresentable where practical.
- Keep aggregates synchronous, deterministic, and infrastructure-free.
- Use Rust module privacy as an aggregate boundary.
- Prefer explicit domain errors over strings, panics, or boolean return codes.
- Prefer simple concrete types until a trait boundary has real value.
- Treat async, database clients, web frameworks, message brokers, metrics, and logging as application/infrastructure concerns.

## Tactical Mapping

### Value Object

Use value objects for concepts identified only by their attributes.

Recommended Rust shape:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Quantity(u32);

impl Quantity {
    pub fn new(value: u32) -> Result<Self, QuantityError> {
        if value == 0 {
            return Err(QuantityError::Zero);
        }
        Ok(Self(value))
    }

    pub fn get(self) -> u32 {
        self.0
    }
}
```

Rules:

- Use newtypes for units, money, IDs, percentages, states, and constrained strings.
- Keep fields private unless the type is passive data with no invariant.
- Use `TryFrom`, `FromStr`, or smart constructors for validation.
- Avoid passing raw `String`, `bool`, `u64`, or `Decimal` when the business meaning matters.

### Entity

Use entities only when identity matters across time.

Rules:

- Store identity in a dedicated ID newtype.
- Do not expose mutable fields directly.
- Put behavior that changes state on methods.
- Keep equality semantics explicit: identity equality is not always structural equality.

### Aggregate Root

An aggregate is a transactional consistency boundary. Only the root should be loaded, saved, and mutated from outside.

Recommended shape:

```rust
pub struct Order {
    id: OrderId,
    lines: Vec<OrderLine>,
    status: OrderStatus,
    pending_events: Vec<OrderEvent>,
}

impl Order {
    pub fn add_line(&mut self, sku: Sku, qty: Quantity) -> Result<(), OrderError> {
        if self.status != OrderStatus::Draft {
            return Err(OrderError::AlreadySubmitted);
        }
        self.lines.push(OrderLine::new(sku, qty));
        Ok(())
    }

    pub fn submit(&mut self) -> Result<Vec<OrderEvent>, OrderError> {
        if self.lines.is_empty() {
            return Err(OrderError::EmptyOrder);
        }
        self.status = OrderStatus::Submitted;
        Ok(vec![OrderEvent::Submitted { order_id: self.id }])
    }
}
```

Rules:

- Use `&mut self` methods for state transitions.
- Enforce invariants inside the aggregate root, not in handlers only.
- Do not let repositories save child entities independently.
- Use domain events to coordinate across aggregates.
- If one command must atomically change multiple aggregates, reconsider the aggregate boundary.

### Domain Service

Use a domain service only for business logic that does not naturally belong to one aggregate or value object.

Rules:

- Keep it pure where possible.
- Pass required domain objects and policies as arguments.
- Do not hide application orchestration in a `*Service` name.

### Application Service

Application services orchestrate use cases.

They may:

- load aggregates from repositories;
- call aggregate methods;
- start/commit transactions;
- persist aggregate changes;
- publish domain or integration events after commit;
- call infrastructure ports.

They must not:

- contain core business invariants;
- mutate aggregate internals directly;
- leak database models into domain objects.

### Repository

Use one repository per aggregate root when persistence abstraction is useful.

Typical port:

```rust
pub trait OrderRepository {
    fn load(&self, id: OrderId) -> Result<Option<Order>, RepositoryError>;
    fn save(&self, order: &Order) -> Result<(), RepositoryError>;
}
```

Guidance:

- Place repository traits in the application layer if they are ports for use cases.
- Place them in the domain layer only if the domain service genuinely needs collection-like access.
- Avoid generic CRUD repositories as a default.
- Do not expose query shapes that let callers mutate child entities outside the root.

### Domain Events

Use typed event structs/enums for domain-significant facts.

Rules:

- Name events in past tense: `OrderSubmitted`, `PaymentAuthorized`.
- Do not dispatch from inside aggregate methods.
- Return events from methods or store pending events for the application layer to drain.
- Publish integration events through an outbox or equivalent after transaction commit.
- Keep internal domain events distinct from external integration events.

## Dependency Direction

Recommended Rust-first layout:

```text
src/
  ordering/
    mod.rs
    domain/
      mod.rs
      order.rs
      value_objects.rs
      events.rs
      errors.rs
    application/
      mod.rs
      commands.rs
      handlers.rs
      ports.rs
    infrastructure/
      mod.rs
      postgres_order_repository.rs
      outbox.rs
    interface/
      mod.rs
      http.rs
```

Dependency rule:

```text
interface -> application -> domain
infrastructure -> application/domain
domain -> no infrastructure
```

For larger systems, make each bounded context a crate:

```text
crates/
  ordering-domain/
  ordering-application/
  ordering-postgres/
  ordering-http/
```

## Rust-Specific Design Choices

- Use `pub(crate)` and private modules to enforce boundaries.
- Avoid `Arc<Mutex<Aggregate>>`; aggregate mutation should be loaded, changed, saved in a transaction.
- Avoid async in domain traits unless the trait is explicitly an infrastructure port.
- Prefer value-returning functions and explicit `Result`.
- Use `serde` DTOs at the boundary; do not derive wire formats on domain types by default if it exposes persistence/API shape.
- Use `#[non_exhaustive]` or private fields for public APIs that must evolve.
- Use sealed traits only when downstream implementations would break invariants.

## Testing

- Domain tests: pure unit tests, no database, no Tokio unless the domain itself is time-dependent through injected values.
- Application tests: fake repositories and fake event publishers.
- Adapter tests: real database migrations, serialization, HTTP, broker, outbox, retry behavior.
- Contract tests: verify anti-corruption layers and external API mappings.

## Anti-Patterns

- Public aggregate fields with validation in controllers.
- `domain::service` containing all business logic while entities are plain data.
- Generic `Repository<T>` for all tables.
- Repositories for child entities inside an aggregate.
- Domain objects deriving API DTOs only because it is convenient.
- `async fn` and database clients inside aggregate methods.
- Domain events used as arbitrary technical notifications.
- One trait per dependency without multiple implementations, test seam, or architectural boundary.

## Sources

- Rust Book: packages, crates, modules, privacy, and workspaces.
- Rust API Guidelines: newtypes, custom argument types, builders, private fields, future-proofing.
- Microsoft Learn: tactical DDD, aggregates as consistency boundaries, domain/application services, domain events.
- Eric Evans, Vaughn Vernon, Vlad Khononov for mainstream DDD terminology.
