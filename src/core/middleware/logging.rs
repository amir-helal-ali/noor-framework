// ============================================================
// Logging Middleware - middleware التسجيل
// ============================================================

use std::sync::Arc;
use std::time::Instant;
use crate::core::http::{Request, Response};

/// Logging middleware for request/response logging
pub struct LoggingMiddleware {
    log_bodies: bool,
    max_body_log_size: usize,
}

impl LoggingMiddleware {
    pub fn new() -> Self {
        Self {
            log_bodies: false,
            max_body_log_size: 1000,
        }
    }
    
    pub fn with_body_logging(mut self, enabled: bool) -> Self {
        self.log_bodies = enabled;
        self
    }
    
    /// Log an incoming request
    pub fn log_request(&self, request: &Request) -> Instant {
        let start = Instant::now();
        
        let body_info = if self.log_bodies && !request.body.is_empty() {
            let body_str = String::from_utf8_lossy(&request.body);
            let truncated = if body_str.len() > self.max_body_log_size {
                format!("{}... (truncated)", &body_str[..self.max_body_log_size])
            } else {
                body_str.to_string()
            };
            format!(", body: {}", truncated)
        } else {
            String::new()
        };
        
        tracing::info!(
            method = %request.method,
            path = %request.path,
            ip = ?request.client_ip,
            "→ {} {}{}",
            request.method, request.path, body_info
        );
        
        start
    }
    
    /// Log a completed response
    pub fn log_response(&self, request: &Request, response: &Response, start: Instant) {
        let duration = start.elapsed();
        let duration_ms = duration.as_millis();
        
        let body_info = if self.log_bodies && !response.body.is_empty() {
            let body_str = String::from_utf8_lossy(&response.body);
            let truncated = if body_str.len() > self.max_body_log_size {
                format!("{}... (truncated)", &body_str[..self.max_body_log_size])
            } else {
                body_str.to_string()
            };
            format!(", body: {}", truncated)
        } else {
            String::new()
        };
        
        let level = if response.status.is_server_error() {
            "error"
        } else if response.status.is_client_error() {
            "warn"
        } else {
            "info"
        };
        
        if level == "error" {
            tracing::error!(
                status = response.status.0,
                duration_ms = duration_ms,
                "← {} {} ({}ms){}",
                request.method, request.path, duration_ms, body_info
            );
        } else if level == "warn" {
            tracing::warn!(
                status = response.status.0,
                duration_ms = duration_ms,
                "← {} {} ({}ms){}",
                request.method, request.path, duration_ms, body_info
            );
        } else {
            tracing::info!(
                status = response.status.0,
                duration_ms = duration_ms,
                "← {} {} ({}ms){}",
                request.method, request.path, duration_ms, body_info
            );
        }
    }
}

impl Default for LoggingMiddleware {
    fn default() -> Self {
        Self::new()
    }
}
