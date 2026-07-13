-- ============================================================
-- Seeder: Default Tags
-- تعبئة: الوسوم الافتراضية
-- ============================================================

INSERT INTO tags (name, slug, description, created_at, updated_at)
SELECT 'Rust', 'rust', 'The Rust programming language', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM tags WHERE slug = 'rust');

INSERT INTO tags (name, slug, description, created_at, updated_at)
SELECT 'Zig', 'zig', 'The Zig programming language', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM tags WHERE slug = 'zig');

INSERT INTO tags (name, slug, description, created_at, updated_at)
SELECT 'Framework', 'framework', 'Web framework discussions', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM tags WHERE slug = 'framework');

INSERT INTO tags (name, slug, description, created_at, updated_at)
SELECT 'Security', 'security', 'Security topics', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM tags WHERE slug = 'security');

INSERT INTO tags (name, slug, description, created_at, updated_at)
SELECT 'Performance', 'performance', 'Performance optimization', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM tags WHERE slug = 'performance');

INSERT INTO tags (name, slug, description, created_at, updated_at)
SELECT 'Tutorial', 'tutorial', 'Educational content', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM tags WHERE slug = 'tutorial');

INSERT INTO tags (name, slug, description, created_at, updated_at)
SELECT 'Beginners', 'beginners', 'Content for beginners', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM tags WHERE slug = 'beginners');

INSERT INTO tags (name, slug, description, created_at, updated_at)
SELECT 'Advanced', 'advanced', 'Advanced topics', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM tags WHERE slug = 'advanced');

INSERT INTO tags (name, slug, description, created_at, updated_at)
SELECT 'Docker', 'docker', 'Docker and containerization', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM tags WHERE slug = 'docker');

INSERT INTO tags (name, slug, description, created_at, updated_at)
SELECT 'WebAssembly', 'webassembly', 'WebAssembly technology', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM tags WHERE slug = 'webassembly');
