# Enterprise Patterns | أنماط المؤسسات

Noor Framework includes enterprise-grade patterns for building scalable, distributed, and reliable applications.

يحتوي إطار عمل نور على أنماط مؤسسية لبناء تطبيقات قابلة للتوسع وموزعة وموثوقة.

## Table of Contents | فهرس المحتويات

1. [CQRS | فصل الأوامر عن الاستعلامات](#cqrs)
2. [Event Sourcing | مصدر الأحداث](#event-sourcing)
3. [Distributed Lock | القفل الموزع](#distributed-lock)
4. [Service Discovery | اكتشاف الخدمات](#service-discovery)
5. [API Gateway | بوابة API](#api-gateway)
6. [Idempotency | التساوي القوي](#idempotency)
7. [Outbox Pattern | صندوق الصادر](#outbox-pattern)
8. [Saga Pattern | نمط الساغا](#saga-pattern)

---

## CQRS

Command Query Responsibility Segregation - separates write operations from read operations.

فصل المسؤولية بين الأوامر والاستعلامات.

```rust
use noor::core::cqrs::{CqrsSystem, Command, CommandHandler, Query, QueryHandler};

// Define a command
#[derive(Clone)]
struct CreateUserCommand { name: String, email: String }

impl Command for CreateUserCommand {
    type Result = i64;
    fn command_type(&self) -> &'static str { "CreateUserCommand" }
}

struct CreateUserHandler;
impl CommandHandler<CreateUserCommand> for CreateUserHandler {
    fn handle(&self, cmd: CreateUserCommand) -> NoorResult<i64> {
        // Write to database
        Ok(1)
    }
}

// Define a query
#[derive(Clone)]
struct GetUserByIdQuery { id: i64 }

impl Query for GetUserByIdQuery {
    type Result = Option<String>;
    fn query_type(&self) -> &'static str { "GetUserByIdQuery" }
}

struct GetUserByIdHandler;
impl QueryHandler<GetUserByIdQuery> for GetUserByIdHandler {
    fn handle(&self, query: GetUserByIdQuery) -> NoorResult<Option<String>> {
        Ok(Some("John".to_string()))
    }
}

// Usage
let system = CqrsSystem::new();
system.command_bus.register(CreateUserHandler);
system.query_bus.register(GetUserByIdHandler);

// Write
let id = system.execute(CreateUserCommand { name: "John".into(), email: "john@ex.com".into() })?;

// Read
let user = system.query(GetUserByIdQuery { id: 1 })?;
```

---

## Event Sourcing

Store all changes as events. Rebuild state by replaying events.

خزن جميع التغييرات كأحداث. أعد بناء الحالة بإعادة تشغيل الأحداث.

```rust
use noor::core::event_sourcing::{InMemoryEventStore, EventSourcingRepository, EventFactory, AggregateRoot};

// Define aggregate
struct UserAggregate {
    id: String,
    name: String,
    active: bool,
    version: i64,
    uncommitted: Vec<DomainEvent>,
}

impl AggregateRoot for UserAggregate {
    type Id = String;
    
    fn id(&self) -> &String { &self.id }
    fn aggregate_type(&self) -> &'static str { "User" }
    
    fn apply(&mut self, event: &DomainEvent) {
        match event.event_type.as_str() {
            "UserCreated" => {
                self.name = event.data["name"].as_str().unwrap().to_string();
            }
            _ => {}
        }
        self.version = event.version;
    }
    
    fn get_uncommitted_events(&self) -> &[DomainEvent] { &self.uncommitted }
    fn clear_uncommitted_events(&mut self) { self.uncommitted.clear(); }
}

// Usage
let store = Arc::new(InMemoryEventStore::new());
let repo = EventSourcingRepository::<UserAggregate>::new(store);

// Save
let mut user = UserAggregate::new("user-1");
user.uncommitted.push(EventFactory::create(
    "user-1", "User", "UserCreated",
    serde_json::json!({"name": "John"}), 1,
));
repo.save(&mut user)?;

// Load (replays events)
let loaded = repo.load("user-1", || UserAggregate::new("user-1"))?;
```

---

## Distributed Lock

Coordinate access to shared resources across instances.

نسق الوصول للموارد المشتركة عبر المثيلات.

```rust
use noor::core::distributed_lock::{MemoryLockManager, DistributedLock};

let manager = MemoryLockManager::new();

// Execute with lock
let result = DistributedLock::with_lock(&manager, "resource", "worker1", 60, || {
    // Critical section
    Ok(42)
})?;

// With retries
let result = DistributedLock::try_with_retries(
    &manager, "resource", "worker2", 60, 3, 100, || {
        Ok(())
    }
)?;
```

---

## Service Discovery

Register and discover services in a microservices architecture.

سجل واكتشف الخدمات في بنية الـ microservices.

```rust
use noor::core::service_discovery::{ServiceRegistry, ServiceInstance, LoadBalancer, LoadBalancerStrategy};

let registry = Arc::new(ServiceRegistry::default());

// Register a service
registry.register(ServiceInstance::new("user-service", "10.0.0.1", 8080));
registry.register(ServiceInstance::new("user-service", "10.0.0.2", 8080));

// Discover instances
let instances = registry.discover("user-service");

// With load balancer
let lb = LoadBalancer::new(registry.clone(), LoadBalancerStrategy::RoundRobin);
let instance = lb.next("user-service").unwrap();
println!("Selected: {}", instance.url());
```

---

## API Gateway

Single entry point for all microservices.

نقطة دخول واحدة لجميع الـ microservices.

```rust
use noor::core::api_gateway::{ApiGateway, GatewayRoute};
use noor::core::service_discovery::ServiceRegistry;

let registry = Arc::new(ServiceRegistry::default());
registry.register(ServiceInstance::new("user-service", "localhost", 8081));
registry.register(ServiceInstance::new("post-service", "localhost", 8082));

let gateway = ApiGateway::new(registry);

gateway.route(GatewayRoute::new("/api/users", "user-service").require_auth());
gateway.route(GatewayRoute::new("/api/posts", "post-service"));

// Process request
let response = gateway.process(&request)?;
```

---

## Idempotency

Ensure duplicate requests have the same effect as a single request.

تأكد من أن الطلبات المكررة لها نفس تأثير الطلب الواحد.

```rust
use noor::core::idempotency::IdempotencyManager;

let manager = IdempotencyManager::default();

let result: i32 = manager.execute("idempotency-key-123", || {
    // This only runs once for the same key
    Ok(42)
})?;

// Second call with same key returns cached result
let cached: i32 = manager.execute("idempotency-key-123", || {
    Ok(99) // This is never called
})?;

assert_eq!(result, cached);
```

---

## Outbox Pattern

Ensure database updates and event publication happen atomically.

تأكد من تحديثات قاعدة البيانات ونشر الأحداث بشكل ذري.

```rust
use noor::core::outbox::{InMemoryOutboxStore, OutboxMessage, OutboxProcessor};

let store = Arc::new(InMemoryOutboxStore::new());

// Save message in same transaction as DB update
store.save(OutboxMessage::new("order-1", "OrderCreated", serde_json::json!({
    "order_id": 1,
    "total": 99.99
})))?;

// Process outbox asynchronously
let processor = OutboxProcessor::new(store.clone());
let processed = processor.process_batch(|msg| {
    // Publish to message broker
    println!("Publishing: {}", msg.event_type);
    Ok(())
})?;
```

---

## Saga Pattern

Manage distributed transactions across multiple services.

إدارة المعاملات الموزعة عبر خدمات متعددة.

```rust
use noor::core::saga::{Saga, SagaStep, SagaOrchestrator};

let saga = Saga::new("order_processing")
    .step(SagaStep::new(
        "create_order",
        || { /* Create order */ Ok(()) },
        || { /* Cancel order */ Ok(()) },
    ))
    .step(SagaStep::new(
        "charge_payment",
        || { /* Charge payment */ Ok(()) },
        || { /* Refund payment */ Ok(()) },
    ))
    .step(SagaStep::new(
        "ship_order",
        || { /* Ship order */ Ok(()) },
        || { /* Cancel shipment */ Ok(()) },
    ));

let orchestrator = SagaOrchestrator::new();
let result = orchestrator.execute(saga);

match result {
    SagaResult::Success(status) => println!("Order processed!"),
    SagaResult::Failed(status) => println!("Failed at: {:?}", status.failed_step),
}
```

---

## Architecture Diagram | مخطط البنية

```
┌─────────────────────────────────────────────────────────────┐
│                      API Gateway                            │
│   (Routing, Auth, Rate Limiting, CORS)                      │
└──────────────────────────┬──────────────────────────────────┘
                           │
         ┌─────────────────┼─────────────────┐
         ▼                 ▼                  ▼
┌─────────────┐   ┌─────────────┐   ┌─────────────┐
│  User       │   │  Order      │   │  Payment    │
│  Service    │   │  Service    │   │  Service    │
│             │   │             │   │             │
│ CQRS:       │   │ Event       │   │ Saga        │
│ Commands    │   │ Sourcing    │   │ Participant │
│ Queries     │   │             │   │             │
└──────┬──────┘   └──────┬──────┘   └──────┬──────┘
       │                 │                  │
       └─────────────────┼──────────────────┘
                         │
              ┌──────────┴──────────┐
              │                     │
    ┌─────────▼─────────┐ ┌────────▼────────┐
    │  Service          │ │  Distributed    │
    │  Discovery        │ │  Lock           │
    │  (Registry)       │ │  (Coordinator)  │
    └───────────────────┘ └─────────────────┘
              │
    ┌─────────▼─────────┐
    │  Outbox Pattern   │
    │  (Event Bus)      │
    └───────────────────┘
```

## When to Use Each Pattern | متى تستخدم كل نمط

| Pattern | Use Case |
|---------|----------|
| **CQRS** | Read-heavy apps with complex queries |
| **Event Sourcing** | Need audit trail, time travel, or event replay |
| **Distributed Lock** | Coordinate access across instances |
| **Service Discovery** | Microservices that scale dynamically |
| **API Gateway** | Single entry point for microservices |
| **Idempotency** | Payment processing, any duplicate-sensitive operation |
| **Outbox** | Need atomic DB + event publishing |
| **Saga** | Distributed transactions across services |

## Conclusion | خاتمة

These enterprise patterns make Noor Framework suitable for building large-scale, distributed, and mission-critical applications.

تجعل هذه الأنماط المؤسسية إطار عمل نور مناسباً لبناء تطبيقات كبيرة وموزعة وحرجة.
