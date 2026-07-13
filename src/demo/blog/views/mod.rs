// ============================================================
// Blog Views - قوالب المدونة
// ============================================================
// This module contains template helpers for the blog demo.
// يحتوي على مساعدات القوالب لعرض المدونة.
// ============================================================

/// Generate a styled blog post card
pub fn post_card(post: &crate::demo::blog::models::Post) -> String {
    format!(
        r#"<div class="post-card" style="background:#fff;padding:20px;margin:15px 0;border-radius:8px;box-shadow:0 2px 4px rgba(0,0,0,0.1);">
    <h3 style="margin-top:0;color:#2c3e50;">
        <a href="/blog/{}" style="text-decoration:none;color:#2c3e50;">{}</a>
    </h3>
    <p style="color:#7f8c8d;font-size:0.9em;">By {} • {}</p>
    <p style="color:#34495e;">{}</p>
</div>"#,
        post.id,
        crate::core::security::Xss::escape(&post.title),
        crate::core::security::Xss::escape(&post.author),
        post.created_at,
        crate::core::security::Xss::escape(&post.excerpt())
    )
}
