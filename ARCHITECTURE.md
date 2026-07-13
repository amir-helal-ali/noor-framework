# Noor Framework Architecture | بنية إطار عمل نور

## Overview | نظرة عامة

Noor is built with a layered architecture that separates concerns and allows each component to be used independently. The framework follows modern design principles while prioritizing performance, security, and developer experience.

نور مبني ببنية طبقية تفصل المسؤوليات وتسمح باستخدام كل مكون بشكل مستقل. يتبع الإطار مبادئ التصميم الحديثة مع إعطاء الأولوية للأداء والأمان وتجربة المطور.

## Architecture Diagram | مخطط البنية

```
┌─────────────────────────────────────────────────────────────────┐
│                        Client Request                            │
└──────────────────────────────┬──────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│                      HTTP Server (Hyper)                         │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              Middleware Stack                            │   │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐  │   │
│  │  │  CORS    │→│ Throttle │→│  Auth    │→│ Logging  │  │   │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────┘  │   │
│  └─────────────────────────────────────────────────────────┘   │
└──────────────────────────────┬──────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│                        Router                                    │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Pattern Matching → Route Params → Handler Dispatch     │   │
│  └─────────────────────────────────────────────────────────┘   │
└──────────────────────────────┬──────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Controller Layer                            │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Request Validation → Business Logic → Response Build   │   │
│  └─────────────────────────────────────────────────────────┘   │
└──────────────────────────────┬──────────────────────────────────┘
                               │
              ┌────────────────┼────────────────┐
              ▼                ▼                 ▼
┌──────────────────┐ ┌──────────────────┐ ┌──────────────────┐
│   ORM Layer      │ │   Cache Layer    │ │   Auth Layer     │
│  ┌────────────┐  │ │  ┌────────────┐  │ │  ┌────────────┐  │
│  │ Query Bld  │  │ │  │  Memory    │  │ │  │    JWT     │  │
│  │ Models     │  │ │  │  File      │  │ │  │  Session   │  │
│  │ Migrations │  │ │  │  Manager   │  │ │  │   RBAC     │  │
│  └────────────┘  │ │  └────────────┘  │ │  └────────────┘  │
└──────────────────┘ └──────────────────┘ └──────────────────┘
              │                │                 │
              ▼                ▼                 ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Security Layer                              │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────┐ │
│  │   CSRF   │ │   XSS    │ │ Rate Lim │ │ Encrypt  │ │Valid.│ │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────┘ │
└─────────────────────────────────────────────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    View Template Engine                          │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │         Handlebars + Custom Helpers                      │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Response to Client                          │
└─────────────────────────────────────────────────────────────────┘
```

## Layer Breakdown | تفصيل الطبقات

### 1. HTTP Server Layer | طبقة خادم HTTP

The HTTP server is built on top of Hyper (in production) with async I/O powered by Tokio. It handles:

- Connection management
- Request parsing (with Zig optimization)
- Response serialization
- Keep-alive connections
- HTTP/1.1 compliance

```rust
Server::new(config, router)
    .run()
    .await?;
```

### 2. Middleware Layer | طبقة الوسائط

Middleware process requests before they reach the route handler. The middleware stack is executed in order:

1. **CORS** - Cross-origin resource sharing
2. **Helmet** - Security headers
3. **Throttle** - Rate limiting
4. **Auth** - Authentication
5. **Logging** - Request logging
6. **Compression** - Response compression

Each middleware can either pass the request to the next middleware or short-circuit with a response.

### 3. Router Layer | طبقة التوجيه

The router uses a fast pattern-matching algorithm to dispatch requests to handlers.

```rust
Router → Match Pattern → Extract Params → Dispatch to Handler
```

**Features:**
- Static routes: `/users`
- Dynamic params: `/users/{id}`
- Optional params: `/posts/{id?}`
- Wildcards: `/api/*`
- Route groups with shared prefix and middleware
- Named routes for URL generation

### 4. Controller Layer | طبقة المتحكمات

Controllers contain the application's business logic. They:

- Validate input
- Interact with models
- Call services
- Build and return responses

```rust
pub fn store(req: Request) -> NoorResult<Response> {
    let data: UserInput = req.json()?;
    Validator::email(&data.email, "email")?;
    let user = User::create(&data)?;
    Response::created().json(&user)
}
```

### 5. ORM Layer | طبقة ORM

The ORM provides a fluent, type-safe way to interact with the database.

**Components:**
- **Model** - Trait for defining database entities
- **QueryBuilder** - Fluent query construction
- **Database** - Connection pool management
- **Migration** - Schema versioning

All queries use parameterized binding to prevent SQL injection.

### 6. Cache Layer | طبقة التخزين المؤقت

The cache layer provides multiple storage backends:

- **MemoryCache** - Fast in-memory with LRU eviction
- **FileCache** - File-based (perfect for weak servers)
- **CacheManager** - Manages primary + fallback

```rust
// Cache-aside pattern
let user = cache.remember("user:123", 3600, || {
    fetch_user_from_db(123)
})?;
```

### 7. Security Layer | طبقة الأمان

Security is baked into the framework's core:

| Component | Purpose |
|-----------|---------|
| CSRF | Prevents cross-site request forgery |
| XSS | Filters cross-site scripting |
| RateLimit | Prevents brute force attacks |
| Encryption | AES-256-GCM, Argon2id, SHA-256, HMAC |
| Validator | Input validation |
| Jwt | Token-based authentication |
| Rbac | Role-based access control |

