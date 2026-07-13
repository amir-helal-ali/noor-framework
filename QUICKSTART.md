# Quick Start Guide | دليل البداية السريعة

## 5-Minute Setup | إعداد في 5 دقائق

### 1. Install | التثبيت

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Create a new Noor project
cargo install noor
noor new my_app
cd my_app
```

### 2. Configure | الإعداد

Edit `noor.toml`:

```toml
[app]
name = "My App"
env = "development"

[server]
port = 8080

[database]
driver = "sqlite"
url = "sqlite://storage/noor.db"

[security]
jwt_secret = "change-me-in-production"
```

### 3. Run | التشغيل

```bash
noor serve
# Visit http://localhost:8080
```

### 4. Create Your First Route | أنشئ أول مسار

```rust
use noor::*;
use noor::core::{Application, Config, Router};
use noor::core::http::{Request, Response};

fn main() -> NoorResult<()> {
    let config = Config::default();
    let mut router = Router::new();
    
    router.get("/", |_req| {
        Ok(Response::ok().html("<h1>Hello, Noor!</h1>"))
    });
    
    router.get("/api/users", |_req| {
        Ok(Response::ok().json(&serde_json::json!({
            "users": [
                {"id": 1, "name": "John"},
                {"id": 2, "name": "Jane"}
            ]
        }))?)
    });
    
    Application::new(config, router).run()
}
```

### 5. Generate Code | توليد الكود

```bash
# Generate a controller
noor make:controller User

# Generate a model
noor make:model User

# Generate a migration
noor make:migration create_users_table

# Run migrations
noor migrate
```

## Common Patterns | أنماط شائعة

### Database Query | استعلام قاعدة البيانات

```rust
use noor::core::orm::QueryBuilder;

let (sql, params) = QueryBuilder::table("users")
    .select(&["id", "name", "email"])
    .where_("status", "=", "active")
    .order_by("created_at", "desc")
    .limit(10)
    .to_sql();
```

### Authentication | المصادقة

```rust
use noor::core::auth::Jwt;

let jwt = Jwt::new("secret", "myapp", "users");
let token = jwt.generate_access_token("user123", vec!["admin".to_string()])?;
```

### Caching | التخزين المؤقت

```rust
use noor::core::cache::CacheManager;

let cache = CacheManager::for_weak_server("storage/cache")?;
let user = cache.remember("user:1", 3600, || {
    fetch_user_from_db(1)
})?;
```

### File Upload | رفع الملفات

```rust
use noor::core::upload::{FileUploader, UploadConfig};

let uploader = FileUploader::new(UploadConfig::default());
let file = uploader.save("photo.jpg", "image/jpeg", &content)?;
```

## Docker | Docker

```bash
# Quick start with Docker
docker-compose up -d

# Or build manually
docker build -t my-app .
docker run -p 8080:8080 my-app
```

## Next Steps | الخطوات التالية

1. Read the [README](README.md) for full documentation
2. Check [ADVANCED_FEATURES](ADVANCED_FEATURES.md) for advanced usage
3. Review [ARCHITECTURE](ARCHITECTURE.md) for design details
4. See [DEPLOYMENT](DEPLOYMENT.md) for production deployment

## Need Help? | تحتاج مساعدة؟

- 📖 [Documentation](https://noor-framework.github.io/docs)
- 💬 [Discord](https://discord.gg/noor)
- 🐛 [GitHub Issues](https://github.com/noor-framework/noor/issues)
