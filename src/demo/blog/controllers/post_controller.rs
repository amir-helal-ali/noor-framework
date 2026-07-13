// ============================================================
// Post Controller - متحكم المقالات
// ============================================================

use crate::core::http::{Request, Response, StatusCode};
use crate::demo::blog::models::Post;
use crate::NoorResult;

/// List all posts - عرض جميع المقالات
pub fn index(_req: Request) -> NoorResult<Response> {
    let posts = Post::all();
    Response::ok().json(&serde_json::json!({
        "data": posts,
        "total": posts.len(),
    }))
}

/// Show a single post - عرض مقال واحد
pub fn show(req: Request) -> NoorResult<Response> {
    let id: i64 = req.param("id").unwrap_or("0").parse().unwrap_or(0);
    
    match Post::find(id) {
        Some(post) => Response::ok().json(&serde_json::json!({
            "data": post,
        })),
        None => Ok(Response::new(StatusCode::NOT_FOUND).json(&serde_json::json!({
            "error": "Post not found",
            "id": id,
        }))?)
    }
}

/// Create a new post - إنشاء مقال جديد
pub fn store(req: Request) -> NoorResult<Response> {
    let body: serde_json::Value = req.json()?;
    
    let title = body.get("title")
        .and_then(|v| v.as_str())
        .ok_or_else(|| crate::NoorError::Validation("Title is required".to_string()))?;
    
    let content = body.get("content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| crate::NoorError::Validation("Content is required".to_string()))?;
    
    let author = body.get("author")
        .and_then(|v| v.as_str())
        .unwrap_or("Anonymous");
    
    let post = Post::create(title, content, author);
    
    Response::new(StatusCode::CREATED).json(&serde_json::json!({
        "message": "Post created successfully",
        "data": post,
    }))
}

/// Update a post - تحديث مقال
pub fn update(req: Request) -> NoorResult<Response> {
    let id: i64 = req.param("id").unwrap_or("0").parse().unwrap_or(0);
    let body: serde_json::Value = req.json()?;
    
    match Post::find(id) {
        Some(mut post) => {
            if let Some(title) = body.get("title").and_then(|v| v.as_str()) {
                post.title = title.to_string();
            }
            if let Some(content) = body.get("content").and_then(|v| v.as_str()) {
                post.content = content.to_string();
            }
            post.updated_at = chrono::Utc::now().to_rfc3339();
            post.save();
            
            Response::ok().json(&serde_json::json!({
                "message": "Post updated successfully",
                "data": post,
            }))
        }
        None => Ok(Response::new(StatusCode::NOT_FOUND).json(&serde_json::json!({
            "error": "Post not found",
        }))?)
    }
}

/// Delete a post - حذف مقال
pub fn destroy(req: Request) -> NoorResult<Response> {
    let id: i64 = req.param("id").unwrap_or("0").parse().unwrap_or(0);
    
    if Post::delete(id) {
        Ok(Response::new(StatusCode::NO_CONTENT))
    } else {
        Ok(Response::new(StatusCode::NOT_FOUND).json(&serde_json::json!({
            "error": "Post not found",
        }))?)
    }
}
