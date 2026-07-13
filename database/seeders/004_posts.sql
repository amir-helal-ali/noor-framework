-- ============================================================
-- Seeder: Sample Posts
-- تعبئة: مقالات تجريبية
-- ============================================================

INSERT INTO posts (title, slug, excerpt, content, author_id, category_id, status, views, likes, published_at, meta_title, meta_description, created_at, updated_at)
SELECT 
    'Welcome to Noor Framework',
    'welcome-to-noor-framework',
    'Discover the high-performance, secure web framework built with Rust and Zig',
    'Noor is a high-performance, secure, fullstack MVC framework built with Rust and Zig. It is designed to solve common problems in existing frameworks: security vulnerabilities, poor performance on weak servers, complex developer experience, and difficulty scaling.

## Why Noor?

Noor was created to address the most pressing issues in today popular web frameworks:

1. **Performance** - Rust core + Zig hot paths = near-native speed
2. **Security** - Built-in CSRF, XSS, SQLi protection, JWT, RBAC
3. **Weak Server Friendly** - Runs on 256MB RAM, file-based cache
4. **Developer Experience** - Elegant CLI, code generators, zero-config

## Getting Started

Getting started with Noor is incredibly simple. Just install the CLI tool and create a new project:

```bash
noor new my_app
cd my_app
noor serve
```

That is it! Your application is now running at http://localhost:8080',
    1, 1, 'published', 1523, 42, CURRENT_TIMESTAMP,
    'Welcome to Noor Framework - High-Performance Web Framework',
    'Discover Noor Framework, a high-performance, secure web framework built with Rust and Zig',
    CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM posts WHERE slug = 'welcome-to-noor-framework');

INSERT INTO posts (title, slug, excerpt, content, author_id, category_id, status, views, likes, published_at, meta_title, meta_description, created_at, updated_at)
SELECT 
    'Why Rust + Zig?',
    'why-rust-plus-zig',
    'The perfect combination of safety and performance',
    'Rust provides memory safety without garbage collection, while Zig offers ultimate performance for hot paths. Together they create a framework that is both safe and blazingly fast.

## The Power of Rust

Rust zero-cost abstractions and ownership model provide:
- Memory safety without garbage collection
- Thread safety at compile time
- Zero-cost abstractions
- Excellent package ecosystem (crates.io)

## The Speed of Zig

Zig is used for performance-critical paths:
- HTTP request parsing
- Buffer pooling
- CRC32 hashing
- URL decoding

## Together: The Best of Both Worlds

By combining Rust safety with Zig speed, Noor achieves:
- 450,000+ requests per second
- 30-50MB memory usage
- Less than 100ms startup time
- 15MB binary size',
    1, 1, 'published', 892, 28, CURRENT_TIMESTAMP,
    'Why Rust + Zig? - The Perfect Combination',
    'Learn why Noor Framework combines Rust and Zig for optimal performance and safety',
    CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM posts WHERE slug = 'why-rust-plus-zig');

INSERT INTO posts (title, slug, excerpt, content, author_id, category_id, status, views, likes, published_at, meta_title, meta_description, created_at, updated_at)
SELECT 
    'Security First Approach',
    'security-first-approach',
    'How Noor Framework puts security at the forefront',
    'Noor comes with built-in CSRF protection, XSS filtering, SQL injection prevention, rate limiting, secure password hashing, JWT authentication, and RBAC.

## Built-in Security Features

### CSRF Protection
Every state-changing request (POST, PUT, DELETE) is automatically protected with CSRF tokens.

### XSS Filtering
All user input is sanitized and HTML-escaped by default.

### SQL Injection Prevention
The ORM uses parameterized queries everywhere - no exceptions.

### Rate Limiting
Built-in sliding window rate limiter prevents brute force attacks.

### Password Hashing
Passwords are hashed with Argon2id - the winner of the Password Hashing Competition.

### JWT Authentication
Secure JWT implementation with HS256, refresh tokens, and blacklist support.

### RBAC
Hierarchical Role-Based Access Control with role inheritance and wildcard permissions.',
    1, 5, 'published', 2154, 67, CURRENT_TIMESTAMP,
    'Security First Approach - Noor Framework',
    'Learn about Noor Framework built-in security features',
    CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM posts WHERE slug = 'security-first-approach');
