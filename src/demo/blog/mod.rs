// ============================================================
// Blog Demo - عرض المدونة
// ============================================================

pub mod controllers;
pub mod models;
pub mod views;

use crate::core::router::Router;
use crate::core::http::{Request, Response, StatusCode};

/// Register blog routes
/// تسجيل مسارات المدونة
pub fn register_routes(router: &mut Router) {
    // Blog homepage
    router.get("/blog", |_req| {
        let posts = models::Post::all();
        let categories = models::Category::all();
        let popular = models::Post::popular(3);
        
        let html = format!(
            r#"<html>
<head>
    <title>My Blog - Noor Framework</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; 
               max-width: 800px; margin: 0 auto; padding: 20px; line-height: 1.6; color: #333; }}
        h1 {{ color: #2c3e50; border-bottom: 2px solid #3498db; padding-bottom: 10px; }}
        .post {{ background: #f9f9f9; padding: 20px; margin: 15px 0; border-radius: 8px; 
                border-left: 4px solid #3498db; }}
        .post h2 {{ margin-top: 0; color: #2c3e50; }}
        .post-meta {{ color: #7f8c8d; font-size: 0.9em; margin-bottom: 10px; }}
        .post-content {{ margin-top: 10px; }}
        .actions {{ margin-top: 15px; }}
        .actions a {{ display: inline-block; padding: 8px 16px; background: #3498db; 
                      color: white; text-decoration: none; border-radius: 4px; margin-right: 10px; }}
        .actions a:hover {{ background: #2980b9; }}
        .new-post {{ display: inline-block; padding: 10px 20px; background: #27ae60; color: white;
                    text-decoration: none; border-radius: 4px; margin-bottom: 20px; }}
        .new-post:hover {{ background: #229954; }}
        nav {{ margin: 20px 0; }}
        nav a {{ color: #3498db; text-decoration: none; margin-right: 15px; }}
    </style>
</head>
<body>
    <nav>
        <a href="/">Home</a>
        <a href="/blog">Blog</a>
        <a href="/admin">Admin Panel</a>
    </nav>
    <h1>📝 My Blog</h1>
    <a href="/admin/posts/new" class="new-post">+ New Post</a>
    
    {}"""
        )"#,
            if posts.is_empty() {
                r#"<p>No posts yet. <a href="/admin/posts/new">Create your first post</a></p>"#.to_string()
            } else {
                posts.iter().map(|post| format!(
                    r#"<div class="post">
        <h2>{}</h2>
        <div class="post-meta">By {} on {}</div>
        <div class="post-content">{}</div>
        <div class="actions">
            <a href="/blog/{}">Read More</a>
            <a href="/admin/posts/{}/edit">Edit</a>
        </div>
    </div>"#,
                    post.title, post.author, post.created_at, post.excerpt(),
                    post.id, post.id
                )).collect::<Vec<_>>().join("\n    ")
            }
        );
        
        Ok(Response::ok().html(html))
    });
    
    // View single post
    router.get("/blog/{id}", |req: Request| {
        let id: i64 = req.param("id").unwrap_or("0").parse().unwrap_or(0);
        
        match models::Post::find(id) {
            Some(post) => {
                let html = format!(
                    r#"<html>
<head>
    <title>{} - Noor Blog</title>
    <style>
        body {{ font-family: -apple-system, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; line-height: 1.6; color: #333; }}
        h1 {{ color: #2c3e50; }}
        .post-meta {{ color: #7f8c8d; margin-bottom: 20px; }}
        .post-content {{ background: #f9f9f9; padding: 20px; border-radius: 8px; border-left: 4px solid #3498db; }}
        nav {{ margin: 20px 0; }}
        nav a {{ color: #3498db; text-decoration: none; margin-right: 15px; }}
        .actions {{ margin-top: 20px; }}
        .actions a {{ display: inline-block; padding: 8px 16px; background: #3498db; color: white; text-decoration: none; border-radius: 4px; margin-right: 10px; }}
    </style>
</head>
<body>
    <nav>
        <a href="/">Home</a>
        <a href="/blog">← Back to Blog</a>
        <a href="/admin">Admin</a>
    </nav>
    <h1>{}</h1>
    <div class="post-meta">By {} on {}</div>
    <div class="post-content">{}</div>
    <div class="actions">
        <a href="/admin/posts/{}/edit">Edit</a>
        <a href="/admin/posts/{}/delete" style="background: #e74c3c;">Delete</a>
    </div>
</body>
</html>"#,
                    post.title, post.title, post.author, post.created_at,
                    post.content, post.id, post.id
                );
                Ok(Response::ok().html(html))
            }
            None => {
                Ok(Response::new(StatusCode::NOT_FOUND).html(
                    r#"<html><body>
<h1>404 - Post Not Found</h1>
<p><a href="/blog">← Back to Blog</a></p>
</body></html>"#
                ))
            }
        }
    });
    
    // Category page
    router.get("/blog/category/{slug}", |req: Request| {
        let slug = req.param("slug").unwrap_or("");
        
        match models::Category::find_by_slug(slug) {
            Some(category) => {
                let posts = models::Post::by_category(category.id);
                let html = format!(
                    r#"<html>
<head><title>{} - Noor Blog</title>
<style>
    body {{ font-family: -apple-system, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; line-height: 1.6; color: #333; }}
    h1 {{ color: #2c3e50; border-bottom: 3px solid {}; padding-bottom: 10px; }}
    .post {{ background: #f9f9f9; padding: 20px; margin: 15px 0; border-radius: 8px; border-left: 4px solid {}; }}
    .post h2 {{ margin-top: 0; }}
    .post h2 a {{ color: #2c3e50; text-decoration: none; }}
    .post-meta {{ color: #7f8c8d; font-size: 0.9em; }}
    nav a {{ color: #3498db; text-decoration: none; margin-right: 15px; }}
</style>
</head>
<body>
    <nav>
        <a href="/">Home</a>
        <a href="/blog">← All Posts</a>
    </nav>
    <h1 style="border-color: {};">📂 {}</h1>
    <p>{}</p>
    <p><strong>{} posts in this category</strong></p>
    <hr>
    {}
</body>
</html>"#,
                    category.name,
                    category.color, category.color,
                    category.color,
                    category.name,
                    category.description,
                    posts.len(),
                    if posts.is_empty() {
                        "<p>No posts in this category yet.</p>".to_string()
                    } else {
                        posts.iter().map(|p| format!(
                            r#"<div class="post">
        <h2><a href="/blog/{}">{}</a></h2>
        <div class="post-meta">By {} • {} views • {} comments</div>
        <p>{}</p>
    </div>"#,
                            p.id, crate::core::security::Xss::escape(&p.title),
                            crate::core::security::Xss::escape(&p.author),
                            p.views,
                            models::Comment::count_for_post(p.id),
                            crate::core::security::Xss::escape(&p.excerpt())
                        )).collect::<Vec<_>>().join("\n    ")
                    }
                );
                Ok(Response::ok().html(html))
            }
            None => Ok(Response::new(StatusCode::NOT_FOUND).html("<h1>Category not found</h1>")),
        }
    });
    
    // Search
    router.get("/blog/search", |req: Request| {
        let query = req.query("q").unwrap_or("");
        let posts = if query.is_empty() {
            vec![]
        } else {
            models::Post::search(query)
        };
        
        let html = format!(
            r#"<html>
<head><title>Search: "{}" - Noor Blog</title>
<style>
    body {{ font-family: -apple-system, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; line-height: 1.6; color: #333; }}
    h1 {{ color: #2c3e50; }}
    .search-box {{ padding: 12px; width: 100%; max-width: 400px; font-size: 16px; border: 2px solid #3498db; border-radius: 6px; }}
    .post {{ background: #f9f9f9; padding: 20px; margin: 15px 0; border-radius: 8px; }}
    .post h2 a {{ color: #2c3e50; text-decoration: none; }}
    nav a {{ color: #3498db; text-decoration: none; margin-right: 15px; }}
</style>
</head>
<body>
    <nav>
        <a href="/">Home</a>
        <a href="/blog">← Blog</a>
    </nav>
    <h1>🔍 Search Posts</h1>
    <form method="GET" action="/blog/search">
        <input type="text" name="q" class="search-box" placeholder="Search..." value="{}">
        <button type="submit" style="padding: 12px 24px; background: #3498db; color: white; border: none; border-radius: 6px; cursor: pointer;">Search</button>
    </form>
    
    {}
</body>
</html>"#,
            crate::core::security::Xss::escape(query),
            crate::core::security::Xss::escape(query),
            if query.is_empty() {
                "<p>Enter a search term above.</p>".to_string()
            } else if posts.is_empty() {
                format!("<p>No results found for <strong>'{}'</strong></p>", crate::core::security::Xss::escape(query))
            } else {
                format!("<p>Found <strong>{}</strong> results for <strong>'{}'</strong>:</p><hr>",
                    posts.len(), crate::core::security::Xss::escape(query))
                + &posts.iter().map(|p| format!(
                    r#"<div class="post">
        <h2><a href="/blog/{}">{}</a></h2>
        <div style="color: #7f8c8d; font-size: 0.9em;">By {} • {} views</div>
        <p>{}</p>
    </div>"#,
                    p.id, crate::core::security::Xss::escape(&p.title),
                    crate::core::security::Xss::escape(&p.author),
                    p.views,
                    crate::core::security::Xss::escape(&p.excerpt())
                )).collect::<Vec<_>>().join("\n    ").as_str()
            }
        );
        
        Ok(Response::ok().html(html))
    });
    
    // Submit a comment
    router.post("/blog/{id}/comments", |req: Request| {
        let id: i64 = req.param("id").unwrap_or("0").parse().unwrap_or(0);
        let form = req.form();
        
        let author_name = form.get("author_name").cloned().unwrap_or_default();
        let author_email = form.get("author_email").cloned().unwrap_or_default();
        let content = form.get("content").cloned().unwrap_or_default();
        
        if author_name.is_empty() || content.is_empty() {
            return Ok(Response::new(StatusCode::BAD_REQUEST).html(
                "<h1>Error: Name and comment are required</h1>"
            ));
        }
        
        if models::Post::find(id).is_none() {
            return Ok(Response::new(StatusCode::NOT_FOUND).html("<h1>Post not found</h1>"));
        }
        
        // Validate email format
        if !author_email.is_empty() && !crate::core::security::Validator::is_email(&author_email) {
            return Ok(Response::new(StatusCode::BAD_REQUEST).html(
                "<h1>Invalid email format</h1>"
            ));
        }
        
        let _comment = models::Comment::create(
            id,
            &author_name,
            &author_email,
            &content,
            None,
        );
        
        Ok(Response::redirect(&format!("/blog/{}", id)))
    });
    
    // API: Get comments for a post
    router.get("/api/posts/{id}/comments", |req: Request| {
        let id: i64 = req.param("id").unwrap_or("0").parse().unwrap_or(0);
        let comments = models::Comment::for_post(id);
        
        Ok(Response::ok().json(&serde_json::json!({
            "data": comments,
            "total": comments.len(),
        }))?)
    });
    
    // API: Get all categories
    router.get("/api/categories", |_req| {
        let categories = models::Category::all();
        Ok(Response::ok().json(&serde_json::json!({
            "data": categories,
            "total": categories.len(),
        }))?)
    });
    
    // API: Search posts
    router.get("/api/posts/search", |req: Request| {
        let query = req.query("q").unwrap_or("");
        let posts = if query.is_empty() {
            vec![]
        } else {
            models::Post::search(query)
        };
        
        Ok(Response::ok().json(&serde_json::json!({
            "data": posts,
            "total": posts.len(),
            "query": query,
        }))?)
    });
}
