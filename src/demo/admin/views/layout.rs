// ============================================================
// Admin Layout - تخطيط لوحة التحكم
// ============================================================

/// Base HTML layout for admin pages
pub fn admin_layout(title: &str, content: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="ar" dir="rtl">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{} - Noor Admin</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Tahoma, sans-serif;
               background: #f5f7fa; color: #333; }}
        .header {{ background: linear-gradient(135deg, #2c3e50 0%, #3498db 100%);
                  color: white; padding: 20px 40px; display: flex; justify-content: space-between; }}
        .header h1 {{ font-size: 22px; }}
        .nav {{ margin-top: 10px; }}
        .nav a {{ color: white; text-decoration: none; margin-left: 20px; opacity: 0.9; }}
        .nav a:hover {{ opacity: 1; }}
        .container {{ max-width: 1200px; margin: 30px auto; padding: 0 20px; }}
        .card {{ background: white; border-radius: 10px; padding: 25px; margin-bottom: 20px;
                box-shadow: 0 2px 8px rgba(0,0,0,0.05); }}
    </style>
</head>
<body>
    <div class="header">
        <div>
            <h1>✨ Noor Admin</h1>
            <div class="nav">
                <a href="/admin">Dashboard</a>
                <a href="/admin/posts">Posts</a>
                <a href="/">View Site</a>
                <a href="/admin/logout">Logout</a>
            </div>
        </div>
    </div>
    <div class="container">
        {}
    </div>
</body>
</html>"#,
        crate::core::security::Xss::escape(title),
        content
    )
}
