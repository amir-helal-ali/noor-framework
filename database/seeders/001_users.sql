-- ============================================================
-- Seeder: Default Admin User
-- تعبئة: المستخدم الإداري الافتراضي
-- ============================================================
-- Creates the default admin user with full privileges
-- ينشئ المستخدم الإداري الافتراضي بصلاحيات كاملة

-- Note: The password hash below is for "admin123" using Argon2id
-- ملاحظة: تشفير كلمة المرور أدناه هو لـ "admin123" باستخدام Argon2id

INSERT INTO users (name, email, password_hash, role, status, email_verified_at, created_at, updated_at)
SELECT 'Administrator', 'admin@noor.dev', 
       '$argon2id$v=19$m=65536,t=3,p=4$placeholder_salt$placeholder_hash',
       'super_admin', 'active', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM users WHERE email = 'admin@noor.dev');

-- Create a regular editor user
INSERT INTO users (name, email, password_hash, role, status, email_verified_at, created_at, updated_at)
SELECT 'Editor User', 'editor@noor.dev',
       '$argon2id$v=19$m=65536,t=3,p=4$placeholder_salt$placeholder_hash',
       'editor', 'active', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM users WHERE email = 'editor@noor.dev');

-- Create a regular user
INSERT INTO users (name, email, password_hash, role, status, email_verified_at, created_at, updated_at)
SELECT 'Regular User', 'user@noor.dev',
       '$argon2id$v=19$m=65536,t=3,p=4$placeholder_salt$placeholder_hash',
       'user', 'active', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM users WHERE email = 'user@noor.dev');