### 8. View Layer | طبقة العرض

The view layer renders templates using Handlebars with custom security helpers.

```rust
let html = view.render("blog/post", &data)?;
```

**Built-in helpers:**
- `{{escape value}}` - XSS-safe output
- `{{truncate text 100}}` - Truncate text
- `{{date value}}` - Format dates
- `{{json value}}` - JSON stringify

## Design Principles | مبادئ التصميم

### 1. Security First | الأمان أولاً
All security features are enabled by default. Developers must explicitly disable protections if needed.

### 2. Performance | الأداء
- Zero-cost abstractions (Rust)
- Hot path optimization (Zig)
- Minimal allocations (buffer pooling)
- Lazy initialization

### 3. Weak Server Friendly | متوافق مع السيرفرات الضعيفة
- File-based cache (no Redis required)
- SQLite default (no external DB required)
- Small binary size (~15MB)
- Low memory footprint (30-50MB)

### 4. Developer Experience | تجربة المطور
- Zero-config setup
- Powerful CLI
- Code generators
- Bilingual documentation (Arabic + English)
- Comprehensive testing utilities

### 5. Modularity | الوحدوية
Each component can be used independently:

```rust
// Use only the router
use noor::core::router::Router;

// Use only the cache
use noor::core::cache::FileCache;

// Use only security
use noor::core::security::{Csrf, Xss};
```

## Data Flow | تدفق البيانات

### Request Flow | تدفق الطلب

```
1. Client sends HTTP request
2. Server receives and parses request
3. Middleware stack processes request (CORS → Auth → Logging → ...)
4. Router matches request to handler
5. Controller executes business logic
6. Controller may:
   a. Query database via ORM
   b. Read/write cache
   c. Dispatch events
   d. Queue jobs
   e. Render template
7. Response is built
8. Middleware processes response (Compression → Headers → ...)
9. Response sent to client
```

### Async Operations | العمليات غير المتزامنة

```
1. Controller dispatches job to queue
2. Queue persists job (file/memory)
3. Queue worker picks up job
4. Handler executes (e.g., send email)
5. On success: mark complete
6. On failure: retry with backoff
```

## Directory Structure | بنية المجلدات

```
src/
├── core/                   # Framework core (do not modify)
│   ├── http/              # HTTP request/response
│   ├── router/            # Routing
│   ├── middleware/        # Built-in middleware
│   ├── orm/               # ORM and database
│   ├── auth/              # Authentication
│   ├── security/          # Security utilities
│   ├── cache/             # Caching
│   ├── view/              # Template engine
│   ├── events/            # Event system
│   ├── queue/             # Job queue
│   ├── websocket/         # WebSocket
│   ├── mail/              # Email
│   ├── upload/            # File uploads
│   ├── scheduler/         # Task scheduling
│   ├── seeder/            # Data seeding
│   ├── testing/           # Testing utilities
│   ├── config.rs          # Configuration
│   ├── application.rs     # Application bootstrap
│   └── server.rs          # HTTP server
├── demo/                   # Demo application
│   ├── blog/              # Blog demo
│   └── admin/             # Admin panel demo
├── zig/                    # Zig performance modules
└── bin/                    # Binary entry points
```

## Performance Characteristics | خصائص الأداء

| Metric | Value |
|--------|-------|
| Requests/sec | 450,000+ |
| Memory usage | 30-50MB |
| Binary size | ~15MB |
| Startup time | <100ms |
| P99 latency | <5ms |

## Security Model | نموذج الأمان

### Defense in Depth | الدفاع متعدد الطبقات

1. **Input Layer** - Validation and sanitization
2. **Transport Layer** - HTTPS, HSTS
3. **Application Layer** - CSRF, XSS, RBAC
4. **Data Layer** - Parameterized queries, encryption
5. **Output Layer** - HTML escaping, security headers

### Authentication Flow | تدفق المصادقة

```
1. User submits credentials
2. Server verifies password (Argon2id)
3. Server generates JWT (access + refresh)
4. Client stores token
5. Client sends token with each request
6. AuthMiddleware verifies token
7. RBAC checks permissions
8. Request proceeds or 401/403
```

## Extension Points | نقاط التوسعة

### Custom Middleware | middleware مخصص

```rust
use noor::core::middleware::{Middleware, MiddlewareOutcome};

struct MyMiddleware;

impl Middleware for MyMiddleware {
    fn handle(&self, req: Request) -> NoorResult<MiddlewareOutcome> {
        // Custom logic
        Ok(MiddlewareOutcome::Continue(req))
    }
    
    fn name(&self) -> &str { "my_middleware" }
}
```

### Custom Cache Driver | driver cache مخصص

```rust
use noor::core::cache::Cache;

struct MyCache;

impl Cache for MyCache {
    fn get(&self, key: &str) -> Option<Vec<u8>> { ... }
    fn set(&self, key: &str, value: &[u8], ttl: u64) -> NoorResult<()> { ... }
    fn delete(&self, key: &str) -> NoorResult<()> { ... }
    fn clear(&self) -> NoorResult<()> { ... }
    fn driver_name(&self) -> &str { "my_cache" }
}
```

### Custom ORM Driver | driver ORM مخصص

Implement the `Database` trait to support additional database engines.

## Future Plans | الخطط المستقبلية

- GraphQL support
- gRPC integration
- Plugin system
- Hot reload
- WebAssembly modules
- Built-in admin generator
- Full-text search
- Image processing
- File storage abstraction (S3, local, FTP)
