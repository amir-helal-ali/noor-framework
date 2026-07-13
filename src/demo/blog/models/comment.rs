// ============================================================
// Comment Model - نموذج التعليق
// ============================================================

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: i64,
    pub post_id: i64,
    pub author_name: String,
    pub author_email: String,
    pub content: String,
    pub status: CommentStatus,
    pub parent_id: Option<i64>,  // For threaded replies
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CommentStatus {
    Pending,
    Approved,
    Spam,
    Rejected,
}

static COMMENTS: once_cell::sync::Lazy<Arc<RwLock<Vec<Comment>>>> = 
    once_cell::sync::Lazy::new(|| {
        Arc::new(RwLock::new(vec![
            Comment {
                id: 1,
                post_id: 1,
                author_name: "Ahmed".to_string(),
                author_email: "ahmed@example.com".to_string(),
                content: "Great framework! Looking forward to using it.".to_string(),
                status: CommentStatus::Approved,
                parent_id: None,
                created_at: "2026-07-10T14:30:00Z".to_string(),
            },
            Comment {
                id: 2,
                post_id: 1,
                author_name: "Sara".to_string(),
                author_email: "sara@example.com".to_string(),
                content: "The security features are impressive. Keep up the good work!".to_string(),
                status: CommentStatus::Approved,
                parent_id: None,
                created_at: "2026-07-10T15:00:00Z".to_string(),
            },
            Comment {
                id: 3,
                post_id: 2,
                author_name: "Mohamed".to_string(),
                author_email: "mohamed@example.com".to_string(),
                content: "Rust + Zig is an excellent choice for performance".to_string(),
                status: CommentStatus::Approved,
                parent_id: None,
                created_at: "2026-07-10T16:00:00Z".to_string(),
            },
        ]))
    });

impl Comment {
    pub fn all() -> Vec<Comment> {
        COMMENTS.read().clone()
    }
    
    pub fn find(id: i64) -> Option<Comment> {
        COMMENTS.read().iter().find(|c| c.id == id).cloned()
    }
    
    pub fn for_post(post_id: i64) -> Vec<Comment> {
        COMMENTS.read()
            .iter()
            .filter(|c| c.post_id == post_id && c.status == CommentStatus::Approved)
            .cloned()
            .collect()
    }
    
    pub fn pending() -> Vec<Comment> {
        COMMENTS.read()
            .iter()
            .filter(|c| c.status == CommentStatus::Pending)
            .cloned()
            .collect()
    }
    
    pub fn create(
        post_id: i64,
        author_name: &str,
        author_email: &str,
        content: &str,
        parent_id: Option<i64>,
    ) -> Comment {
        let mut comments = COMMENTS.write();
        let id = comments.iter().map(|c| c.id).max().unwrap_or(0) + 1;
        
        let comment = Comment {
            id,
            post_id,
            author_name: author_name.to_string(),
            author_email: author_email.to_string(),
            content: content.to_string(),
            status: CommentStatus::Pending,
            parent_id,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        
        comments.push(comment.clone());
        comment
    }
    
    pub fn approve(id: i64) -> bool {
        let mut comments = COMMENTS.write();
        if let Some(c) = comments.iter_mut().find(|c| c.id == id) {
            c.status = CommentStatus::Approved;
            return true;
        }
        false
    }
    
    pub fn mark_as_spam(id: i64) -> bool {
        let mut comments = COMMENTS.write();
        if let Some(c) = comments.iter_mut().find(|c| c.id == id) {
            c.status = CommentStatus::Spam;
            return true;
        }
        false
    }
    
    pub fn delete(id: i64) -> bool {
        let mut comments = COMMENTS.write();
        let initial = comments.len();
        comments.retain(|c| c.id != id);
        comments.len() < initial
    }
    
    pub fn count_for_post(post_id: i64) -> usize {
        COMMENTS.read()
            .iter()
            .filter(|c| c.post_id == post_id && c.status == CommentStatus::Approved)
            .count()
    }
    
    /// Get replies to a comment
    pub fn replies(parent_id: i64) -> Vec<Comment> {
        COMMENTS.read()
            .iter()
            .filter(|c| c.parent_id == Some(parent_id) && c.status == CommentStatus::Approved)
            .cloned()
            .collect()
    }
}
