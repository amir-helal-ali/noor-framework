# Migrations Guide | دليل الترحيلات

## Overview | نظرة عامة

Noor Framework provides a powerful migration system for managing database schema changes. Migrations are version-controlled SQL files that allow you to evolve your database schema over time.

يوفر إطار عمل نور نظام ترحيلات قوي لإدارة تغييرات قاعدة البيانات. الترحيلات هي ملفات SQL ذات تحكم في الإصدار تسمح بتطوير schema قاعدة البيانات عبر الزمن.

## Directory Structure | بنية المجلدات

```
database/
├── migrations/
│   ├── 20260710_000001_create_users_table.sql
│   ├── 20260710_000002_create_posts_table.sql
│   ├── 20260710_000003_create_categories_table.sql
│   ├── 20260710_000004_create_comments_table.sql
│   ├── 20260710_000005_create_tags_table.sql
│   ├── 20260710_000006_create_sessions_table.sql
│   ├── 20260710_000007_create_password_resets_table.sql
│   └── 20260710_000008_create_audit_logs_table.sql
└── seeders/
    ├── 001_users.sql
    ├── 002_categories.sql
    ├── 003_tags.sql
    ├── 004_posts.sql
    └── 005_comments.sql
```

## Migration File Format | صيغة ملف الترحيل

Each migration file contains both UP and DOWN sections:

كل ملف ترحيل يحتوي على قسمين UP و DOWN:

```sql
-- ============================================================
-- Migration: Description
-- ============================================================

-- UP (Run when migrating forward)
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL UNIQUE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);

-- DOWN (Run when rolling back)
-- DROP TABLE IF EXISTS users;
```

## Available Migrations | الترحيلات المتاحة

### 1. Users Table | جدول المستخدمين

**File**: `20260710_000001_create_users_table.sql`

Creates the users table with:
- Authentication fields (email, password_hash)
- Role management (role field)
- Security fields (failed_login_attempts, locked_until)
- Profile fields (avatar, bio)
- Timestamps (created_at, updated_at)

### 2. Posts Table | جدول المقالات

**File**: `20260710_000002_create_posts_table.sql`

Creates the posts table with:
- Content fields (title, slug, excerpt, content)
- SEO fields (meta_title, meta_description, meta_keywords)
- Status management (status, published_at, scheduled_at)
- Analytics (views, likes)
- Full-text search using SQLite FTS5
- Triggers to keep FTS index updated

### 3. Categories Table | جدول التصنيفات

**File**: `20260710_000003_create_categories_table.sql`

Creates the categories table with:
- Hierarchical structure (parent_id for subcategories)
- Visual customization (color, icon)
- Sorting (sort_order)
- SEO fields

### 4. Comments Table | جدول التعليقات

**File**: `20260710_000004_create_comments_table.sql`

Creates the comments table with:
- Threaded replies (parent_id)
- Author information (name, email, url, IP, user_agent)
- Moderation (status: pending, approved, spam, rejected)
- Engagement tracking (likes, dislikes)

### 5. Tags Table | جدول الوسوم

**File**: `20260710_000005_create_tags_table.sql`

Creates:
- `tags` table for tag definitions
- `post_tags` junction table for many-to-many relationship

### 6. Sessions Table | جدول الجلسات

**File**: `20260710_000006_create_sessions_table.sql`

Creates the sessions table for file-based session storage.

### 7. Password Resets Table | جدول استعادة كلمة المرور

**File**: `20260710_000007_create_password_resets_table.sql`

Creates the password resets table for password reset functionality.

### 8. Audit Logs Table | جدول سجل التدقيق

**File**: `20260710_000008_create_audit_logs_table.sql`

Creates the audit logs table for tracking all important actions.

## CLI Commands | أوامر CLI

### Create a New Migration | إنشاء ترحيل جديد

```bash
noor make:migration create_products_table
```

This creates a new file in `database/migrations/` with a timestamp prefix.

### Run Migrations | تشغيل الترحيلات

```bash
# Run all pending migrations
noor migrate

# Run a specific migration
noor migrate --path=20260710_000001_create_users_table.sql
```

### Rollback Migrations | التراجع عن الترحيلات

```bash
# Rollback the last migration
noor migrate --rollback

# Rollback all migrations
noor migrate --reset
```

### Check Migration Status | فحص حالة الترحيل

```bash
noor migrate --status
```

## Running Seeders | تشغيل المعبئات

```bash
# Run all seeders
noor db:seed

# Run a specific seeder
noor db:seed --file=001_users.sql
```

## Best Practices | أفضل الممارسات

1. **Always include both UP and DOWN sections** | اذكر قسمي UP و DOWN دائماً
2. **Use IF NOT EXISTS for CREATE statements** | استخدم IF NOT EXISTS
3. **Create indexes for frequently queried columns** | أنشئ فهارس للأعمدة المستخدمة بكثرة
4. **Use foreign keys for relationships** | استخدم foreign keys للعلاقات
5. **Add comments to explain complex changes** | أضف تعليقات للتغييرات المعقدة
6. **Test migrations on a copy of production data** | اختبر الترحيلات على نسخة من بيانات الإنتاج
7. **Never modify a migration that has been deployed** | لا تعدل ترحيلاً تم نشره

## Database Support | دعم قواعد البيانات

Noor supports three database engines:

| Database | Driver | Use Case |
|----------|--------|----------|
| SQLite | sqlite | Development, weak servers |
| PostgreSQL | postgres | Production, complex queries |
| MySQL | mysql | Production, common hosting |

Configure in `noor.toml`:

```toml
[database]
driver = "sqlite"  # or "postgres" or "mysql"
url = "sqlite://storage/noor.db"
```
