// ============================================================
// Admin Panel - لوحة التحكم
// ============================================================

use crate::core::router::Router;
use crate::core::http::{Request, Response, StatusCode};
use crate::demo::blog::models::Post;

/// Register admin routes
pub fn register_routes(router: &mut Router) {
    // Admin login page
    router.get("/admin/login", |_req| {
        Ok(Response::ok().html(login_page()))
    });
    
    // Admin dashboard
    router.get("/admin", |_req| {
        let posts = Post::all();
        let html = dashboard(&posts);
        Ok(Response::ok().html(html))
    });
    
    // New post form
    router.get("/admin/posts/new", |_req| {
        Ok(Response::ok().html(post_form(None)))
    });
    
    // Edit post form
    router.get("/admin/posts/{id}/edit", |req: Request| {
        let id: i64 = req.param("id").unwrap_or("0").parse().unwrap_or(0);
        match Post::find(id) {
            Some(post) => Ok(Response::ok().html(post_form(Some(&post)))),
            None => Ok(Response::new(StatusCode::NOT_FOUND).html("<h1>Post not found</h1>")),
        }
    });
    
    // Create post (POST)
    router.post("/admin/posts", |req: Request| {
        let form = req.form();
        let title = form.get("title").cloned().unwrap_or_default();
        let content = form.get("content").cloned().unwrap_or_default();
        let author = form.get("author").cloned().unwrap_or_else(|| "Admin".to_string());
        
        if title.is_empty() || content.is_empty() {
            return Ok(Response::new(StatusCode::BAD_REQUEST).html(
                "<h1>Error: Title and content are required</h1><a href='/admin/posts/new'>Try again</a>"
            ));
        }
        
        let post = Post::create(&title, &content, &author);
        
        Ok(Response::redirect(&format!("/blog/{}", post.id)))
    });
    
    // Update post (PUT)
    router.post("/admin/posts/{id}", |req: Request| {
        let id: i64 = req.param("id").unwrap_or("0").parse().unwrap_or(0);
        let form = req.form();
        
        match Post::find(id) {
            Some(mut post) => {
                if let Some(title) = form.get("title") {
                    post.title = title.clone();
                }
                if let Some(content) = form.get("content") {
                    post.content = content.clone();
                }
                if let Some(author) = form.get("author") {
                    post.author = author.clone();
                }
                post.updated_at = chrono::Utc::now().to_rfc3339();
                post.save();
                
                Ok(Response::redirect(&format!("/blog/{}", post.id)))
            }
            None => Ok(Response::new(StatusCode::NOT_FOUND).html("<h1>Post not found</h1>")),
        }
    });
    
    // Delete post confirmation
    router.get("/admin/posts/{id}/delete", |req: Request| {
        let id: i64 = req.param("id").unwrap_or("0").parse().unwrap_or(0);
        
        match Post::find(id) {
            Some(post) => {
                let html = format!(
                    r#"<html>
<head><title>Delete Post - Noor Admin</title>
<style>
    body {{ font-family: -apple-system, sans-serif; max-width: 600px; margin: 50px auto; padding: 20px; }}
    .warning {{ background: #fff3cd; border: 1px solid #ffeaa7; padding: 20px; border-radius: 8px; margin: 20px 0; }}
    .btn {{ display: inline-block; padding: 10px 20px; border-radius: 4px; text-decoration: none; color: white; margin-right: 10px; }}
    .btn-danger {{ background: #e74c3c; }}
    .btn-cancel {{ background: #95a5a6; }}
</style>
</head>
<body>
    <h1>Delete Post</h1>
    <div class="warning">
        <strong>⚠️ Warning:</strong> Are you sure you want to delete "<strong>{}</strong>"?
        This action cannot be undone.
    </div>
    <form method="POST" action="/admin/posts/{}/delete">
        <a href="/admin" class="btn btn-cancel">Cancel</a>
        <button type="submit" class="btn btn-danger" style="border:none;cursor:pointer;">Yes, Delete</button>
    </form>
</body>
</html>"#,
                    crate::core::security::Xss::escape(&post.title),
                    id
                );
                Ok(Response::ok().html(html))
            }
            None => Ok(Response::new(StatusCode::NOT_FOUND).html("<h1>Post not found</h1>")),
        }
    });
    
    // Delete post (POST)
    router.post("/admin/posts/{id}/delete", |req: Request| {
        let id: i64 = req.param("id").unwrap_or("0").parse().unwrap_or(0);
        Post::delete(id);
        Ok(Response::redirect("/admin"))
    });
}

/// Admin login page
fn login_page() -> String {
    r#"<html>
<head>
    <title>Login - Noor Admin</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
               background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
               min-height: 100vh; display: flex; align-items: center; justify-content: center; }
        .login-card { background: white; padding: 40px; border-radius: 12px;
                      box-shadow: 0 20px 40px rgba(0,0,0,0.1); width: 100%; max-width: 400px; }
        .login-card h1 { color: #2c3e50; margin-bottom: 30px; text-align: center; font-size: 24px; }
        .form-group { margin-bottom: 20px; }
        .form-group label { display: block; margin-bottom: 5px; color: #34495e; font-weight: 500; }
        .form-group input { width: 100%; padding: 12px; border: 2px solid #ecf0f1; border-radius: 6px;
                            font-size: 16px; transition: border-color 0.3s; }
        .form-group input:focus { outline: none; border-color: #3498db; }
        .btn { width: 100%; padding: 14px; background: #3498db; color: white; border: none;
               border-radius: 6px; font-size: 16px; font-weight: 600; cursor: pointer; transition: background 0.3s; }
        .btn:hover { background: #2980b9; }
        .footer { text-align: center; margin-top: 20px; color: #7f8c8d; font-size: 14px; }
    </style>
</head>
<body>
    <div class="login-card">
        <h1>🔐 Noor Admin</h1>
        <form method="POST" action="/admin/login">
            <div class="form-group">
                <label>Email</label>
                <input type="email" name="email" placeholder="admin@example.com" required>
            </div>
            <div class="form-group">
                <label>Password</label>
                <input type="password" name="password" placeholder="••••••••" required>
            </div>
            <button type="submit" class="btn">Sign In</button>
        </form>
        <div class="footer">Noor Framework v1.0.0</div>
    </div>
</body>
</html>"#.to_string()
}

/// Admin dashboard
fn dashboard(posts: &[Post]) -> String {
    let total_posts = posts.len();
    
    let posts_html = posts.iter().map(|p| format!(
        r#"<tr>
            <td>{}</td>
            <td><a href="/blog/{}" style="color:#3498db;text-decoration:none;">{}</a></td>
            <td>{}</td>
            <td>{}</td>
            <td class="actions">
                <a href="/admin/posts/{}/edit" class="btn-edit">Edit</a>
                <a href="/admin/posts/{}/delete" class="btn-delete">Delete</a>
            </td>
        </tr>"#,
        p.id, p.id, crate::core::security::Xss::escape(&p.title),
        crate::core::security::Xss::escape(&p.author),
        p.created_at.split('T').next().unwrap_or(""),
        p.id, p.id
    )).collect::<Vec<_>>().join("\n            ");
    
    format!(
        r#"<html>
<head>
    <title>Admin Dashboard - Noor</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{ font-family: -apple-system, sans-serif; background: #f5f7fa; color: #333; }}
        .header {{ background: linear-gradient(135deg, #2c3e50 0%, #3498db 100%); color: white;
                  padding: 20px 40px; display: flex; justify-content: space-between; align-items: center; }}
        .header h1 {{ font-size: 22px; }}
        .header a {{ color: white; text-decoration: none; margin-left: 20px; }}
        .container {{ max-width: 1200px; margin: 30px auto; padding: 0 20px; }}
        .stats {{ display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 20px; margin-bottom: 30px; }}
        .stat-card {{ background: white; padding: 25px; border-radius: 10px; box-shadow: 0 2px 8px rgba(0,0,0,0.05); }}
        .stat-card h3 {{ color: #7f8c8d; font-size: 14px; text-transform: uppercase; margin-bottom: 10px; }}
        .stat-card .number {{ font-size: 32px; font-weight: 700; color: #2c3e50; }}
        .table-container {{ background: white; border-radius: 10px; overflow: hidden; box-shadow: 0 2px 8px rgba(0,0,0,0.05); }}
        table {{ width: 100%; border-collapse: collapse; }}
        th {{ background: #f8f9fa; padding: 15px; text-align: left; color: #2c3e50; font-weight: 600; border-bottom: 2px solid #ecf0f1; }}
        td {{ padding: 15px; border-bottom: 1px solid #ecf0f1; }}
        tr:hover {{ background: #f8f9fa; }}
        .actions a {{ display: inline-block; padding: 6px 12px; border-radius: 4px; text-decoration: none; font-size: 13px; margin-right: 5px; }}
        .btn-edit {{ background: #3498db; color: white; }}
        .btn-delete {{ background: #e74c3c; color: white; }}
        .new-btn {{ display: inline-block; padding: 10px 20px; background: #27ae60; color: white;
                   text-decoration: none; border-radius: 6px; margin-bottom: 20px; font-weight: 600; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>📊 Noor Admin Dashboard</h1>
        <div>
            <a href="/">View Site</a>
            <a href="/blog">Blog</a>
            <a href="/admin/login">Logout</a>
        </div>
    </div>
    
    <div class="container">
        <div class="stats">
            <div class="stat-card">
                <h3>Total Posts</h3>
                <div class="number">{}</div>
            </div>
            <div class="stat-card">
                <h3>Published</h3>
                <div class="number">{}</div>
            </div>
            <div class="stat-card">
                <h3>Drafts</h3>
                <div class="number">0</div>
            </div>
            <div class="stat-card">
                <h3>Comments</h3>
                <div class="number">0</div>
            </div>
        </div>
        
        <a href="/admin/posts/new" class="new-btn">+ New Post</a>
        
        <div class="table-container">
            <table>
                <thead>
                    <tr>
                        <th>ID</th>
                        <th>Title</th>
                        <th>Author</th>
                        <th>Date</th>
                        <th>Actions</th>
                    </tr>
                </thead>
                <tbody>
            {}
                </tbody>
            </table>
        </div>
    </div>
</body>
</html>"#,
        total_posts, total_posts, posts_html
    )
}

/// Post create/edit form
fn post_form(post: Option<&Post>) -> String {
    let (title_val, content_val, author_val, action, h1) = match post {
        Some(p) => (
            crate::core::security::Xss::escape(&p.title),
            crate::core::security::Xss::escape(&p.content),
            crate::core::security::Xss::escape(&p.author),
            format!("/admin/posts/{}", p.id),
            "Edit Post".to_string(),
        ),
        None => (
            "".to_string(),
            "".to_string(),
            "Admin".to_string(),
            "/admin/posts".to_string(),
            "New Post".to_string(),
        ),
    };
    
    format!(
        r#"<html>
<head>
    <title>{} - Noor Admin</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{ font-family: -apple-system, sans-serif; background: #f5f7fa; color: #333; }}
        .header {{ background: linear-gradient(135deg, #2c3e50 0%, #3498db 100%); color: white;
                  padding: 20px 40px; }}
        .header a {{ color: white; text-decoration: none; margin-right: 20px; }}
        .container {{ max-width: 800px; margin: 30px auto; padding: 0 20px; }}
        .form-card {{ background: white; padding: 30px; border-radius: 10px; box-shadow: 0 2px 8px rgba(0,0,0,0.05); }}
        h1 {{ margin-bottom: 20px; color: #2c3e50; }}
        .form-group {{ margin-bottom: 20px; }}
        .form-group label {{ display: block; margin-bottom: 5px; font-weight: 500; color: #34495e; }}
        .form-group input, .form-group textarea {{ width: 100%; padding: 12px; border: 2px solid #ecf0f1;
            border-radius: 6px; font-size: 16px; font-family: inherit; transition: border-color 0.3s; }}
        .form-group input:focus, .form-group textarea:focus {{ outline: none; border-color: #3498db; }}
        .form-group textarea {{ min-height: 200px; resize: vertical; }}
        .btn-group {{ display: flex; gap: 10px; }}
        .btn {{ padding: 12px 24px; border: none; border-radius: 6px; font-size: 16px; font-weight: 600;
               cursor: pointer; text-decoration: none; display: inline-block; text-align: center; }}
        .btn-primary {{ background: #3498db; color: white; }}
        .btn-cancel {{ background: #95a5a6; color: white; }}
    </style>
</head>
<body>
    <div class="header">
        <a href="/admin">← Dashboard</a>
        <a href="/blog">View Blog</a>
    </div>
    
    <div class="container">
        <div class="form-card">
            <h1>{}</h1>
            <form method="POST" action="{}">
                <div class="form-group">
                    <label for="title">Title</label>
                    <input type="text" id="title" name="title" value="{}" required placeholder="Enter post title">
                </div>
                <div class="form-group">
                    <label for="author">Author</label>
                    <input type="text" id="author" name="author" value="{}" placeholder="Author name">
                </div>
                <div class="form-group">
                    <label for="content">Content</label>
                    <textarea id="content" name="content" required placeholder="Write your post content here...">{}</textarea>
                </div>
                <div class="btn-group">
                    <button type="submit" class="btn btn-primary">Save Post</button>
                    <a href="/admin" class="btn btn-cancel">Cancel</a>
                </div>
            </form>
        </div>
    </div>
</body>
</html>"#,
        h1, h1, action, title_val, author_val, content_val
    )
}
