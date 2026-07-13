# Security Policy | سياسة الأمان

## Overview | نظرة عامة

Noor Framework is built with security as a core principle. This document outlines our security practices, supported versions, and how to report vulnerabilities.

تم بناء إطار عمل نور مع الأمان كمبدأ أساسي. يوضح هذا المستند ممارسات الأمان والإصدارات المدعومة وكيفية الإبلاغ عن الثغرات.

## Supported Versions | الإصدارات المدعومة

| Version | Supported          |
|---------|--------------------|
| 1.3.x   | ✅ Yes              |
| 1.2.x   | ✅ Yes              |
| 1.1.x   | ⚠️ Security only    |
| < 1.1   | ❌ No               |

## Built-in Security Features | ميزات الأمان المدمجة

### 1. CSRF Protection | الحماية من CSRF

```rust
use noor::core::security::Csrf;

let csrf = Csrf::new(3600); // 1 hour token lifetime
let token = csrf.generate_token()?;
```

### 2. XSS Filtering | تصفية XSS

```rust
use noor::core::security::Xss;

let safe = Xss::escape(user_input);
let clean = Xss::new().sanitize(html_content);
```

### 3. SQL Injection Prevention | منع حقن SQL

All ORM queries use parameterized bindings:

```rust
// Safe - parameterized
let (sql, params) = QueryBuilder::table("users")
    .where_("id", "=", user_input)
    .to_sql();
```

### 4. Rate Limiting | تحديد المعدل

```rust
use noor::core::security::RateLimit;

let limiter = RateLimit::new(60, 60); // 60 req/min
```

### 5. Password Hashing | تشفير كلمة المرور

Uses Argon2id (memory-hard, resistant to GPU attacks):

```rust
let hash = Encryption::hash_password(password)?;
let valid = Encryption::verify_password(password, &hash);
```

### 6. JWT Authentication | مصادقة JWT

```rust
let jwt = Jwt::new(secret, issuer, audience);
let token = jwt.generate_access_token(user_id, roles)?;
```

### 7. RBAC | التحكم بالوصول القائم على الأدوار

```rust
let rbac = Rbac::new();
rbac.assign_role(user_id, "admin");
if rbac.can(user_id, "posts.delete") { ... }
```

### 8. Security Headers | headers الأمان

```rust
let response = Response::ok().secure_headers();
```

### 9. Encryption | التشفير

AES-256-GCM for data at rest:

```rust
let ciphertext = enc.encrypt(plaintext, &key)?;
let decrypted = enc.decrypt(&ciphertext, &key)?;
```

### 10. Audit Trail | سجل التدقيق

```rust
let audit = AuditLogger::new();
audit.log_action(user_id, action, module, description);
```

## Security Configuration | إعدادات الأمان

```toml
[security]
jwt_secret = "use-a-strong-random-secret"
jwt_expiry = 3600
session_lifetime = 86400
bcrypt_cost = 12
enable_csrf = true
enable_xss_filter = true
rate_limit_per_minute = 60
cors_origins = ["https://yourdomain.com"]
secure_headers = true
```

## Security Best Practices | أفضل ممارسات الأمان

### 1. Production Environment | بيئة الإنتاج

- ✅ Set `APP_ENV=production`
- ✅ Set a strong `JWT_SECRET` (at least 64 characters)
- ✅ Set `debug = false`
- ✅ Enable HTTPS only
- ✅ Set secure CORS origins
- ✅ Enable all security headers

### 2. Password Policy | سياسة كلمة المرور

```rust
if !Validator::is_strong_password(password) {
    return Err(NoorError::Validation("Password must be at least 8 characters with uppercase, lowercase, number, and special character".to_string()));
}
```

### 3. Input Validation | التحقق من المدخلات

Always validate and sanitize user input:

```rust
Validator::email(email, "email")?;
Validator::required(name, "name")?;
```

### 4. Session Management | إدارة الجلسات

- Use secure, HttpOnly cookies
- Set appropriate expiration
- Implement session rotation after login
- Destroy sessions on logout

### 5. File Upload Security | أمان رفع الملفات

```rust
let uploader = FileUploader::new(UploadConfig::default());
// Validates MIME type, extension, and size
uploader.save(filename, mime_type, content)?;
```

### 6. Database Security | أمان قاعدة البيانات

- Use parameterized queries (built-in)
- Limit database user privileges
- Enable query logging in development
- Use connection pooling

## Reporting a Vulnerability | الإبلاغ عن ثغرة

If you discover a security vulnerability, please:

1. **DO NOT** open a public issue
2. Email: security@noor-framework.dev
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

We will acknowledge receipt within 48 hours and provide a fix timeline.

## Security Audit | تدقيق الأمان

We recommend regular security audits:

```bash
# Run security tests
cargo test security

# Check for known vulnerabilities
cargo audit

# Static analysis
cargo clippy -- -W clippy::all
```

## Compliance | الامتثال

Noor Framework helps with compliance for:

- **GDPR** - Data protection features
- **HIPAA** - Encryption at rest
- **SOC 2** - Audit logging
- **PCI DSS** - Secure payment handling (with add-ons)

## Conclusion | خاتمة

Security is everyone's responsibility. Always follow best practices and keep your dependencies updated.

الأمان مسؤولية الجميع. اتبع دائماً أفضل الممارسات وحافظ على تحديث الاعتماديات.
