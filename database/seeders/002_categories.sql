-- ============================================================
-- Seeder: Default Categories
-- تعبئة: التصنيفات الافتراضية
-- ============================================================

INSERT INTO categories (name, slug, description, color, icon, sort_order, created_at, updated_at)
SELECT 'Technology', 'technology', 'Posts about technology, programming, and software development', '#3498db', 'fas fa-laptop-code', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM categories WHERE slug = 'technology');

INSERT INTO categories (name, slug, description, color, icon, sort_order, created_at, updated_at)
SELECT 'Design', 'design', 'UI/UX design, graphics, and creative work', '#e74c3c', 'fas fa-palette', 2, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM categories WHERE slug = 'design');

INSERT INTO categories (name, slug, description, color, icon, sort_order, created_at, updated_at)
SELECT 'Tutorial', 'tutorial', 'Step-by-step tutorials and guides', '#27ae60', 'fas fa-graduation-cap', 3, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM categories WHERE slug = 'tutorial');

INSERT INTO categories (name, slug, description, color, icon, sort_order, created_at, updated_at)
SELECT 'News', 'news', 'Latest news and announcements', '#f39c12', 'fas fa-newspaper', 4, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM categories WHERE slug = 'news');

INSERT INTO categories (name, slug, description, color, icon, sort_order, created_at, updated_at)
SELECT 'Security', 'security', 'Security best practices and vulnerability discussions', '#9b59b6', 'fas fa-shield-alt', 5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM categories WHERE slug = 'security');

INSERT INTO categories (name, slug, description, color, icon, sort_order, created_at, updated_at)
SELECT 'Performance', 'performance', 'Performance optimization and benchmarking', '#1abc9c', 'fas fa-tachometer-alt', 6, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM categories WHERE slug = 'performance');
