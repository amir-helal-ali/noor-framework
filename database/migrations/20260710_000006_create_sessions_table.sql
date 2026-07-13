-- ============================================================
-- Migration: Create Sessions Table
-- إنشاء جدول الجلسات
-- ============================================================

-- UP
CREATE TABLE IF NOT EXISTS sessions (
    id VARCHAR(255) PRIMARY KEY,
    user_id INTEGER NULL,
    ip_address VARCHAR(45) NULL,
    user_agent VARCHAR(500) NULL,
    payload TEXT NOT NULL,
    last_activity INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_last_activity ON sessions(last_activity);

-- DOWN
-- DROP TABLE IF EXISTS sessions;
