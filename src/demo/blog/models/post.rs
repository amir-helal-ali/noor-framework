// ============================================================
// Post Model - نموذج المقال
// ============================================================

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;

/// Blog post model
/// نموذج مقال المدونة
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub author: String,
    pub category_id: Option<i64>,
    pub tags: Vec<String>,
    pub views: i64,
    pub status: PostStatus,
    pub created_at: String,
    pub updated_at: String,
}

/// Post status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PostStatus {
    Draft,
    Published,
    Archived,
}

// In-memory storage (in a real app, this would use the ORM)
static POSTS: once_cell::sync::Lazy<Arc<RwLock<Vec<Post>>>> = 
    once_cell::sync::Lazy::new(|| {
        Arc::new(RwLock::new(vec![
            Post {
                id: 1,
                title: "Welcome to Noor Framework".to_string(),
                content: "Noor is a high-performance, secure, fullstack MVC framework built with Rust and Zig. It's designed to solve common problems in existing frameworks: security vulnerabilities, poor performance on weak servers, complex developer experience, and difficulty scaling. This is the first post in our demo blog!".to_string(),
                author: "Noor Team".to_string(),
                category_id: Some(1),
                tags: vec!["rust".to_string(), "zig".to_string(), "framework".to_string()],
                views: 1523,
                status: PostStatus::Published,
                created_at: "2026-07-10T10:00:00Z".to_string(),
                updated_at: "2026-07-10T10:00:00Z".to_string(),
            },
            Post {
                id: 2,
                title: "Why Rust + Zig?".to_string(),
                content: "Rust provides memory safety without garbage collection, while Zig offers ultimate performance for hot paths. Together they create a framework that is both safe and blazingly fast. Rust's zero-cost abstractions and Zig's compile-time features make Noor the most performant PHP alternative.".to_string(),
                author: "Noor Team".to_string(),
                category_id: Some(1),
                tags: vec!["rust".to_string(), "zig".to_string(), "performance".to_string()],
                views: 892,
                status: PostStatus::Published,
                created_at: "2026-07-10T11:00:00Z".to_string(),
                updated_at: "2026-07-10T11:00:00Z".to_string(),
            },
            Post {
                id: 3,
                title: "Security First Approach".to_string(),
                content: "Noor comes with built-in CSRF protection, XSS filtering, SQL injection prevention via parameterized queries, rate limiting, secure password hashing with Argon2id, JWT authentication with blacklist support, and RBAC (Role-Based Access Control). Security is not an afterthought - it's baked into the core.".to_string(),
                author: "Security Team".to_string(),
                category_id: Some(1),
                tags: vec!["security".to_string(), "csrf".to_string(), "xss".to_string(), "jwt".to_string()],
                views: 2154,
                status: PostStatus::Published,
                created_at: "2026-07-10T12:00:00Z".to_string(),
                updated_at: "2026-07-10T12:00:00Z".to_string(),
            },
            Post {
                id: 4,
                title: "Getting Started Tutorial".to_string(),
                content: "In this tutorial, we'll walk through creating your first Noor application from scratch. We'll cover installation, project structure, routing, controllers, models, views, and deployment. By the end, you'll have a complete understanding of how to build production-ready applications with Noor.".to_string(),
                author: "Noor Team".to_string(),
                category_id: Some(3),
                tags: vec!["tutorial".to_string(), "beginners".to_string(), "guide".to_string()],
                views: 445,
                status: PostStatus::Published,
                created_at: "2026-07-10T13:00:00Z".to_string(),
                updated_at: "2026-07-10T13:00:00Z".to_string(),
            },
            Post {
                id: 5,
                title: "Building Beautiful Admin Panels".to_string(),
                content: "A great admin panel should be both functional and beautiful. Noor's built-in admin generator creates stunning interfaces with modern design patterns, responsive layouts, and intuitive navigation. Learn how to customize and extend the admin panel for your specific needs.".to_string(),
                author: "Design Team".to_string(),
                category_id: Some(2),
                tags: vec!["design".to_string(), "admin".to_string(), "ui".to_string()],
                views: 678,
                status: PostStatus::Published,
                created_at: "2026-07-10T14:00:00Z".to_string(),
                updated_at: "2026-07-10T14:00:00Z".to_string(),
            },
        ]))
    });

