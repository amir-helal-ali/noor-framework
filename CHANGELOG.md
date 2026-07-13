# Changelog | سجل التغييرات

All notable changes to the Noor Framework will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.3.0] - 2026-07-11

### Added | مضاف

#### Service Container / DI | حاوية الخدمات
- ✨ Dependency injection container with singleton, transient, and scoped lifetimes
- 🏭 Factory-based service registration
- 🔗 Automatic dependency resolution
- 📦 Instance registration for pre-created services
- 🎯 Scoped containers for request-scoped services

#### Internationalization (i18n) | التدويل
- 🌍 Multi-language support with JSON translation files
- 📝 Translation with parameter substitution
- 🔢 Pluralization support
- 🧭 RTL/LTR text direction detection
- 🔍 Auto locale detection from Accept-Language header
- 📚 Built-in Arabic and English translations

#### OpenAPI/Swagger | توثيق OpenAPI
- 📖 OpenAPI 3.0 specification generation
- 🎨 Swagger UI integration
- 🏗️ Fluent builders for operations and schemas
- 🔐 JWT security scheme support
- 📊 Request/response schema documentation

#### Admin Generator | مولد لوحة التحكم
- 🏗️ CRUD scaffolding for models
- 📝 Controller, list view, form view, and show view generation
- 🎯 Field builder with validation rules
- 📋 Support for multiple field types (string, text, select, boolean, etc.)
- ✅ Automatic validation code generation

#### Command Bus Pattern | نمط ناقل الأوامر
- 📨 Command/handler pattern implementation
- 🔗 Decoupled command execution
- 🏷️ Type-safe command dispatch

#### Webhook System | نظام الويب هوك
- 🪝 Webhook registration and management
- 🔐 HMAC-SHA256 payload signing
- ✅ Incoming webhook verification
- 📊 Failure tracking and retry support
- 🎯 Event-based webhook dispatch

#### Request Validation | التحقق من الطلبات
- ✅ Fluent validation rule builder
- 📋 Multiple validation rules per field
- 🎯 Support for: required, email, url, min, max, in, regex, integer
- 📝 Custom error messages
- 📊 Validation result with all errors

#### API Versioning | إصدارات الـ API
- 🔢 URI versioning (/v1/users)
- 📧 Header versioning (Accept header)
- ❓ Query parameter versioning (?version=1)
- 🔄 Multiple version support
- 📋 Version extraction from requests

#### Backup & Restore | النسخ الاحتياطي والاستعادة
- 💾 Database and file backups
- 🗜️ Gzip compression
- 🔐 SHA-256 checksum verification
- 📊 Backup metadata tracking
- 🧹 Automatic old backup cleanup
- 📦 TAR archive format

#### Performance Profiler | محلل الأداء
- ⏱️ Section-based profiling
- 📊 Profile summaries
- 🧠 Memory usage tracking
- 📈 Slowest section identification
- 📤 JSON export
- 🔧 Enable/disable toggle

## [1.2.0] - 2026-07-10

### Added
- Plugin system with lifecycle hooks
- Metrics & monitoring (Counter, Gauge, Histogram, Timer)
- File storage abstraction (Local, S3)
- Pagination and sorting helpers
- Health check system
- GraphQL support
- Database migrations (8 SQL files)
- Database seeders (5 SQL files)
- Development Docker setup with hot reload
- Alpine-based production Dockerfile (~10MB)

## [1.1.0] - 2026-07-10

### Added
- WebSocket server with channels and rooms
- Event emitter (publish-subscribe pattern)
- Job queue with priorities and retries
- Email service with templates
- File upload handling
- Task scheduler (cron-like)
- Database seeder
- Testing utilities
- Enhanced middleware (CORS, Throttle, Compression, Auth, Logging, Helmet)
- Comments and categories in demo
- Search functionality
- 30+ integration tests
- CSS and JS assets
- CHANGELOG.md and ARCHITECTURE.md

## [1.0.0] - 2026-07-10

### Added
- Initial release of Noor Framework
- Rust 1.75+ core with zero-cost abstractions
- Zig 0.11+ performance modules
- HTTP server with async I/O
- Fast tree-based router
- Security: CSRF, XSS, SQLi prevention, Rate limiting, Encryption, JWT, RBAC
- ORM with Query Builder and Migrations
- File and memory cache
- Handlebars template engine
- CLI tools
- Docker support
- Blog + Admin demo
- Bilingual documentation (Arabic + English)
