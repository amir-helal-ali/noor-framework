-- ============================================================
-- Seeder: Sample Comments
-- تعبئة: تعليقات تجريبية
-- ============================================================

INSERT INTO comments (post_id, author_name, author_email, content, status, likes, created_at, updated_at)
SELECT 1, 'Ahmed Hassan', 'ahmed@example.com', 
       'Great framework! I have been looking for something like this for a long time. The performance numbers are impressive.',
       'approved', 5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM comments WHERE post_id = 1 AND author_email = 'ahmed@example.com');

INSERT INTO comments (post_id, author_name, author_email, content, status, likes, created_at, updated_at)
SELECT 1, 'Sara Mohamed', 'sara@example.com',
       'The security features are exactly what I need. Keep up the excellent work!',
       'approved', 3, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM comments WHERE post_id = 1 AND author_email = 'sara@example.com');

INSERT INTO comments (post_id, author_name, author_email, content, status, likes, created_at, updated_at)
SELECT 1, 'Omar Ali', 'omar@example.com',
       'Can you add WebSocket support in the next version? That would be amazing!',
       'approved', 8, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM comments WHERE post_id = 1 AND author_email = 'omar@example.com');

INSERT INTO comments (post_id, author_name, author_email, content, status, likes, created_at, updated_at)
SELECT 2, 'Mohamed Ibrahim', 'mohamed@example.com',
       'Rust + Zig is an excellent choice. The performance speaks for itself.',
       'approved', 4, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM comments WHERE post_id = 2 AND author_email = 'mohamed@example.com');

INSERT INTO comments (post_id, author_name, author_email, content, status, likes, created_at, updated_at)
SELECT 2, 'Fatima Ahmed', 'fatima@example.com',
       'I would love to see a comparison benchmark against other frameworks.',
       'approved', 2, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM comments WHERE post_id = 2 AND author_email = 'fatima@example.com');

INSERT INTO comments (post_id, author_name, author_email, content, status, likes, created_at, updated_at)
SELECT 3, 'Khalid Mansour', 'khalid@example.com',
       'The security-first approach is what sets Noor apart. Excellent work!',
       'approved', 12, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM comments WHERE post_id = 3 AND author_email = 'khalid@example.com');

INSERT INTO comments (post_id, author_name, author_email, content, status, likes, created_at, updated_at)
SELECT 3, 'Layla Mostafa', 'layla@example.com',
       'Could you provide more details about the RBAC implementation?',
       'pending', 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM comments WHERE post_id = 3 AND author_email = 'layla@example.com');
