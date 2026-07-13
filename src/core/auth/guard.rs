// ============================================================
// Auth Guard - حارس المصادقة
// ============================================================
// Middleware-like guards for protecting routes.
// Used to check authentication and authorization before
// allowing access to a route handler.
//
// حراس لحماية المسارات.
// ============================================================

use crate::core::http::{Request, Response, StatusCode};
use crate::NoorResult;
use crate::core::auth::Jwt;

/// Authentication guard
/// حارس المصادقة
#[derive(Clone)]
pub struct Guard {
    jwt: std::sync::Arc<Jwt>,
}

impl Guard {
    pub fn new(jwt: std::sync::Arc<Jwt>) -> Self {
        Self { jwt }
    }
    
    /// Require authentication - returns user ID if authenticated
    /// يتطلب مصادقة - يرجع ID المستخدم إذا كان مصادق
    pub fn authenticate(&self, request: &Request) -> crate::NoorResult<String> {
        let token = request
            .bearer_token()
            .ok_or_else(|| crate::NoorError::Auth("Missing authentication token".to_string()))?;
        
        let claims = self.jwt.verify(token)?;
        
        if claims.typ != "access" {
            return Err(crate::NoorError::Auth("Invalid token type".to_string()));
        }
        
        Ok(claims.sub)
    }
    
    /// Require a specific permission
    /// يتطلب صلاحية محددة
    pub fn authorize(&self, request: &Request, permission: &str) -> crate::NoorResult<String> {
        let user_id = self.authenticate(request)?;
        
        // In a full implementation, we'd check RBAC here
        // For now, we just return the user ID
        // The application layer should call rbac.can(user_id, permission)
        
        Ok(user_id)
    }
    
    /// Create an unauthorized response
    /// إنشاء استجابة غير مصرح
    pub fn unauthorized(message: &str) -> Response {
        Response::new(StatusCode::UNAUTHORIZED)
            .json(&serde_json::json!({
                "error": "Unauthorized",
                "message": message,
            }))
            .unwrap()
    }
    
    /// Create a forbidden response
    /// إنشاء استجابة ممنوعة
    pub fn forbidden(message: &str) -> Response {
        Response::new(StatusCode::FORBIDDEN)
            .json(&serde_json::json!({
                "error": "Forbidden",
                "message": message,
            }))
            .unwrap()
    }
}

/// Convenience function to create an auth middleware
/// دالة ملائمة لإنشاء middleware مصادقة
pub fn require_auth(jwt: std::sync::Arc<Jwt>) -> impl Fn(&Request) -> crate::NoorResult<()> + Clone {
    let guard = Guard::new(jwt);
    move |request: &Request| {
        guard.authenticate(request).map(|_| ())
    }
}
