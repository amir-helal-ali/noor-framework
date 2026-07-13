// ============================================================
// Auth Middleware - middleware المصادقة
// ============================================================

use std::sync::Arc;
use crate::core::http::{Request, Response, StatusCode};
use crate::core::auth::Jwt;

/// Authentication middleware
pub struct AuthMiddleware {
    jwt: Arc<Jwt>,
}

impl AuthMiddleware {
    pub fn new(jwt: Arc<Jwt>) -> Self {
        Self { jwt }
    }
    
    /// Verify the request is authenticated
    /// التحقق من أن الطلب مصادق
    pub fn authenticate(&self, request: &Request) -> AuthResult {
        let token = match request.bearer_token() {
            Some(t) => t,
            None => return AuthResult::unauthorized("Missing authentication token"),
        };
        
        match self.jwt.verify(token) {
            Ok(claims) => {
                if claims.typ != "access" {
                    return AuthResult::unauthorized("Invalid token type");
                }
                AuthResult::authenticated(claims.sub, claims.roles)
            }
            Err(e) => AuthResult::unauthorized(&e.to_string()),
        }
    }
    
    /// Require authentication - returns user ID or error response
    pub fn require_auth(&self, request: &Request) -> Result<AuthInfo, Response> {
        match self.authenticate(request) {
            AuthResult::Authenticated(info) => Ok(info),
            AuthResult::Unauthorized(message) => Err(Self::unauthorized_response(&message)),
        }
    }
    
    /// Require a specific permission
    pub fn require_permission(&self, request: &Request, permission: &str) -> Result<AuthInfo, Response> {
        let info = self.require_auth(request)?;
        
        // In a full implementation, check RBAC here
        // For now, we just check if user has any of the required roles
        // rbac.can(&info.user_id, permission)
        
        Ok(info)
    }
    
    /// Create an unauthorized response
    pub fn unauthorized_response(message: &str) -> Response {
        Response::new(StatusCode::UNAUTHORIZED)
            .json(&serde_json::json!({
                "error": "Unauthorized",
                "message": message,
            }))
            .unwrap()
    }
    
    /// Create a forbidden response
    pub fn forbidden_response(message: &str) -> Response {
        Response::new(StatusCode::FORBIDDEN)
            .json(&serde_json::json!({
                "error": "Forbidden",
                "message": message,
            }))
            .unwrap()
    }
}

/// Result of authentication check
pub enum AuthResult {
    Authenticated(AuthInfo),
    Unauthorized(String),
}

/// Authentication information
#[derive(Debug, Clone)]
pub struct AuthInfo {
    pub user_id: String,
    pub roles: Vec<String>,
}

impl AuthResult {
    fn authenticated(user_id: String, roles: Vec<String>) -> Self {
        Self::Authenticated(AuthInfo { user_id, roles })
    }

    fn unauthorized(message: &str) -> Self {
        Self::Unauthorized(message.to_string())
    }
}

/// Register a JWT auth middleware under the given name in a `Router`'s
/// middleware stack. Routes can then require it via
/// `router.get("/protected", handler).middleware("auth")` (once route-level
/// middleware selection is exposed) — or you can use `require_auth` inside
/// the handler.
///
/// Usage:
/// ```ignore
/// let jwt = Arc::new(Jwt::new(&secret, "noor", "noor_app"));
/// noor::core::middleware::auth::register(&mut router, jwt, "auth");
/// router.use_middleware("auth");
/// ```
pub fn register(
    router: &mut crate::core::router::Router,
    jwt: Arc<Jwt>,
    name: &str,
) {
    let jwt_for_mw = jwt.clone();
    router.middleware_stack().register(
        name,
        std::sync::Arc::new(move |request| {
            let mw = AuthMiddleware::new(jwt_for_mw.clone());
            match mw.authenticate(&request) {
                AuthResult::Authenticated(_) => Ok(crate::core::middleware::MiddlewareOutcome::Continue(request)),
                AuthResult::Unauthorized(msg) => {
                    Ok(crate::core::middleware::MiddlewareOutcome::Stop(
                        AuthMiddleware::unauthorized_response(&msg),
                    ))
                }
            }
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_unauthorized_no_token() {
        let jwt = Arc::new(Jwt::new("secret", "noor", "noor_app"));
        let middleware = AuthMiddleware::new(jwt);
        
        let request = Request::new(crate::core::http::Method::Get, "/".to_string());
        
        match middleware.authenticate(&request) {
            AuthResult::Unauthorized(_) => {} // Expected
            _ => panic!("Expected Unauthorized"),
        }
    }
    
    #[test]
    fn test_authenticated_with_valid_token() {
        let jwt = Arc::new(Jwt::new("secret", "noor", "noor_app"));
        let middleware = AuthMiddleware::new(jwt.clone());
        
        let token = jwt.generate_access_token("user123", vec!["admin".to_string()]).unwrap();
        
        let mut request = Request::new(crate::core::http::Method::Get, "/".to_string());
        request.headers.insert(
            "authorization".to_string(),
            format!("Bearer {}", token),
        );
        
        match middleware.authenticate(&request) {
            AuthResult::Authenticated(info) => {
                assert_eq!(info.user_id, "user123");
                assert!(info.roles.contains(&"admin".to_string()));
            }
            _ => panic!("Expected Authenticated"),
        }
    }
}
