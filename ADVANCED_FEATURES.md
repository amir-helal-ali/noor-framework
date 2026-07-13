# Advanced Features | الميزات المتقدمة

This document covers the advanced features added in Noor Framework v1.3+.

يغطي هذا المستند الميزات المتقدمة المضافة في إطار عمل نور v1.3+.

## Table of Contents | فهرس المحتويات

1. [Service Container (DI) | حاوية الخدمات](#service-container-di--حاوية-الخدمات)
2. [Internationalization (i18n) | التدويل](#internationalization-i18n--التدويل)
3. [OpenAPI/Swagger | توثيق API](#openapiswagger--توثيق-api)
4. [Admin Generator | مولد لوحة التحكم](#admin-generator--مولد-لوحة-التحكم)
5. [Command Bus | ناقل الأوامر](#command-bus--ناقل-الأوامر)
6. [Webhook System | نظام الويب هوك](#webhook-system--نظام-الويب-هوك)
7. [Request Validation | التحقق من الطلبات](#request-validation--التحقق-من-الطلبات)
8. [API Versioning | إصدارات API](#api-versioning--إصدارات-api)
9. [Backup & Restore | النسخ الاحتياطي](#backup--restore--النسخ-الاحتياطي)
10. [Performance Profiler | محلل الأداء](#performance-profiler--محلل-الأداء)

---

## Service Container (DI) | حاوية الخدمات

The Service Container provides dependency injection with three lifetimes:

توفر حاوية الخدمات حقن التبعيات بثلاث دورات حياة:

### Service Lifetimes | دورات حياة الخدمة

- **Singleton**: Single instance for the entire application | مثيل واحد للتطبيق بأكمله
- **Transient**: New instance every time | مثيل جديد في كل مرة
- **Scoped**: One instance per request/scope | مثيل واحد لكل طلب

### Usage | الاستخدام

```rust
use noor::core::container::Container;

let container = Container::new();

// Register a singleton
container.singleton(|| Database { url: "...".to_string() });

// Register with dependencies
container.singleton(|c| UserRepository {
    db: c.expect::<Database>(),
});

// Register a transient
container.transient(|| EmailService);

// Register a pre-created instance
container.instance(MyConfig::default());

// Resolve
let repo = container.resolve::<UserRepository>().unwrap();
```

### Scoped Containers | الحاويات النطاقية

```rust
let container = Container::new();
container.scoped(|| RequestContext::new());

// Each scope gets its own instance
let scope = container.create_scope();
let ctx1 = scope.resolve::<RequestContext>().unwrap();
let ctx2 = scope.resolve::<RequestContext>().unwrap();
// ctx1 and ctx2 are the same within the scope
```

---

## Internationalization (i18n) | التدويل

Multi-language support with JSON translation files.

دعم متعدد اللغات مع ملفات ترجمة JSON.

### Setup | الإعداد

```rust
use noor::core::i18n::Translator;

let translator = Translator::new("ar", "en");
translator.load_directory("lang")?;
```

### Translation Files | ملفات الترجمة

```json
// lang/en.json
{
    "welcome": "Welcome",
    "greeting": "Hello, {name}!",
    "items.one": "{count} item",
    "items.other": "{count} items"
}
```

```json
// lang/ar.json
{
    "welcome": "أهلاً وسهلاً",
    "greeting": "مرحباً، {name}!",
    "items.one": "عنصر واحد",
    "items.other": "{count} عناصر"
}
```

### Usage | الاستخدام

```rust
// Simple translation
let text = translator.translate("welcome", Some("ar"));
// "أهلاً وسهلاً"

// With parameters
let mut params = HashMap::new();
params.insert("name".to_string(), "John".to_string());
let text = translator.translate_with_params("greeting", &params, Some("en"));
// "Hello, John!"

// Pluralization
let text = translator.plural("items", 5, Some("en"));
// "5 items"

// Locale detection from Accept-Language header
let locale = translator.detect_locale("ar,en;q=0.8");
// "ar"

// Text direction
let dir = translator.direction("ar");
// TextDirection::Rtl
```

### Global Helper Functions | الدوال المساعدة العامة

```rust
use noor::core::i18n::{t, tl};

// Using default locale
let text = t("welcome");

// With specific locale
let text = tl("welcome", "ar");
```

---

## OpenAPI/Swagger | توثيق API

Automatically generate OpenAPI 3.0 specification and Swagger UI.

توليد تلقائي لمواصفات OpenAPI 3.0 و Swagger UI.

### Setup | الإعداد

```rust
use noor::core::openapi::{OpenApiBuilder, OperationBuilder, SchemaBuilder};

let spec = OpenApiBuilder::new("My API", "1.0.0")
    .description("A sample API")
    .contact("Support", "support@example.com", Some("https://example.com"))
    .license("MIT", "https://opensource.org/licenses/MIT")
    .server("https://api.example.com", "Production")
    .server("http://localhost:8080", "Development")
    .with_jwt_security()
    .operation(
        "/users",
        "get",
        OperationBuilder::new("listUsers")
            .tag("Users")
            .summary("Get all users")
            .parameter("page", "query", false, "Page number", "integer")
            .parameter("limit", "query", false, "Items per page", "integer")
            .response("200", "Successful response")
            .with_jwt()
            .build()
    )
    .operation(
        "/users",
        "post",
        OperationBuilder::new("createUser")
            .tag("Users")
            .summary("Create a new user")
            .json_request(
                "User data",
                SchemaBuilder::new("object")
                    .description("User creation data")
                    .build(),
                true
            )
            .json_response(
                "201",
                "User created",
                SchemaBuilder::new("object").ref_to("User").build()
            )
            .build()
    )
    .build();
```

### Generate Swagger UI | توليد Swagger UI

```rust
let builder = OpenApiBuilder::new("My API", "1.0.0")
    .description("API documentation");

let html = builder.to_swagger_ui();
// Serve this HTML at /api/docs
```

---

## Admin Generator | مولد لوحة التحكم

Generate CRUD controllers and views automatically.

توليد المتحكمات والعروض تلقائياً.

### Usage | الاستخدام

```rust
use noor::core::admin::{ScaffoldBuilder, FieldBuilder, FieldType};

let generator = ScaffoldBuilder::new("Product")
    .table("products")
    .field(
        FieldBuilder::new("name", FieldType::String)
            .label("Product Name")
            .required()
            .searchable()
            .sortable()
            .build()
    )
    .field(
        FieldBuilder::new("description", FieldType::Textarea)
            .label("Description")
            .in_list(false)
            .build()
    )
    .field(
        FieldBuilder::new("price", FieldType::Float)
            .label("Price")
            .required()
            .validation("min:0", "Price must be positive")
            .build()
    )
    .field(
        FieldBuilder::new("status", FieldType::Select)
            .label("Status")
            .options(vec![
                ("draft".to_string(), "Draft".to_string()),
                ("active".to_string(), "Active".to_string()),
                ("inactive".to_string(), "Inactive".to_string()),
            ])
            .build()
    )
    .per_page(20)
    .searchable(true)
    .sortable(true)
    .build();

// Generate all files
let files = generator.generate_all();

// files.controller - Rust controller code
// files.list_view - Handlebars list template
// files.form_view - Handlebars form template
// files.show_view - Handlebars detail template
```

### Field Types | أنواع الحقول

| Type | Description |
|------|-------------|
| String | Short text input |
| Text | Medium text input |
| Textarea | Long text input |
| Integer | Integer number |
| Float | Decimal number |
| Boolean | Checkbox |
| Date | Date picker |
| DateTime | Date and time picker |
| Email | Email input |
| Url | URL input |
| Password | Password input |
| Select | Dropdown select |
| File | File upload |
| Image | Image upload |
| Json | JSON editor |

---

## Command Bus | ناقل الأوامر

Decouple requesting an action from performing it.

فصل طلب الإجراء عن تنفيذه.

### Define Commands | تعريف الأوامر

```rust
use noor::core::command::{Command, CommandHandler};

#[derive(Debug, Clone)]
struct CreateUser {
    name: String,
    email: String,
}

impl Command for CreateUser {
    type Result = i64;
    fn command_name(&self) -> &str { "CreateUser" }
}

struct CreateUserHandler;

impl CommandHandler<CreateUser> for CreateUserHandler {
    fn handle(&self, cmd: CreateUser) -> NoorResult<i64> {
        // Create user in database
        Ok(1) // Return user ID
    }
}
```

### Register and Dispatch | التسجيل والتنفيذ

```rust
let bus = CommandBus::new();
bus.register(CreateUserHandler);

let cmd = CreateUser {
    name: "John".to_string(),
    email: "john@example.com".to_string(),
};

let user_id = bus.dispatch(cmd)?;
```

---

## Webhook System | نظام الويب هوك

Send and receive webhooks for system integrations.

إرسال واستقبال الويب هوك لتكامل الأنظمة.

### Register Webhooks | تسجيل الويب هوك

```rust
use noor::core::webhook::WebhookManager;

let manager = WebhookManager::new();

let webhook_id = manager.create(
    "https://example.com/webhook",
    vec!["user.created".to_string(), "user.updated".to_string()],
    Some("webhook_secret".to_string()),
);
```

### Dispatch Events | إطلاق الأحداث

```rust
let results = manager.dispatch("user.created", serde_json::json!({
    "id": 123,
    "name": "John",
    "email": "john@example.com"
}));

for result in &results {
    if result.success {
        println!("Webhook delivered: {}", result.webhook_id);
    } else {
        println!("Webhook failed: {:?}", result.error);
    }
}
```

### Verify Incoming Webhooks | التحقق من الويب هوك الواردة

```rust
use noor::core::webhook::verify_incoming_webhook;

let payload = br#"{"event":"test","data":{}}"#;
let signature = request.header("x-webhook-signature").unwrap();
let secret = "shared_secret";

if verify_incoming_webhook(payload, signature, secret) {
    // Process the webhook
} else {
    // Reject the request
}
```

---

## Request Validation | التحقق من الطلبات

Fluent validation for request data.

تحقق سلس لبيانات الطلب.

### Usage | الاستخدام

```rust
use noor::core::validation::{Validator, RuleBuilder};

let validator = Validator::new()
    .rule("name", RuleBuilder::new()
        .required()
        .min(2)
        .max(50)
        .message("Name must be between 2 and 50 characters")
        .build()
    )
    .rule("email", RuleBuilder::new()
        .required()
        .email()
        .build()
    )
    .rule("password", RuleBuilder::new()
        .required()
        .min(8)
        .build()
    )
    .rule("role", RuleBuilder::new()
        .required()
        .in_values(vec![
            "admin".to_string(),
            "editor".to_string(),
            "user".to_string(),
        ])
        .build()
    );

let data = serde_json::json!({
    "name": "John",
    "email": "john@example.com",
    "password": "securepassword",
    "role": "admin"
});

let result = validator.validate(&data);

if result.valid {
    // Use validated data
    let name = result.validated.get("name");
} else {
    // Show errors
    for (field, errors) in &result.errors {
        println!("{}: {}", field, errors.join(", "));
    }
}
```

### Available Rules | القواعد المتاحة

| Rule | Description |
|------|-------------|
| `required()` | Field must be present |
| `email()` | Must be a valid email |
| `url()` | Must be a valid URL |
| `min(n)` | Minimum length |
| `max(n)` | Maximum length |
| `integer()` | Must be an integer |
| `in_values(v)` | Must be one of the values |
| `regex(p)` | Must match the pattern |

---

## API Versioning | إصدارات API

Support multiple API versions with different strategies.

دعم إصدارات متعددة لـ API باستراتيجيات مختلفة.

### URI Versioning | إصدار في URL

```rust
use noor::core::versioning::{VersionManager, VersioningStrategy};

let manager = VersionManager::new(VersioningStrategy::Uri, "v1");

// Routes: /v1/users, /v2/users
let version = manager.extract_version(&request);
// "/v1/users" -> "v1"
```

### Header Versioning | إصدار في Header

```rust
let manager = VersionManager::new(VersioningStrategy::Header, "v1");

// Accept: application/vnd.noor.v1+json
let version = manager.extract_version(&request);
```

### Query Parameter Versioning | إصدار في Query

```rust
let manager = VersionManager::new(VersioningStrategy::QueryParam, "v1");

// /users?version=1
let version = manager.extract_version(&request);
```

### Register Multiple Versions | تسجيل إصدارات متعددة

```rust
let manager = VersionManager::new(VersioningStrategy::Uri, "v1");
manager.register_version("v2");
manager.register_version("v3");

// Check if version is supported
if manager.is_supported("v2") {
    // Handle v2 request
}
```

---

## Backup & Restore | النسخ الاحتياطي

Create and restore database and file backups.

إنشاء واستعادة النسخ الاحتياطية لقاعدة البيانات والملفات.

### Setup | الإعداد

```rust
use noor::core::backup::{BackupManager, BackupConfig, BackupType};

let config = BackupConfig {
    backup_dir: "storage/backups".to_string(),
    max_backups: 10,
    compress: true,
    include_database: true,
    include_files: false,
};

let manager = BackupManager::new(config)?;
```

### Create Backup | إنشاء نسخة احتياطية

```rust
// Database backup
let backup = manager.create_backup(BackupType::Database)?;

// Full backup (database + files)
let backup = manager.create_backup(BackupType::Full)?;
```

### Restore Backup | استعادة نسخة

```rust
manager.restore_backup(&backup.id)?;
```

### List and Manage | القائمة والإدارة

```rust
// List all backups
let backups = manager.list_backups();
for backup in &backups {
    println!("{} - {} ({} bytes)", backup.filename, backup.backup_type, backup.size_bytes);
}

// Delete a backup
manager.delete_backup(&backup_id)?;

// Get total size
let total = manager.total_size();
```

### Scheduled Backups | نسخ احتياطية مجدولة

```rust
use noor::core::scheduler::Scheduler;

let scheduler = Scheduler::new();
let backup_manager = Arc::new(manager);

scheduler.daily_at("daily_backup", 2, 0, move || {
    // Run at 2:00 AM daily
    backup_manager.create_backup(BackupType::Database)?;
    Ok(())
});
```

---

## Performance Profiler | محلل الأداء

Measure and analyze code performance.

قياس وتحليل أداء الكود.

### Basic Usage | الاستخدام الأساسي

```rust
use noor::core::profiler::Profiler;

let profiler = Profiler::new();
profiler.set_enabled(true);

// Start a profile
let profile_id = profiler.start_profile("api_request");

// Measure sections
profiler.start("database_query");
let users = fetch_users();
profiler.end("database_query");

profiler.start("serialization");
let json = serde_json::to_string(&users)?;
profiler.end("serialization");

// End profile
let profile = profiler.end_profile(&profile_id).unwrap();

println!("Total: {:.2}ms", profile.total_duration_ms);
for section in &profile.sections {
    println!("  {}: {:.2}ms", section.name, section.duration_ms);
}
```

### Measure Helper | مساعد القياس

```rust
let result = profiler.measure("computation", || {
    // Expensive computation
    calculate_something()
});
```

### Macro | ماكرو

```rust
let result = profile!(profiler, "operation", {
    // Code to profile
    expensive_operation()
});
```

### Summary | الملخص

```rust
let summary = profiler.summary();
println!("Profiles: {}", summary.total_profiles);
println!("Sections: {}", summary.total_sections);
println!("Avg duration: {:.2}ms", summary.avg_duration_ms);
if let Some((name, duration)) = summary.slowest_section {
    println!("Slowest: {} ({:.2}ms)", name, duration);
}
```

### Export | التصدير

```rust
let json = profiler.export_json();
// Save to file or send to monitoring service
```

---

## Conclusion | خاتمة

These advanced features make Noor Framework a complete solution for building production-ready web applications. Each feature is designed with performance, security, and developer experience in mind.

تجعل هذه الميزات المتقدمة من إطار عمل نور حلاً كاملاً لبناء تطبيقات ويب جاهزة للإنتاج. كل ميزة مصممة مع مراعاة الأداء والأمان وتجربة المطور.
