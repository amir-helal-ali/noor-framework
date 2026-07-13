// ============================================================
// API Gateway - بوابة API
// ============================================================
// Single entry point for all microservices.
// Handles routing, authentication, rate limiting, and more.
//
// نقطة دخول واحدة لجميع الـ microservices.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

use crate::core::http::{Request, Response, StatusCode, Method};
use crate::core::service_discovery::{ServiceRegistry, LoadBalancer, LoadBalancerStrategy};

/// Route configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayRoute {
    pub path_prefix: String,
    pub service_name: String,
    pub strip_prefix: bool,
    pub methods: Vec<String>,
    pub auth_required: bool,
    pub rate_limit: Option<u32>,
    pub timeout_secs: Option<u64>,
    pub retries: u32,
}

impl GatewayRoute {
    pub fn new(path_prefix: &str, service_name: &str) -> Self {
        Self {
            path_prefix: path_prefix.to_string(),
            service_name: service_name.to_string(),
            strip_prefix: false,
            methods: vec!["GET".to_string(), "POST".to_string(), "PUT".to_string(), 
                          "PATCH".to_string(), "DELETE".to_string()],
            auth_required: false,
            rate_limit: None,
            timeout_secs: Some(30),
            retries: 0,
        }
    }
    
    pub fn strip_prefix(mut self) -> Self {
        self.strip_prefix = true;
        self
    }
    
    pub fn methods(mut self, methods: Vec<&str>) -> Self {
        self.methods = methods.iter().map(|s| s.to_string()).collect();
        self
    }
    
    pub fn require_auth(mut self) -> Self {
        self.auth_required = true;
        self
    }
    
    pub fn rate_limit(mut self, requests_per_minute: u32) -> Self {
        self.rate_limit = Some(requests_per_minute);
        self
    }
    
    pub fn timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = Some(secs);
        self
    }
    
    pub fn retries(mut self, count: u32) -> Self {
        self.retries = count;
        self
    }
    
    /// Check if this route matches a request
    pub fn matches(&self, path: &str, method: &Method) -> bool {
        if !path.starts_with(&self.path_prefix) {
            return false;
        }
        
        let method_str = method.as_str();
        self.methods.iter().any(|m| m == method_str)
    }
    
    /// Get the target path (with optional prefix stripping)
    pub fn target_path(&self, original_path: &str) -> String {
        if self.strip_prefix {
            original_path.strip_prefix(&self.path_prefix).unwrap_or(original_path).to_string()
        } else {
            original_path.to_string()
        }
    }
}

/// API Gateway
pub struct ApiGateway {
    routes: Arc<RwLock<Vec<GatewayRoute>>>,
    registry: Arc<ServiceRegistry>,
    load_balancer: Arc<LoadBalancer>,
    /// Whether auth is required for all routes
    global_auth: Arc<RwLock<bool>>,
}

impl ApiGateway {
    pub fn new(registry: Arc<ServiceRegistry>) -> Self {
        let load_balancer = Arc::new(LoadBalancer::new(registry.clone(), LoadBalancerStrategy::RoundRobin));
        
        Self {
            routes: Arc::new(RwLock::new(Vec::new())),
            registry,
            load_balancer,
            global_auth: Arc::new(RwLock::new(false)),
        }
    }
    
    /// Add a route
    pub fn route(&self, route: GatewayRoute) -> &Self {
        self.routes.write().push(route);
        self
    }
    
    /// Enable global authentication
    pub fn require_global_auth(&self) {
        *self.global_auth.write() = true;
    }
    
    /// Process an incoming request
    pub fn process(&self, request: &Request) -> crate::NoorResult<Response> {
        let routes = self.routes.read();
        
        // Find matching route
        let route = routes.iter().find(|r| r.matches(&request.path, &request.method));
        
        let route = match route {
            Some(r) => r,
            None => {
                return Ok(Response::new(StatusCode::NOT_FOUND)
                    .json(&serde_json::json!({
                        "error": "Route not found",
                        "path": request.path,
                    }))?);
            }
        };
        
        // Check authentication
        let auth_required = *self.global_auth.read() || route.auth_required;
        
        if auth_required && request.bearer_token().is_none() {
            return Ok(Response::new(StatusCode::UNAUTHORIZED)
                .json(&serde_json::json!({
                    "error": "Authentication required",
                }))?);
        }
        
        // Discover service instance
        let instance = match self.load_balancer.next(&route.service_name) {
            Some(inst) => inst,
            None => {
                return Ok(Response::new(StatusCode::SERVICE_UNAVAILABLE)
                    .json(&serde_json::json!({
                        "error": "Service unavailable",
                        "service": route.service_name,
                    }))?);
            }
        };
        
        // Get target path
        let target_path = route.target_path(&request.path);
        
        // Build target URL
        let target_url = instance.url_path(&target_path);
        
        tracing::info!(
            "Gateway routing: {} {} -> {} ({})",
            request.method,
            request.path,
            target_url,
            instance.id
        );
        
        // In a real implementation, we would:
        // 1. Make an HTTP request to the target service
        // 2. Apply timeout
        // 3. Retry on failure
        // 4. Return the response
        
        // For now, return a simulated response
        Ok(Response::ok()
            .json(&serde_json::json!({
                "gateway": true,
                "service": route.service_name,
                "instance": instance.id,
                "target_url": target_url,
                "method": request.method.as_str(),
                "original_path": request.path,
                "target_path": target_path,
            }))?)
    }
    
