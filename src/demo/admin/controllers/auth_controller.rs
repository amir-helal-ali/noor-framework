// ============================================================
// Admin Auth Controller - متحكم مصادقة لوحة التحكم
// ============================================================

use crate::core::http::{Request, Response, StatusCode};
use crate::core::security::Encryption;
use crate::NoorResult;

/// Admin user (in-memory for demo)
/// مستخدم لوحة التحكم
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminUser {
    pub id: i64,
    pub email: String,
    pub name: String,
    pub password_hash: String,
    pub role: String,
}

impl AdminUser {
    /// Default admin user for demo
    pub fn default_admin() -> Self {
        Self {
            id: 1,
            email: "admin@noor.dev".to_string(),
            name: "Administrator".to_string(),
            password_hash: Encryption::hash_password("admin123").unwrap_or_default(),
            role: "admin".to_string(),
        }
    }
    
    /// Verify password
    pub fn verify_password(&self, password: &str) -> bool {
        Encryption::verify_password(password, &self.password_hash)
    }
}

/// Handle admin login
/// معالجة تسجيل الدخول
pub fn login(req: Request) -> NoorResult<Response> {
    let form = req.form();
    
    let email = form.get("email").cloned().unwrap_or_default();
    let password = form.get("password").cloned().unwrap_or_default();
    
    let admin = AdminUser::default_admin();
    
    if email == admin.email && admin.verify_password(&password) {
        // In a real app, we'd create a session/JWT here
        Ok(Response::redirect("/admin")
            .cookie("noor_admin", &admin.id.to_string(), 3600))
    } else {
        Ok(Response::new(StatusCode::UNAUTHORIZED).html(
            r#"<html><body>
<h1>Invalid credentials</h1>
<p>Email: admin@noor.dev</p>
<p>Password: admin123</p>
<p><a href="/admin/login">Try again</a></p>
</body></html>"#
        ))
    }
}

/// Handle admin logout
pub fn logout(_req: Request) -> NoorResult<Response> {
    Ok(Response::redirect("/admin/login"))
}
