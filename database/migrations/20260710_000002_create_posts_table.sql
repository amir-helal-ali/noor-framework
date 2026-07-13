-- ============================================================
-- Migration: Create Posts Table
-- إنشاء جدول المقالات
-- ============================================================

-- UP
CREATE TABLE IF NOT EXISTS posts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title VARCHAR(255) NOT NULL,
    slug VARCHAR(255) NOT NULL UNIQUE,
    excerpt TEXT NULL,
    content TEXT NOT NULL,
    author_id INTEGER NOT NULL,
    category_id INTEGER NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'draft',
    featured_image VARCHAR(500) NULL,
    views INTEGER NOT NULL DEFAULT 0,
    likes INTEGER NOT NULL DEFAULT 0,
    is_pinned BOOLEAN NOT NULL DEFAULT 0,
    published_at TIMESTAMP NULL,
    scheduled_at TIMESTAMP NULL,
    meta_title VARCHAR(255) NULL,
    meta_description TEXT NULL,
    meta_keywords VARCHAR(500) NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (author_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (category_id) REFERENCES categories(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_posts_slug ON posts(slug);
CREATE INDEX IF NOT EXISTS idx_posts_author_id ON posts(author_id);
CREATE INDEX IF NOT EXISTS idx_posts_category_id ON posts(category_id);
CREATE INDEX IF NOT EXISTS idx_posts_status ON posts(status);
CREATE INDEX IF NOT EXISTS idx_posts_published_at ON posts(published_at);
CREATE INDEX IF NOT EXISTS idx_posts_created_at ON posts(created_at);

-- Full-text search index (SQLite)
CREATE VIRTUAL TABLE IF NOT EXISTS posts_fts USING fts5(
    title, content, excerpt,
    content='posts',
    content_rowid='id'
);

-- Trigger to keep FTS index updated
CREATE TRIGGER IF NOT EXISTS posts_ai AFTER INSERT ON posts BEGIN
    INSERT INTO posts_fts(rowid, title, content, excerpt)
    VALUES (new.id, new.title, new.content, new.excerpt);
END;

CREATE TRIGGER IF NOT EXISTS posts_ad AFTER DELETE ON posts BEGIN
    INSERT INTO posts_fts(posts_fts, rowid, title, content, excerpt)
    VALUES ('delete', old.id, old.title, old.content, old.excerpt);
END;

CREATE TRIGGER IF NOT EXISTS posts_au AFTER UPDATE ON posts BEGIN
    INSERT INTO posts_fts(posts_fts, rowid, title, content, excerpt)
    VALUES ('delete', old.id, old.title, old.content, old.excerpt);
    INSERT INTO posts_fts(rowid, title, content, excerpt)
    VALUES (new.id, new.title, new.content, new.excerpt);
END;

-- DOWN
-- DROP TRIGGER IF EXISTS posts_au;
-- DROP TRIGGER IF EXISTS posts_ad;
-- DROP TRIGGER IF EXISTS posts_ai;
-- DROP TABLE IF EXISTS posts_fts;
-- DROP TABLE IF EXISTS posts;