    /// Get all routes
    pub fn routes(&self) -> Vec<GatewayRoute> {
        self.routes.read().clone()
    }
    
    /// Get the count of routes
    pub fn route_count(&self) -> usize {
        self.routes.read().len()
    }
    
    /// Remove a route by path prefix
    pub fn remove_route(&self, path_prefix: &str) -> bool {
        let mut routes = self.routes.write();
        let initial = routes.len();
        routes.retain(|r| r.path_prefix != path_prefix);
        routes.len() < initial
    }
}

/// Gateway middleware
pub trait GatewayMiddleware: Send + Sync {
    /// Process request before forwarding
    fn before(&self, request: &mut Request, route: &GatewayRoute) -> crate::NoorResult<()>;
    
    /// Process response after receiving
    fn after(&self, response: &mut Response, route: &GatewayRoute) -> crate::NoorResult<()>;
    
    /// Get middleware name
    fn name(&self) -> &str;
}

/// Logging middleware
pub struct LoggingMiddleware;

impl GatewayMiddleware for LoggingMiddleware {
    fn before(&self, request: &mut Request, route: &GatewayRoute) -> crate::NoorResult<()> {
        tracing::info!(
            "Gateway request: {} {} -> {}",
            request.method,
            request.path,
            route.service_name
        );
        Ok(())
    }
    
    fn after(&self, response: &mut Response, _route: &GatewayRoute) -> crate::NoorResult<()> {
        tracing::info!("Gateway response: {}", response.status.0);
        Ok(())
    }
    
    fn name(&self) -> &str {
        "logging"
    }
}

/// Rate limiting middleware
pub struct RateLimitMiddleware {
    limiter: Arc<crate::core::security::RateLimit>,
}

impl RateLimitMiddleware {
    pub fn new(max_requests: u32, window_secs: u64) -> Self {
        Self {
            limiter: Arc::new(crate::core::security::RateLimit::new(max_requests, window_secs)),
        }
    }
}

impl GatewayMiddleware for RateLimitMiddleware {
    fn before(&self, request: &mut Request, _route: &GatewayRoute) -> crate::NoorResult<()> {
        let ip = request.client_ip.as_deref().unwrap_or("unknown");
        
        if !self.limiter.check(ip).allowed {
            return Err(crate::NoorError::Security("Rate limit exceeded".to_string()));
        }
        
        Ok(())
    }
    
    fn after(&self, _response: &mut Response, _route: &GatewayRoute) -> crate::NoorResult<()> {
        Ok(())
    }
    
    fn name(&self) -> &str {
        "rate_limit"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ServiceInstance;
    
    #[test]
    fn test_route_matching() {
        let route = GatewayRoute::new("/api/users", "user-service")
            .methods(vec!["GET", "POST"]);
        
        assert!(route.matches("/api/users", &Method::Get));
        assert!(route.matches("/api/users/123", &Method::Get));
        assert!(route.matches("/api/users", &Method::Post));
        assert!(!route.matches("/api/users", &Method::Delete));
        assert!(!route.matches("/api/posts", &Method::Get));
    }
    
    #[test]
    fn test_route_strip_prefix() {
        let route = GatewayRoute::new("/api/users", "user-service")
            .strip_prefix();
        
        assert_eq!(route.target_path("/api/users/123"), "/123");
        assert_eq!(route.target_path("/api/users"), "");
    }
    
    #[test]
    fn test_gateway_process() {
        let registry = Arc::new(ServiceRegistry::default());
        registry.register(ServiceInstance::new("user-service", "localhost", 8081));
        
        let gateway = ApiGateway::new(registry);
        gateway.route(GatewayRoute::new("/api/users", "user-service"));
        
        let request = Request::new(Method::Get, "/api/users".to_string());
        let response = gateway.process(&request).unwrap();
        
        assert_eq!(response.status.0, 200);
    }
    
    #[test]
    fn test_gateway_route_not_found() {
        let registry = Arc::new(ServiceRegistry::default());
        let gateway = ApiGateway::new(registry);
        
        let request = Request::new(Method::Get, "/unknown".to_string());
        let response = gateway.process(&request).unwrap();
        
        assert_eq!(response.status.0, 404);
    }
    
    #[test]
    fn test_gateway_auth_required() {
        let registry = Arc::new(ServiceRegistry::default());
        registry.register(ServiceInstance::new("api", "localhost", 8080));
        
        let gateway = ApiGateway::new(registry);
        gateway.route(GatewayRoute::new("/api", "api").require_auth());
        
        let request = Request::new(Method::Get, "/api/data".to_string());
        let response = gateway.process(&request).unwrap();
        
        assert_eq!(response.status.0, 401);
    }
    
    #[test]
    fn test_gateway_service_unavailable() {
        let registry = Arc::new(ServiceRegistry::default());
        
        let gateway = ApiGateway::new(registry);
        gateway.route(GatewayRoute::new("/api", "nonexistent-service"));
        
        let request = Request::new(Method::Get, "/api/data".to_string());
        let response = gateway.process(&request).unwrap();
        
        assert_eq!(response.status.0, 503);
    }
}
