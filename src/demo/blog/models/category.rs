// ============================================================
// Category Model - نموذج التصنيف
// ============================================================

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub color: String,
    pub created_at: String,
}

static CATEGORIES: once_cell::sync::Lazy<Arc<RwLock<Vec<Category>>>> = 
    once_cell::sync::Lazy::new(|| {
        Arc::new(RwLock::new(vec![
            Category {
                id: 1,
                name: "Technology".to_string(),
                slug: "technology".to_string(),
                description: "Posts about technology and programming".to_string(),
                color: "#3498db".to_string(),
                created_at: "2026-07-10T10:00:00Z".to_string(),
            },
            Category {
                id: 2,
                name: "Design".to_string(),
                slug: "design".to_string(),
                description: "UI/UX design and creative work".to_string(),
                color: "#e74c3c".to_string(),
                created_at: "2026-07-10T10:00:00Z".to_string(),
            },
            Category {
                id: 3,
                name: "Tutorial".to_string(),
                slug: "tutorial".to_string(),
                description: "Step-by-step tutorials and guides".to_string(),
                color: "#27ae60".to_string(),
                created_at: "2026-07-10T10:00:00Z".to_string(),
            },
            Category {
                id: 4,
                name: "News".to_string(),
                slug: "news".to_string(),
                description: "Latest news and announcements".to_string(),
                color: "#f39c12".to_string(),
                created_at: "2026-07-10T10:00:00Z".to_string(),
            },
        ]))
    });

impl Category {
    pub fn all() -> Vec<Category> {
        CATEGORIES.read().clone()
    }
    
    pub fn find(id: i64) -> Option<Category> {
        CATEGORIES.read().iter().find(|c| c.id == id).cloned()
    }
    
    pub fn find_by_slug(slug: &str) -> Option<Category> {
        CATEGORIES.read().iter().find(|c| c.slug == slug).cloned()
    }
    
    pub fn create(name: &str, description: &str, color: &str) -> Category {
        let mut cats = CATEGORIES.write();
        let id = cats.iter().map(|c| c.id).max().unwrap_or(0) + 1;
        let category = Category {
            id,
            name: name.to_string(),
            slug: name.to_lowercase().replace(" ", "-"),
            description: description.to_string(),
            color: color.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        cats.push(category.clone());
        category
    }
    
    pub fn delete(id: i64) -> bool {
        let mut cats = CATEGORIES.write();
        let initial = cats.len();
        cats.retain(|c| c.id != id);
        cats.len() < initial
    }
    
    /// Count posts in this category
    pub fn post_count(&self) -> usize {
        crate::demo::blog::models::post::Post::all()
            .iter()
            .filter(|p| p.category_id == Some(self.id))
            .count()
    }
}
