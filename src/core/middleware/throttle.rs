// ============================================================
// Throttle Middleware - تحديد المعدل
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::Mutex;
use crate::core::http::{Request, Response, StatusCode};

/// Throttle middleware configuration
#[derive(Debug, Clone)]
pub struct ThrottleConfig {
    pub max_requests: u32,
    pub window_secs: u64,
    /// Optional: only throttle specific paths (regex patterns)
    pub paths: Option<Vec<String>>,
    /// Optional: exclude specific IPs
    pub excluded_ips: Vec<String>,
}

impl Default for ThrottleConfig {
    fn default() -> Self {
        Self {
            max_requests: 60,
            window_secs: 60,
            paths: None,
            excluded_ips: vec!["127.0.0.1".to_string(), "::1".to_string()],
        }
    }
}

/// Throttle middleware using sliding window
pub struct ThrottleMiddleware {
    config: ThrottleConfig,
    buckets: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
}

impl ThrottleMiddleware {
    pub fn new(config: ThrottleConfig) -> Self {
        Self {
            config,
            buckets: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Check if request should be allowed
    pub fn check(&self, request: &Request) -> ThrottleResult {
        let ip = request.client_ip.as_deref().unwrap_or("unknown");
        
        // Skip excluded IPs
        if self.config.excluded_ips.iter().any(|excluded| excluded == ip) {
            return ThrottleResult::allowed(self.config.max_requests);
        }
        
        // Skip if path doesn't match (when paths filter is set)
        if let Some(ref paths) = self.config.paths {
            if !paths.iter().any(|p| request.path.starts_with(p)) {
                return ThrottleResult::allowed(self.config.max_requests);
            }
        }
        
        let window = Duration::from_secs(self.config.window_secs);
        let now = Instant::now();
        
        let mut buckets = self.buckets.lock();
        let requests = buckets.entry(ip.to_string()).or_insert_with(Vec::new);
        
        // Remove old requests
        requests.retain(|t| now.duration_since(*t) < window);
        
        if requests.len() as u32 >= self.config.max_requests {
            let oldest = requests[0];
            let retry_after = window.saturating_sub(now.duration_since(oldest)).as_secs();
            
            return ThrottleResult {
                allowed: false,
                remaining: 0,
                limit: self.config.max_requests,
                retry_after: Some(retry_after),
            };
        }
        
        requests.push(now);
        
        ThrottleResult {
            allowed: true,
            remaining: self.config.max_requests - requests.len() as u32,
            limit: self.config.max_requests,
            retry_after: None,
        }
    }
    
    /// Create a 429 Too Many Requests response
    pub fn too_many_requests_response(&self, result: &ThrottleResult) -> Response {
        let mut response = Response::new(StatusCode::TOO_MANY_REQUESTS)
            .json(&serde_json::json!({
                "error": "Too Many Requests",
                "message": "Rate limit exceeded. Please try again later.",
                "retry_after": result.retry_after,
            }))
            .unwrap();
        
        response.headers.insert(
            "x-ratelimit-limit".to_string(),
            result.limit.to_string(),
        );
        response.headers.insert(
            "x-ratelimit-remaining".to_string(),
            result.remaining.to_string(),
        );
        if let Some(retry) = result.retry_after {
            response.headers.insert(
                "retry-after".to_string(),
                retry.to_string(),
            );
        }
        
        response
    }
    
    /// Add rate limit headers to a response
    pub fn add_headers(&self, result: &ThrottleResult, response: &mut Response) {
        response.headers.insert(
            "x-ratelimit-limit".to_string(),
            result.limit.to_string(),
        );
        response.headers.insert(
            "x-ratelimit-remaining".to_string(),
            result.remaining.to_string(),
        );
    }
    
    /// Clean up old entries
    pub fn cleanup(&self) {
        let window = Duration::from_secs(self.config.window_secs);
        let now = Instant::now();
        
        let mut buckets = self.buckets.lock();
        buckets.retain(|_, requests| {
            requests.retain(|t| now.duration_since(*t) < window);
            !requests.is_empty()
        });
    }
}

/// Result of a throttle check
#[derive(Debug, Clone)]
pub struct ThrottleResult {
    pub allowed: bool,
    pub remaining: u32,
    pub limit: u32,
    pub retry_after: Option<u64>,
}

impl ThrottleResult {
    fn allowed(limit: u32) -> Self {
        Self {
            allowed: true,
            remaining: limit,
            limit,
            retry_after: None,
        }
    }
}

/// Register a throttle (rate-limit) middleware under the given name in a
/// `Router`'s middleware stack. Requests exceeding the configured limit are
/// short-circuited with a 429 Too Many Requests response.
///
/// Usage:
/// ```ignore
/// let throttle = ThrottleMiddleware::new(ThrottleConfig {
///     max_requests: 100,
///     window_secs: 60,
///     ..Default::default()
/// });
/// noor::core::middleware::throttle::register(&mut router, throttle, "throttle");
/// router.use_middleware("throttle");
/// ```
pub fn register(
    router: &mut crate::core::router::Router,
    throttle: ThrottleMiddleware,
    name: &str,
) {
    let throttle = Arc::new(throttle);
    router.middleware_stack().register(
        name,
        Arc::new(move |request| {
            let result = throttle.check(&request);
            if result.allowed {
                Ok(crate::core::middleware::MiddlewareOutcome::Continue(request))
            } else {
                Ok(crate::core::middleware::MiddlewareOutcome::Stop(
                    throttle.too_many_requests_response(&result),
                ))
            }
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_throttle_allows_initial_requests() {
        let middleware = ThrottleMiddleware::new(ThrottleConfig {
            max_requests: 3,
            window_secs: 60,
            paths: None,
            excluded_ips: vec![],
        });
        
        let mut request = Request::new(crate::core::http::Method::Get, "/".to_string());
        request.client_ip = Some("192.168.1.1".to_string());
        
        assert!(middleware.check(&request).allowed);
        assert!(middleware.check(&request).allowed);
        assert!(middleware.check(&request).allowed);
        assert!(!middleware.check(&request).allowed);
    }
    
    #[test]
    fn test_throttle_excludes_localhost() {
        let middleware = ThrottleMiddleware::new(ThrottleConfig::default());
        
        let mut request = Request::new(crate::core::http::Method::Get, "/".to_string());
        request.client_ip = Some("127.0.0.1".to_string());
        
        // Should always be allowed for localhost
        for _ in 0..100 {
            assert!(middleware.check(&request).allowed);
        }
    }
}