impl Post {
    /// Get all posts
    pub fn all() -> Vec<Post> {
        POSTS.read().clone()
    }
    
    /// Find a post by ID
    pub fn find(id: i64) -> Option<Post> {
        POSTS.read().iter().find(|p| p.id == id).cloned()
    }
    
    /// Create a new post
    pub fn create(title: &str, content: &str, author: &str) -> Post {
        let mut posts = POSTS.write();
        let id = posts.iter().map(|p| p.id).max().unwrap_or(0) + 1;
        let now = chrono::Utc::now().to_rfc3339();
        
        let post = Post {
            id,
            title: title.to_string(),
            content: content.to_string(),
            author: author.to_string(),
            category_id: None,
            tags: vec![],
            views: 0,
            status: PostStatus::Published,
            created_at: now.clone(),
            updated_at: now,
        };
        
        posts.push(post.clone());
        post
    }
    
    /// Create a new post with full options
    pub fn create_full(
        title: &str,
        content: &str,
        author: &str,
        category_id: Option<i64>,
        tags: Vec<String>,
    ) -> Post {
        let mut posts = POSTS.write();
        let id = posts.iter().map(|p| p.id).max().unwrap_or(0) + 1;
        let now = chrono::Utc::now().to_rfc3339();
        
        let post = Post {
            id,
            title: title.to_string(),
            content: content.to_string(),
            author: author.to_string(),
            category_id,
            tags,
            views: 0,
            status: PostStatus::Published,
            created_at: now.clone(),
            updated_at: now,
        };
        
        posts.push(post.clone());
        post
    }
    
    /// Increment view count
    pub fn increment_views(id: i64) {
        if let Some(post) = POSTS.write().iter_mut().find(|p| p.id == id) {
            post.views += 1;
        }
    }
    
    /// Get posts by category
    pub fn by_category(category_id: i64) -> Vec<Post> {
        POSTS.read()
            .iter()
            .filter(|p| p.category_id == Some(category_id) && p.status == PostStatus::Published)
            .cloned()
            .collect()
    }
    
    /// Search posts
    pub fn search(query: &str) -> Vec<Post> {
        let lower_query = query.to_lowercase();
        POSTS.read()
            .iter()
            .filter(|p| {
                p.status == PostStatus::Published &&
                (p.title.to_lowercase().contains(&lower_query) ||
                 p.content.to_lowercase().contains(&lower_query) ||
                 p.tags.iter().any(|t| t.to_lowercase().contains(&lower_query)))
            })
            .cloned()
            .collect()
    }
    
    /// Get popular posts (most viewed)
    pub fn popular(limit: usize) -> Vec<Post> {
        let mut posts: Vec<Post> = POSTS.read()
            .iter()
            .filter(|p| p.status == PostStatus::Published)
            .cloned()
            .collect();
        posts.sort_by(|a, b| b.views.cmp(&a.views));
        posts.into_iter().take(limit).collect()
    }
    
    /// Get related posts (same category)
    pub fn related(&self, limit: usize) -> Vec<Post> {
        POSTS.read()
            .iter()
            .filter(|p| {
                p.id != self.id &&
                p.status == PostStatus::Published &&
                p.category_id == self.category_id
            })
            .take(limit)
            .cloned()
            .collect()
    }
    
    /// Save (update) a post
    pub fn save(&mut self) {
        let mut posts = POSTS.write();
        if let Some(existing) = posts.iter_mut().find(|p| p.id == self.id) {
            *existing = self.clone();
        }
    }
    
    /// Delete a post
    pub fn delete(id: i64) -> bool {
        let mut posts = POSTS.write();
        let initial_len = posts.len();
        posts.retain(|p| p.id != id);
        posts.len() < initial_len
    }
    
    /// Get an excerpt of the content
    pub fn excerpt(&self) -> String {
        if self.content.len() > 200 {
            format!("{}...", &self.content[..200])
        } else {
            self.content.clone()
        }
    }
}
