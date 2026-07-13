# API Reference | مرجع API

Complete API reference for Noor Framework v1.9.

مرجع API كامل لإطار عمل نور v1.9.

## Table of Contents | فهرس المحتويات

1. [Core | النواة](#core--النواة)
2. [HTTP | بروتوكول HTTP](#http--بروتوكول-http)
3. [Routing | التوجيه](#routing--التوجيه)
4. [Security | الأمان](#security--الأمان)
5. [Authentication | المصادقة](#authentication--المصادقة)
6. [Database & ORM | قاعدة البيانات](#database--orm--قاعدة-البيانات)
7. [Caching | التخزين المؤقت](#caching--التخزين-المؤقت)
8. [Real-time | الوقت الفعلي](#real-time--الوقت-الفعلي)
9. [Patterns | الأنماط](#patterns--الأنماط)
10. [Advanced | المتقدمة](#advanced--المتقدمة)

---

## Core | النواة

### Application

```rust
use noor::core::{Application, Config, Router};

let config = Config::default();
let mut router = Router::new();

let app = Application::new(config, router);
app.run()?;
```

### Config

```rust
use noor::core::config::Config;

let config = Config::load(Path::new("noor.toml"))?;
```

### Container (DI)

```rust
use noor::core::container::Container;

let container = Container::new();
container.singleton(|| MyService::new());
container.transient(|| AnotherService);

let service = container.resolve::<MyService>().unwrap();
```

### Logger

```rust
use noor::core::logger::Logger;

let logger = Logger::new(Some("storage/logs/app.log".into()), LogLevel::Info);
logger.info("Application started");
logger.error("Something went wrong");
```

---

## HTTP | بروتوكول HTTP

### Request

```rust
use noor::core::http::{Request, Method};

// In handler
pub fn handler(req: Request) -> NoorResult<Response> {
    let method = req.method;          // Method::Get
    let path = &req.path;             // "/users/123"
    let id = req.param("id");         // Some("123")
    let name = req.query("name");     // Some("John")
    let auth = req.bearer_token();    // Option<&str>
    let is_ajax = req.is_ajax();      // bool
    let wants_json = req.wants_json(); // bool
    
    let body: UserData = req.json()?; // Parse JSON body
    let form = req.form();            // HashMap<String, String>
    
    Ok(Response::ok())
}
```

### Response

```rust
use noor::core::http::{Response, StatusCode};

// Different response types
let html = Response::ok().html("<h1>Hello</h1>");
let json = Response::ok().json(&serde_json::json!({"key": "value"}))?;
let text = Response::ok().text("Plain text");
let redirect = Response::redirect("/new-path");
let not_found = Response::new(StatusCode::NOT_FOUND);
let created = Response::new(StatusCode::CREATED).json(&data)?;

// With headers and cookies
let response = Response::ok()
    .html("<h1>Hello</h1>")
    .header("X-Custom", "value")
    .cookie("session", "abc123", 3600)
    .secure_headers();
```

### Status Codes

```rust
use noor::core::http::StatusCode;

StatusCode::OK           // 200
StatusCode::CREATED      // 201
StatusCode::NO_CONTENT   // 204
StatusCode::BAD_REQUEST  // 400
StatusCode::UNAUTHORIZED // 401
StatusCode::FORBIDDEN    // 403
StatusCode::NOT_FOUND    // 404
StatusCode::CONFLICT     // 409
StatusCode::UNPROCESSABLE_ENTITY // 422
StatusCode::TOO_MANY_REQUESTS    // 429
StatusCode::INTERNAL_SERVER_ERROR // 500
```

---

## Routing | التوجيه

### Basic Routes

```rust
use noor::core::router::Router;

let mut router = Router::new();

router.get("/", index_handler);
router.post("/users", create_user);
router.put("/users/{id}", update_user);
router.patch("/users/{id}", patch_user);
router.delete("/users/{id}", delete_user);
```

### Route Groups

```rust
router.group("/api/v1", vec!["auth".to_string()], |group| {
    group.get("/users", list_users);
    group.post("/users", create_user);
    group.get("/posts", list_posts);
});
```

### Named Routes

```rust
router.get("/users/{id}", get_user).name("user.show");

let url = router.url_for("user.show", &{"id".to_string() => "123".to_string()});
// Returns: "/users/123"
```

### Route Matching

```rust
// Match a request to a route
if let Some((route, params)) = router.match_route(&method, &path) {
    let id = params.get("id");
}
```

---

## Security | الأمان

### CSRF Protection

```rust
use noor::core::security::Csrf;

let csrf = Csrf::new(3600); // 1 hour token lifetime
let token = csrf.generate_token()?;
let is_valid = csrf.validate_token(&token);
```

### XSS Filtering

```rust
use noor::core::security::Xss;

let escaped = Xss::escape(user_input);
let clean = Xss::new().sanitize(html_content);
let safest = Xss::new().clean(user_input);
let is_safe = Xss::is_safe(input);
```

### Rate Limiting

```rust
use noor::core::security::RateLimit;

let limiter = RateLimit::new(60, 60); // 60 req per 60 seconds
let result = limiter.check(&client_ip);

if !result.allowed {
    // Return 429 Too Many Requests
}
```

### Encryption

```rust
use noor::core::security::Encryption;

let enc = Encryption::new();

// Password hashing
let hash = Encryption::hash_password("password")?;
let valid = Encryption::verify_password("password", &hash);

// AES-256-GCM
let key = enc.generate_key()?;
let ciphertext = enc.encrypt(b"data", &key)?;
let plaintext = enc.decrypt(&ciphertext, &key)?;

// Hashing
let hash = Encryption::sha256_hex(b"data");
let hmac = Encryption::hmac_sha256(&key, b"data")?;
```

### Validation

```rust
use noor::core::security::Validator;

Validator::required(value, "field")?;
Validator::email("user@example.com", "email")?;
Validator::min_length(password, "password", 8)?;
Validator::max_length(name, "name", 50)?;
Validator::is_email("user@example.com");
Validator::is_strong_password("Str0ng!Pass");
Validator::is_url("https://example.com");
```

---

## Authentication | المصادقة

### JWT

```rust
use noor::core::auth::Jwt;

let jwt = Jwt::new("secret", "noor", "noor_app")
    .with_expiry(3600, 86400 * 7);

let token = jwt.generate_access_token("user123", vec!["admin".to_string()])?;
let refresh = jwt.generate_refresh_token("user123")?;
let claims = jwt.verify(&token)?;
jwt.revoke(&token); // Logout
```

### Sessions

```rust
use noor::core::auth::SessionManager;

let manager = SessionManager::new("storage/sessions", 86400)?;
let mut session = manager.create()?;
session.set("user_id", 123)?;
let user_id: i64 = session.get("user_id")?.unwrap_or(0);
manager.destroy(&session.id)?;
```

### RBAC

```rust
use noor::core::auth::Rbac;

let rbac = Rbac::new();
rbac.assign_role("user1", "admin");

if rbac.can("user1", "posts.delete") {
    // Allow deletion
}

if rbac.can_all("user1", &["users.read", "users.write"]) {
    // Has both permissions
}
```

### OAuth2

```rust
use noor::core::oauth::{OAuthManager, providers};

let manager = OAuthManager::new();
manager.register(providers::google("client_id", "secret", "redirect_uri"));

let auth_url = manager.authorization_url("google", "state")?;
let token = manager.exchange_code("google", "code")?;
let user = manager.get_user_info("google", &token)?;
```

---

## Database & ORM | قاعدة البيانات

### Query Builder

```rust
use noor::core::orm::QueryBuilder;

// SELECT
let (sql, params) = QueryBuilder::table("users")
    .select(&["id", "name", "email"])
    .where_("status", "=", "active")
    .order_by("created_at", "desc")
    .limit(10)
    .to_sql();

// INSERT
let (sql, params) = QueryBuilder::insert("users")
    .set("name", "John")
    .set("email", "john@example.com")
    .to_sql();

// UPDATE
let (sql, params) = QueryBuilder::update("users")
    .set("name", "Jane")
    .where_("id", "=", 1)
    .to_sql();
```

### Advanced Query Builder

```rust
use noor::core::advanced_query::AdvancedQueryBuilder;

let (sql, params) = AdvancedQueryBuilder::table("posts")
    .select(&["posts.*", "users.name AS author"])
    .join("users", "users.id = posts.author_id")
    .left_join("categories", "categories.id = posts.category_id")
    .where_("posts.status", "=", "published")
    .where_in("posts.category_id", &[1, 2, 3])
    .where_null("posts.deleted_at")
    .group_by(&["posts.author_id"])
    .having("COUNT(*)", ">", 5)
    .order_by("posts.created_at", "desc")
    .limit(10)
    .for_update()
    .to_sql();
```

### Transactions

```rust
use noor::core::transactions::TransactionManager;

let txn = TransactionManager::new();

// Automatic commit/rollback
txn.transaction(|| {
    // Database operations
    // Auto-commits on Ok, auto-rollback on Err
    Ok(())
})?;

// Nested transactions (savepoints)
txn.transaction(|| {
    // Outer operations
    txn.transaction(|| {
        // Inner operations
        Ok(())
    })?;
    Ok(())
})?;
```

### Migrations

```bash
noor make:migration create_users_table
noor migrate
noor migrate --rollback
noor migrate --fresh --seed
```

---

## Caching | التخزين المؤقت

### File Cache

```rust
use noor::core::cache::FileCache;

let cache = FileCache::new("storage/cache", "noor:")?;
cache.set("key", b"value", 3600)?; // 1 hour TTL
let value = cache.get("key"); // Option<Vec<u8>>
cache.delete("key")?;
```

### Memory Cache

```rust
use noor::core::cache::MemoryCache;

let cache = MemoryCache::new(1000, "noor:"); // Max 1000 entries
cache.set("key", b"value", 60)?;
```

### Cache Manager

```rust
use noor::core::cache::CacheManager;

// Weak server (file only)
let cache = CacheManager::for_weak_server("storage/cache")?;

// With memory + file fallback
let cache = CacheManager::with_fallback("storage/cache", 1000)?;

// Cache-aside pattern
let user = cache.remember("user:123", 3600, || {
    fetch_user_from_db(123)
})?;

// JSON caching
cache.set_json("user:json:1", &user, 3600)?;
let cached = cache.get_json::<User>("user:json:1");
```

### Query Cache

```rust
use noor::core::query_cache::QueryCache;

let cache = QueryCache::default();

let result = cache.remember("SELECT * FROM users", &[], vec!["users".to_string()], || {
    // Execute query
    Ok(serde_json::json!([{"id": 1}]))
})?;

// Invalidate on write
cache.table_modified("users");
```

---

## Real-time | الوقت الفعلي

### WebSocket

```rust
use noor::core::websocket::{WebSocketServer, WsMessage};

let server = WebSocketServer::new();

// Broadcast to all
server.broadcast(&WsMessage::new("update", serde_json::json!({"data": "..."})));

// Send to specific user
server.send_to_user("user123", &message);

// Channel broadcast
server.join_channel("conn1", "notifications");
server.broadcast_to_channel("notifications", &message);
```

### Server-Sent Events (SSE)

```rust
use noor::core::sse::{SseServer, SseEvent};

let server = SseServer::new();

// Connect client
let (client_id, receiver) = server.connect();

// Broadcast
let event = SseEvent::data("Hello").event_type("greeting");
server.broadcast(&event);

// Channel broadcast
server.broadcast_to_channel("updates", &event);
```

### Events

```rust
use noor::core::events::EventEmitter;

let emitter = EventEmitter::new();

emitter.on("user.created", Arc::new(|event| {
    println!("User created: {:?}", event.payload);
    Ok(())
}));

emitter.fire("user.created", serde_json::json!({"id": 123}))?;
```

### Queue/Jobs

```rust
use noor::core::queue::{Queue, Job, Priority};

let queue = Queue::new();

queue.register("send_email", Arc::new(|job| {
    // Send email
    Ok(())
}));

queue.dispatch("send_email", serde_json::json!({"to": "user@example.com"}))?;
queue.dispatch_later("send_email", payload, 3600)?; // Delayed

// With priority
queue.push(Job::new("task", payload).with_priority(Priority::High))?;

// Process jobs
while queue.process_next()? {}
```

---

## Patterns | الأنماط

### Repository Pattern

```rust
use noor::core::repository::{Repository, InMemoryRepository, RepositoryFactory};

let repo = RepositoryFactory::in_memory::<User, _>(|u: &User| u.id);

repo.create(&user)?;
let user = repo.find(1);
let all = repo.all();
let (page, total) = repo.paginate(1, 10);
repo.update(1, &updated)?;
repo.delete(1);
```

### State Machine

```rust
use noor::core::state_machine::{StateMachine, StateMachineInstance};

let machine = StateMachine::new(OrderState::Pending)
    .transition(OrderState::Pending, OrderEvent::Pay, OrderState::Paid)
    .transition(OrderState::Paid, OrderEvent::Ship, OrderState::Shipped);

let mut order = StateMachineInstance::new("order-1", OrderState::Pending);
order.transition(&machine, OrderEvent::Pay)?;
```

### Circuit Breaker

```rust
use noor::core::circuit_breaker::{CircuitBreaker, CircuitConfig};

let breaker = CircuitBreaker::new("api_service", CircuitConfig::default());

let result = breaker.execute(|| {
    // Call external API
    Ok(response)
});
```

### Observer

```rust
use noor::core::observer::{ObserverRegistry, Observer};

let registry = ObserverRegistry::new();

registry.observe("User", Arc::new(MyObserver));

// Fire events
let manager = registry.for_model("User");
manager.fire_creating(&mut model);
manager.fire_created(&model);
```

### DTO

```rust
use noor::core::dto::{Dto, CreateUserDto, LoginDto};

let dto = CreateUserDto {
    name: "John".to_string(),
    email: "john@example.com".to_string(),
    password: "Str0ng!Pass".to_string(),
    role: None,
};

match dto.validate() {
    Ok(()) => { /* valid */ }
    Err(errors) => { /* handle errors */ }
}
```

---

## Advanced | المتقدمة

### Notifications

```rust
use noor::core::notification::{NotificationManager, Notification, Channel};

let manager = NotificationManager::new();

manager.send(Notification::new("user1", "Welcome", "Hello!", Channel::InApp))?;

// Multi-channel
manager.send_multi_channel("user1", "Title", "Body", vec![Channel::Email, Channel::InApp], None);
```

### Search

```rust
use noor::core::search::{SearchEngine, SearchQuery};

let engine = SearchEngine::new();
let index = engine.index("posts");

index.index(SearchDocument::new("1", "Title", "Content"));

let results = index.search(&SearchQuery {
    query: "search term".to_string(),
    limit: 10,
    ..Default::default()
});
```

### File Upload

```rust
use noor::core::upload::{FileUploader, UploadConfig};

let uploader = FileUploader::new(UploadConfig::default());
let file = uploader.save("photo.jpg", "image/jpeg", &content)?;
```

### Image Processing

```rust
use noor::core::image::{ImageProcessor, ResizeOptions};

let processor = ImageProcessor::new("storage/processed")?;
let resized = processor.resize("photo.jpg", &ResizeOptions {
    width: Some(800),
    height: Some(600),
    ..Default::default()
})?;
let thumbnail = processor.thumbnail("photo.jpg", 150)?;
```

### Backup

```rust
use noor::core::backup::{BackupManager, BackupConfig, BackupType};

let manager = BackupManager::new(BackupConfig::default())?;
let backup = manager.create_backup(BackupType::Database)?;
manager.restore_backup(&backup.id)?;
```

### Tracing

```rust
use noor::core::tracing::{Tracer, SpanKind};

let tracer = Tracer::new("noor-app");

tracer.with_span("database_query", SpanKind::Client, |span| {
    span.set_attribute("db.system", "postgresql");
    // Execute query
    Ok(())
})?;
```

### Feature Flags

```rust
use noor::core::features::FeatureFlagManager;

let manager = FeatureFlagManager::new();
manager.boolean("new_ui", "New UI", true);

if manager.is_enabled("new_ui") {
    // Show new UI
}

if manager.is_enabled_for("beta_feature", "user123") {
    // User-specific feature
}
```

### Multi-tenancy

```rust
use noor::core::tenancy::{TenantManager, Tenant, ResolutionStrategy};

let manager = TenantManager::new(ResolutionStrategy::Subdomain);

let tenant = Tenant::new("Acme Corp")
    .with_subdomain("acme")
    .with_plan(TenantPlan::Pro);

manager.register(tenant);

let current = manager.resolve_from_request(&request);
```

### Cookies

```rust
use noor::core::cookies::{Cookie, CookieJar, SignedCookieManager};

// Basic cookie
let cookie = Cookie::new("session", "abc123")
    .max_age(3600)
    .secure()
    .http_only();

// Signed cookie (tamper-proof)
let manager = SignedCookieManager::new("secret");
let cookie = manager.sign("session", "user123")?;
let verified = manager.verify(&cookie)?;

// Encrypted cookie
let enc_manager = EncryptedCookieManager::new("secret");
let cookie = enc_manager.encrypt("data", "sensitive")?;
let decrypted = enc_manager.decrypt(&cookie)?;
```

---

## CLI Commands | أوامر CLI

```bash
# Project management
noor new <project>
noor serve [--host=0.0.0.0] [--port=8080]
noor build [--release|--weak-server]

# Code generation
noor make:controller <Name>
noor make:model <Name>
noor make:migration <name>

# Database
noor migrate [--rollback] [--fresh] [--seed]
noor db:seed [--class=SeederClass]

# Utilities
noor key:generate [--show]
noor cache:clear
noor route:list
noor test
noor repl  # Interactive console
```

---

## Configuration | الإعدادات

```toml
# noor.toml
[app]
name = "My App"
env = "production"
debug = false

[server]
host = "0.0.0.0"
port = 8080
workers = 4
compression = true

[database]
driver = "sqlite"
url = "sqlite://storage/noor.db"
max_connections = 10

[security]
jwt_secret = "your-secret-key"
enable_csrf = true
enable_xss_filter = true
rate_limit_per_minute = 60

[cache]
driver = "file"
prefix = "noor:"
cache_dir = "storage/cache"

[view]
template_dir = "resources/views"
cache_templates = true
auto_reload = true

[log]
level = "info"
file = "storage/logs/app.log"
json = false
```

---

## Conclusion | خاتمة

This API reference covers all major components of Noor Framework v1.9. For more details, see:

- [README.md](README.md) - Getting started
- [ARCHITECTURE.md](ARCHITECTURE.md) - System design
- [PATTERNS.md](PATTERNS.md) - Design patterns
- [ADVANCED_FEATURES.md](ADVANCED_FEATURES.md) - Advanced usage
- [DEPLOYMENT.md](DEPLOYMENT.md) - Production deployment
- [SECURITY.md](SECURITY.md) - Security practices
