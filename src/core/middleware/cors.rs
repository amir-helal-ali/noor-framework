// ============================================================
// CORS Middleware - middleware CORS
// ============================================================

use crate::core::http::{Request, Response, StatusCode};

/// CORS middleware configuration
/// إعدادات middleware CORS
#[derive(Debug, Clone)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
    pub allowed_methods: Vec<String>,
    pub allowed_headers: Vec<String>,
    pub exposed_headers: Vec<String>,
    pub allow_credentials: bool,
    pub max_age: u64,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec![
                "GET".to_string(), "POST".to_string(), "PUT".to_string(),
                "PATCH".to_string(), "DELETE".to_string(), "OPTIONS".to_string(),
            ],
            allowed_headers: vec![
                "Content-Type".to_string(),
                "Authorization".to_string(),
                "X-Requested-With".to_string(),
                "X-CSRF-Token".to_string(),
            ],
            exposed_headers: vec![],
            allow_credentials: false,
            max_age: 86400,
        }
    }
}

/// CORS middleware
pub struct CorsMiddleware {
    config: CorsConfig,
}

impl CorsMiddleware {
    pub fn new(config: CorsConfig) -> Self {
        Self { config }
    }
    
    /// Add CORS headers to a response
    /// إضافة headers CORS للاستجابة
    pub fn apply_headers(&self, request: &Request, response: &mut Response) {
        // Handle preflight OPTIONS
        if request.method == crate::core::http::Method::Options {
            response.status = StatusCode::NO_CONTENT;
        }
        
        // Set Access-Control-Allow-Origin
        let origin = request.header("origin").unwrap_or("*");
        let allowed_origin = if self.config.allowed_origins.contains(&"*".to_string()) {
            "*"
        } else if self.config.allowed_origins.contains(&origin.to_string()) {
            origin
        } else {
            return; // Origin not allowed
        };
        
        response.headers.insert(
            "access-control-allow-origin".to_string(),
            allowed_origin.to_string(),
        );
        
        // Set Access-Control-Allow-Methods
        response.headers.insert(
            "access-control-allow-methods".to_string(),
            self.config.allowed_methods.join(", "),
        );
        
        // Set Access-Control-Allow-Headers
        response.headers.insert(
            "access-control-allow-headers".to_string(),
            self.config.allowed_headers.join(", "),
        );
        
        // Set Access-Control-Expose-Headers
        if !self.config.exposed_headers.is_empty() {
            response.headers.insert(
                "access-control-expose-headers".to_string(),
                self.config.exposed_headers.join(", "),
            );
        }
        
        // Set Access-Control-Allow-Credentials
        if self.config.allow_credentials {
            response.headers.insert(
                "access-control-allow-credentials".to_string(),
                "true".to_string(),
            );
        }
        
        // Set Access-Control-Max-Age
        response.headers.insert(
            "access-control-max-age".to_string(),
            self.config.max_age.to_string(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cors_default() {
        let config = CorsConfig::default();
        assert!(config.allowed_origins.contains(&"*".to_string()));
    }
    
    #[test]
    fn test_cors_apply_headers() {
        let middleware = CorsMiddleware::new(CorsConfig::default());
        let request = Request::new(crate::core::http::Method::Get, "/".to_string());
        let mut response = Response::ok();
        
        middleware.apply_headers(&request, &mut response);
        
        assert!(response.headers.contains_key("access-control-allow-origin"));
    }
}
