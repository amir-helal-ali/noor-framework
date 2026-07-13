// ============================================================
// Dashboard Controller - متحكم لوحة المعلومات
// ============================================================

use crate::core::http::{Request, Response};
use crate::demo::blog::models::Post;
use crate::NoorResult;

/// Show dashboard statistics
pub fn index(_req: Request) -> NoorResult<Response> {
    let posts = Post::all();
    
    Response::ok().json(&serde_json::json!({
        "stats": {
            "total_posts": posts.len(),
            "published": posts.len(),
            "drafts": 0,
        },
        "recent_posts": posts.iter().take(5).collect::<Vec<_>>(),
    }))
}
