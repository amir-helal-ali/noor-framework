# Design Patterns | أنماط التصميم

Noor Framework implements many enterprise design patterns to help you build maintainable, scalable applications.

ينفذ إطار عمل نور العديد من أنماط التصميم المؤسسية لمساعدتك في بناء تطبيقات قابلة للصيانة والتوسع.

## Table of Contents | فهرس المحتويات

1. [Service Provider Pattern | نمط مزود الخدمة](#service-provider-pattern)
2. [Repository Pattern | نمط المستودع](#repository-pattern)
3. [API Resources | موارد API](#api-resources)
4. [Form Requests | طلبات النماذج](#form-requests)
5. [Route Model Binding | ربط النماذج بالمسارات](#route-model-binding)
6. [State Machine | آلة الحالة](#state-machine)
7. [Observer Pattern | نمط المراقب](#observer-pattern)
8. [Circuit Breaker | قاطع الدائرة](#circuit-breaker)
9. [Environment Loader | محمل البيئة](#environment-loader)
10. [Data Transfer Objects | كائنات نقل البيانات](#data-transfer-objects)
11. [Cookie Management | إدارة الكوكيز](#cookie-management)
12. [Session Drivers | مزودات الجلسات](#session-drivers)

---

## Service Provider Pattern

Service providers register and bootstrap application services.

مزودو الخدمات يسجلون ويقلعون خدمات التطبيق.

```rust
use noor::core::provider::{ProviderManager, ServiceProvider};
use noor::core::container::Container;
use std::sync::Arc;

struct MyServiceProvider;

impl ServiceProvider for MyServiceProvider {
    fn name(&self) -> &str { "my_service" }
    
    fn register(&self, container: &Container) -> noor::NoorResult<()> {
        container.singleton(|| MyService::new());
        Ok(())
    }
    
    fn boot(&self, container: &Container) -> noor::NoorResult<()> {
        println!("MyService booted!");
        Ok(())
    }
}

// Usage
let manager = ProviderManager::new();
manager.register(Arc::new(MyServiceProvider));
manager.register_all(&container)?;
manager.boot_all(&container)?;
```

### Built-in Providers

- `DatabaseProvider` - Database connection
- `CacheProvider` - Cache manager
- `AuthProvider` - JWT and RBAC
- `ViewProvider` - Template engine
- `MailProvider` - Email service
- `QueueProvider` - Job queue
- `EventProvider` - Event emitter
- `NotificationProvider` - Notifications

---

## Repository Pattern

Abstracts data access, separating business logic from persistence.

تجريد الوصول للبيانات، فصل المنطق عن التخزين.

```rust
use noor::core::repository::{Repository, InMemoryRepository, RepositoryFactory};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: i64,
    name: String,
    email: String,
}

// Create repository
let repo = RepositoryFactory::in_memory(|u: &User| u.id);

// CRUD operations
repo.create(&User { id: 1, name: "John".into(), email: "john@ex.com".into() })?;
let user = repo.find(1);
let all = repo.all();
let (page, total) = repo.paginate(1, 10);
repo.update(1, &updated_user)?;
repo.delete(1);
```

---

## API Resources

Transform models into consistent API responses.

تحويل النماذج إلى استجابات API متسقة.

```rust
use noor::core::resource::{ApiResource, ResourceCollection, JsonResponse};

struct UserResource;

impl ApiResource for UserResource {
    type Model = User;
    
    fn toArray(&self, user: &User) -> serde_json::Value {
        serde_json::json!({
            "id": user.id,
            "name": user.name,
            "email": user.email,
            // password is NOT exposed
        })
    }
}

// Single resource
let json = UserResource.toArray(&user);

// Collection
let collection = ResourceCollection::new(
    &UserResource,
    &users,
    100,  // total
    1,    // page
    10,   // per_page
    "/api/users"
);

// JSON response
let response = JsonResponse::success(collection.to_json());
```

---

## Form Requests

Encapsulate validation logic in dedicated classes.

تغليف منطق التحقق في فئات مخصصة.

```rust
use noor::core::form_request::FormRequest;
use noor::core::validation::RuleBuilder;

struct CreateUserRequest;

impl FormRequest for CreateUserRequest {
    fn rules(&self) -> Vec<(String, Vec<noor::core::validation::ValidationRule>)> {
        vec![
            ("name".to_string(), RuleBuilder::new().required().min(2).max(50).build()),
            ("email".to_string(), RuleBuilder::new().required().email().build()),
            ("password".to_string(), RuleBuilder::new().required().min(8).build()),
        ]
    }
    
    fn authorize(&self, request: &Request) -> bool {
        // Check if user is authorized
        true
    }
}

// Usage in controller
let result = CreateUserRequest.validate(&request);
if result.passes() {
    // Use result.data
} else {
    // Return result.errors
}
```

---

## Route Model Binding

Automatically resolve route parameters to model instances.

ربط معاملات المسار بالنماذج تلقائياً.

```rust
use noor::core::model_binding::{ModelBinder, ModelResolver};

struct UserResolver;
impl ModelResolver for UserResolver {
    type Model = serde_json::Value;
    fn resolve(&self, id: &str) -> Option<serde_json::Value> {
        // Fetch user from database
        Some(serde_json::json!({"id": id, "name": "John"}))
    }
}

let binder = ModelBinder::new();
binder.bind("user", Arc::new(UserResolver));

// In route handler: /users/{user}
let user = binder.resolve("user", &user_id);
```

---

## State Machine

Manage complex workflows with states and transitions.

إدارة سير العمل المعقد بالحالات والانتقالات.

```rust
use noor::core::state_machine::{StateMachine, StateMachineInstance};

#[derive(Clone, Copy, PartialEq)]
enum OrderState { Pending, Paid, Shipped, Delivered, Canceled }
#[derive(Clone, Copy, PartialEq)]
enum OrderEvent { Pay, Ship, Deliver, Cancel }

let machine = StateMachine::new(OrderState::Pending)
    .transition(OrderState::Pending, OrderEvent::Pay, OrderState::Paid)
    .transition(OrderState::Paid, OrderEvent::Ship, OrderState::Shipped)
    .transition(OrderState::Shipped, OrderEvent::Deliver, OrderState::Delivered)
    .transition(OrderState::Pending, OrderEvent::Cancel, OrderState::Canceled);

let mut order = StateMachineInstance::new("order-1", OrderState::Pending);

// Transition
order.transition(&machine, OrderEvent::Pay).unwrap();
assert_eq!(order.state(), &OrderState::Paid);

// Check valid events
let events = machine.valid_events(&OrderState::Paid);
// [Ship, Cancel]
```

---

## Observer Pattern

Listen to model lifecycle events (creating, created, updating, etc.).

الاستماع لأحداث دورة حياة النموذج.

```rust
use noor::core::observer::{Observer, ObserverRegistry};
use std::sync::Arc;

struct UserObserver;
impl Observer for UserObserver {
    fn on_creating(&self, model: &mut serde_json::Value) -> bool {
        // Modify model before saving
        model["created_at"] = serde_json::json!(chrono::Utc::now().to_rfc3339());
        true // false to cancel
    }
    
    fn on_created(&self, model: &serde_json::Value) {
        println!("User created: {:?}", model);
        // Send welcome email, etc.
    }
}

let registry = ObserverRegistry::new();
registry.observe("User", Arc::new(UserObserver));

// Fire events
let manager = registry.for_model("User");
let mut model = serde_json::json!({"name": "John"});
if manager.fire_creating(&mut model) {
    // Save to database
    manager.fire_created(&model);
}
```

---

## Circuit Breaker

Protect against cascading failures when calling external services.

حماية من الفشل المتسلسل عند استدعاء خدمات خارجية.

```rust
use noor::core::circuit_breaker::{CircuitBreaker, CircuitConfig, CircuitState};

let config = CircuitConfig {
    failure_threshold: 5,
    success_threshold: 3,
    reset_timeout: 60,
    ..Default::default()
};

let breaker = CircuitBreaker::new("api_service", config);

// Execute protected operation
let result = breaker.execute(|| {
    // Call external API
    call_external_api()
});

match result {
    Ok(data) => println!("Success: {:?}", data),
    Err(e) => println!("Failed: {}", e),
}

// Check state
println!("Circuit state: {}", breaker.state());
println!("Failure count: {}", breaker.failure_count());
```

---

## Environment Loader

Load environment variables from .env files.

تحميل متغيرات البيئة من ملفات .env.

```env
# .env file
APP_NAME=My App
APP_ENV=production
DATABASE_URL=postgres://user:pass@localhost:5432/db
JWT_SECRET=your-secret-key
```

```rust
use noor::core::env::{EnvLoader, env, env_or, env_int, env_bool};

// Load .env file
let loader = EnvLoader::new();
loader.load(".env")?;

// Access variables
let app_name = env_or("APP_NAME", "Default");
let port = env_int("PORT").unwrap_or(8080);
let debug = env_bool("DEBUG").unwrap_or(false);

// Set variables
loader.set("CUSTOM_VAR", "value");
```

---

## Data Transfer Objects

Typed objects for data transfer with validation.

كائنات منسقة لنقل البيانات مع تحقق.

```rust
use noor::core::dto::{Dto, DtoError, CreateUserDto, LoginDto, ResponseDto};

// Create DTO
let dto = CreateUserDto {
    name: "John Doe".to_string(),
    email: "john@example.com".to_string(),
    password: "Str0ng!Pass".to_string(),
    role: Some("user".to_string()),
};

// Validate
match dto.validate() {
    Ok(()) => {
        // Use dto.name, dto.email, etc.
        let response = ResponseDto::success(serde_json::json!({"id": 1}));
    }
    Err(errors) => {
        for error in &errors {
            println!("{}: {}", error.field, error.message);
        }
    }
}
```

### Custom DTOs

```rust
#[macro_export]
macro_rules! dto {
    ($name:ident {
        $($field:ident: $type:ty),* $(,)?
    }) => {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub struct $name {
            $(pub $field: $type,)*
        }
        
        impl $crate::core::dto::Dto for $name {
            fn validate(&self) -> Result<(), Vec<$crate::core::dto::DtoError>> {
                Ok(())
            }
            fn dto_name() -> &'static str {
                stringify!($name)
            }
        }
    };
}

dto!(ProductDto {
    id: i64,
    name: String,
    price: f64,
});
```

---

## Cookie Management

Advanced cookie handling with signing and encryption.

إدارة متقدمة للكوكيز مع التوقيع والتشفير.

```rust
use noor::core::cookies::{Cookie, CookieJar, SameSite, SignedCookieManager, EncryptedCookieManager};

// Basic cookie
let cookie = Cookie::new("session", "abc123")
    .max_age(3600)
    .secure()
    .http_only()
    .same_site(SameSite::Strict);

// Cookie jar
let mut jar = CookieJar::new();
jar.add(cookie);
jar.add(Cookie::new("theme", "dark"));

let headers = jar.to_headers();

// Signed cookies (tamper-proof)
let manager = SignedCookieManager::new("secret");
let cookie = manager.sign("session", "user123")?;
let verified = manager.verify(&cookie)?; // "user123"

// Encrypted cookies (confidential)
let enc_manager = EncryptedCookieManager::new("secret");
let cookie = enc_manager.encrypt("data", "sensitive_value")?;
let decrypted = enc_manager.decrypt(&cookie)?; // "sensitive_value"
```

---

## Session Drivers

Multiple session storage backends.

مزودات متعددة لتخزين الجلسات.

```rust
use noor::core::session_drivers::{SessionManager, SessionData};

// File-based (default, for weak servers)
let manager = SessionManager::file("storage/sessions", 3600)?;

// Memory-based (for testing)
let manager = SessionManager::memory(3600);

// Start a session
let mut session = manager.start()?;

// Set data
session.set("user_id", &123i64)?;
session.set("name", &"John".to_string())?;
manager.save(&session)?;

// Get data
let retrieved = manager.get(&session.id)?;
let user_id: i64 = retrieved.get("user_id")?;

// Destroy
manager.destroy(&session.id);

// Garbage collection
let cleaned = manager.gc();
```

---

## Conclusion

These design patterns help you build:

- **Maintainable** code with clear separation of concerns
- **Testable** code with dependency injection and mocking
- **Scalable** code with patterns that support growth
- **Robust** code with error handling and resilience patterns

تساعدك هذه الأنماط في بناء:
- كود **قابل للصيانة** مع فصل واضح للمسؤوليات
- كود **قابل للاختبار** مع حقن التبعيات والمحاكاة
- كود **قابل للتوسع** مع أنماط تدعم النمو
- كود **قوي** مع معالجة الأخطاء وأنماط المرونة
